//! `watch`: the SDK's event stream, one record per event.
//!
//! Nothing rescans on its own, so a discovery reaches the stream only when a
//! scan runs. One scan starts the run, and `--rescan-ms` repeats it.

use std::time::Duration;

use govee_toolkit::transport::Event as TransportEvent;
use govee_toolkit::{Event, Govee};
use serde_json::{Value, json};
use tokio::sync::broadcast::error::RecvError;

use crate::output::{Failure, Writer};

/// Print events until the stream closes or the process is interrupted.
pub(super) async fn run(govee: &Govee, writer: &Writer, rescan_ms: u64) -> Result<(), Failure> {
    // Subscribed before the scan, so that what the scan finds is reported.
    let mut events = govee.events();
    govee.scan().await?;
    if rescan_ms > 0 {
        rescan(govee.clone(), rescan_ms);
    }

    loop {
        match events.recv().await {
            Ok(event) => writer.emit(&as_json(&event), &as_text(&event)),
            // The stream keeps the most recent events and drops the rest, so
            // a slow reader loses events rather than blocking the SDK.
            Err(RecvError::Lagged(missed)) => writer.emit(
                &json!({ "event": "lagged", "missed": missed }),
                &format!("lagged: {missed} events were dropped"),
            ),
            Err(RecvError::Closed) => return Ok(()),
        }
    }
}

/// Scan again on a fixed interval. A failed scan is left to the next tick:
/// one unreachable mode must not end the watch.
fn rescan(govee: Govee, every_ms: u64) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_millis(every_ms));
        // The first tick completes at once, and the run already scanned.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            drop(govee.scan().await);
        }
    });
}

fn as_json(event: &Event) -> Value {
    match event {
        Event::UnknownSku { id, sku } => json!({
            "event": "unknown_sku",
            "id": id.to_string(),
            "sku": sku,
        }),
        Event::Transport(TransportEvent::Discovered {
            mode,
            device,
            change,
        }) => json!({
            "event": "discovered",
            "mode": mode.to_string(),
            "id": device.id.to_string(),
            "sku": device.sku,
            "endpoint": device.endpoint,
            "firmware": device.firmware,
            "change": change_name(*change),
        }),
        Event::Transport(TransportEvent::Forgotten { mode, id }) => json!({
            "event": "forgotten",
            "mode": mode.to_string(),
            "id": id.to_string(),
        }),
        Event::Transport(TransportEvent::Sent(sent)) => json!({
            "event": "sent",
            "mode": sent.mode.to_string(),
            "id": sent.id.to_string(),
            "cmd": sent.cmd,
            "endpoint": sent.endpoint,
        }),
        Event::Transport(TransportEvent::Status { mode, status }) => json!({
            "event": "status",
            "mode": mode.to_string(),
            "id": status.id.to_string(),
            "on": status.on,
            "brightness": status.brightness,
        }),
        Event::Transport(TransportEvent::HealthChanged {
            id,
            mode,
            transition,
        }) => json!({
            "event": "health_changed",
            "mode": mode.to_string(),
            "id": id.to_string(),
            "from": transition.from.to_string(),
            "to": transition.to.to_string(),
        }),
        _ => json!({ "event": "unknown" }),
    }
}

fn as_text(event: &Event) -> String {
    match event {
        Event::UnknownSku { id, sku } => {
            format!("{id}  unknown sku  {sku}")
        }
        Event::Transport(TransportEvent::Discovered {
            mode,
            device,
            change,
        }) => format!(
            "{}  {mode}  discovered  {}  {}  {}",
            device.id,
            change_name(*change),
            device.sku,
            device.endpoint
        ),
        Event::Transport(TransportEvent::Forgotten { mode, id }) => {
            format!("{id}  {mode}  forgotten")
        }
        Event::Transport(TransportEvent::Sent(sent)) => format!(
            "{}  {}  sent  {}  {}",
            sent.id, sent.mode, sent.cmd, sent.endpoint
        ),
        Event::Transport(TransportEvent::Status { mode, status }) => format!(
            "{}  {mode}  status  on={}  brightness={}",
            status.id,
            option(status.on.map(|on| on.to_string())),
            option(status.brightness.map(|value| value.to_string())),
        ),
        Event::Transport(TransportEvent::HealthChanged {
            id,
            mode,
            transition,
        }) => format!(
            "{id}  {mode}  health  {} -> {}",
            transition.from, transition.to
        ),
        _ => "unknown event".to_owned(),
    }
}

fn change_name(change: govee_toolkit::transport::Change) -> &'static str {
    use govee_toolkit::transport::Change;
    match change {
        Change::New => "new",
        Change::Refreshed => "refreshed",
        Change::Moved => "moved",
        Change::FirmwareChanged => "firmware_changed",
    }
}

fn option(value: Option<String>) -> String {
    value.unwrap_or_else(|| "?".to_owned())
}
