//! `stream`: the raw segment channel, fed one frame per line of stdin.
//!
//! A line is one color, which fills every zone, or one color per zone. Writes
//! never block: a frame that arrives before the previous one went out replaces
//! it, and the count of replaced frames is reported at the end.

use std::time::Duration;

use govee_toolkit::stream::{Rate, StreamOptions};
use govee_toolkit::{DeviceId, Govee};
use serde_json::json;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::output::{Failure, Writer};
use crate::run::args;

/// What is waited past one frame interval, so that a tick due at the end of it
/// is not missed.
const MARGIN: Duration = Duration::from_millis(20);

/// Open the channel, send what stdin carries, then close it.
pub(super) async fn run(
    govee: &Govee,
    writer: &Writer,
    id: &DeviceId,
    resolution: &str,
    rate: Option<f64>,
    gradient: bool,
) -> Result<(), Failure> {
    let options = StreamOptions {
        resolution: args::resolution(resolution)?,
        rate: rate.map_or(Rate::Measured, Rate::Fixed),
        gradient,
    };
    let stream = govee.device(id).open_stream(options).await?;

    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    let mut wrote = false;
    loop {
        let line = lines
            .next_line()
            .await
            .map_err(|e| Failure::internal(e.to_string()))?;
        let Some(line) = line else { break };
        let colors = args::colors(&line)?;
        match colors.as_slice() {
            [] => continue,
            [color] => stream.fill(*color)?,
            every => stream.set_all(every)?,
        }
        wrote = true;
    }

    // The emitting task sends on a fixed interval, so the last write needs one
    // interval to reach the device. Input that ends sooner would otherwise
    // disarm the channel before anything went out.
    if wrote {
        let period = Duration::from_secs_f64(1.0 / stream.rate_hz());
        tokio::time::sleep(period + MARGIN).await;
    }

    let sent = stream.frames_sent();
    let superseded = stream.frames_superseded();
    let carried = stream.zones();
    stream.close().await?;

    writer.emit(
        &json!({
            "id": id.to_string(),
            "zones": carried,
            "frames_sent": sent,
            "frames_superseded": superseded,
        }),
        &format!("{id}  {carried} zones  {sent} frames sent  {superseded} superseded"),
    );
    Ok(())
}
