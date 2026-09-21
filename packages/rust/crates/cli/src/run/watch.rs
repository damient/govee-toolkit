//! `watch`: the SDK's event stream, one record per event. Nothing rescans on
//! its own: one scan starts the run, and `--rescan-ms` repeats it.

use std::time::Duration;

use govee_toolkit::codec::Mode;
use govee_toolkit::exit::{Failure, Writer};
use govee_toolkit::{Event, Govee};
use tokio::sync::broadcast::error::RecvError;

pub(super) async fn run(
    govee: &Govee,
    writer: Writer,
    rescan_ms: u64,
    restrict: Option<Mode>,
) -> Result<(), Failure> {
    // Subscribed before the scan, so that what the scan finds is reported.
    let mut events = govee.events();
    let modes = super::modes(govee, restrict);
    govee.scan_on(&modes).await?;
    if rescan_ms > 0 {
        rescan(govee.clone(), rescan_ms, modes);
    }

    loop {
        match events.recv().await {
            Ok(event) if skipped(&event, restrict) => {}
            Ok(event) => writer.emit(&event.to_json(), &event.to_string()),
            // The stream drops the oldest events, so a slow reader loses
            // events rather than blocking the SDK.
            Err(RecvError::Lagged(missed)) => writer.emit(
                &Event::lagged(missed),
                &format!("lagged: {missed} events were dropped"),
            ),
            Err(RecvError::Closed) => return Ok(()),
        }
    }
}

/// Scan again on a fixed interval. A failed scan is left to the next tick:
/// one unreachable mode must not end the watch.
fn rescan(govee: Govee, every_ms: u64, modes: Vec<Mode>) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_millis(every_ms));
        // The first tick completes at once, and the run already scanned.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            drop(govee.scan_on(&modes).await);
        }
    });
}

/// Whether `--mode` rules this event out. An event that names no mode is
/// about the device itself, and every run reports it.
fn skipped(event: &Event, restrict: Option<Mode>) -> bool {
    let (Some(only), Some(mode)) = (restrict, event.mode()) else {
        return false;
    };
    only != mode
}
