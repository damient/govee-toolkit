//! Every `lan` command of a device file, sent to a real device in order.
//!
//! It needs the device on the same network, with LAN Control enabled in the
//! Govee Home app.
//!
//! ```bash
//! cargo run --example lan_tour
//! GOVEE_SKU=H61A0 cargo run --example lan_tour
//! ```
//!
//! It walks the H61A0's table. Another SKU names its own commands and
//! arguments: nothing here knows a command name that `devices/*.yaml` did not
//! give it.

// The no-print lint is the library's rule. An example reports to the person who
// runs it.
#![allow(clippy::print_stdout)]

use std::time::Duration;

use govee_toolkit::stream::{Rate, StreamOptions, Zones};
use govee_toolkit::{Args, Config, Error, Govee};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let sku = std::env::var("GOVEE_SKU").unwrap_or_else(|_| "H61A0".to_owned());

    // `lan` is the default. This example names it so that the user's
    // configuration cannot change what runs below.
    let config = Config {
        defaults: govee_toolkit::config::Defaults {
            modes: vec![govee_toolkit::Mode::Lan],
        },
        ..Config::load()?
    };
    let govee = Govee::start(config).await?;

    println!("scanning...");
    let Some(found) = govee.scan().await?.into_iter().find(|d| d.sku == sku) else {
        println!("no {sku} answered discovery");
        return Ok(());
    };
    println!("{} — {} — modes {:?}", found.id, found.sku, found.modes);
    let device = govee.device(&found.id);

    device.send("power", &Args::new().int("on", 1)).await?;
    device
        .send("brightness", &Args::new().int("level", 60))
        .await?;

    // This mode's color command carries no zones, so the whole strip takes
    // the color.
    device
        .send("color", &Args::new().int("r", 255).int("g", 40).int("b", 0))
        .await?;

    // Kelvin alone, unlike `ble`: this mode's frame asks the firmware for a
    // temperature rather than for its RGB rendering.
    device
        .send("colortemp", &Args::new().int("kelvin", 4000))
        .await?;

    // `status` is the entry marked `role: status`; `raw` keeps the whole
    // `msg.data`, so undocumented fields stay reachable.
    let status = device.status().await?;
    println!("on {:?}, brightness {:?}", status.on, status.brightness);
    println!("raw {}", status.raw);

    // An undocumented read the SDK models nothing of. `DeviceHandle::read`
    // serves it on a mode that answers frames. Over `lan` the answer is JSON,
    // so the status above reads it back instead.
    device.send("raw_status", &Args::new()).await?;

    paint_zones(&device).await?;

    device.send("power", &Args::new().int("on", 0)).await?;
    Ok(())
}

/// The raw segment channel: armed once, then fed frames on a clock.
///
/// This mode carries every zone in one frame, so a repaint costs one write
/// whatever the picture holds. It reaches the strip's native resolution, past
/// the zones the Govee app exposes.
async fn paint_zones(device: &govee_toolkit::DeviceHandle<'_>) -> Result<(), Error> {
    let stream = device
        .open_stream(StreamOptions {
            // Every addressable LED. It fails where nobody measured that count
            // on the unit: the count belongs to the strip, not to the SKU.
            zones: Zones::Native,
            rate: Rate::Measured,
            gradient: false,
        })
        .await?;

    stream.fill([0, 0, 40])?;
    for zone in 0..stream.zones() {
        stream.set_zone(zone, [255, 140, 0])?;
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
    println!(
        "{} frames sent, {} superseded",
        stream.frames_sent(),
        stream.frames_superseded()
    );
    stream.close().await?;
    Ok(())
}
