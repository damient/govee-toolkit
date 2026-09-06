//! Every `lan` command of a device file, sent to a real device in order.
//!
//! What it needs: the device on the same network, with LAN Control enabled in
//! the Govee Home app.
//!
//! ```bash
//! cargo run --example lan_tour
//! GOVEE_SKU=H61A0 cargo run --example lan_tour
//! ```
//!
//! It walks the H61A0's table. Another SKU names its commands and arguments in
//! its own device file, and the calls below are then that file's, not this
//! example's — nothing here knows a command name that `devices/*.yaml` did not
//! give it.

// An example is a program: it reports to the person running it. The no-print
// rule is the library's.
#![allow(clippy::print_stdout)]

use std::time::Duration;

use govee_toolkit::stream::{Rate, StreamOptions, Zones};
use govee_toolkit::{Args, Config, Error, Govee};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let sku = std::env::var("GOVEE_SKU").unwrap_or_else(|_| "H61A0".to_owned());

    // `lan` is the default, spelled out here so this example is unaffected by
    // whatever the user's configuration enables.
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

    // Colour is per channel here, and the whole strip wears it: this mode's
    // colour command carries no zones.
    device
        .send("color", &Args::new().int("r", 255).int("g", 40).int("b", 0))
        .await?;

    // Kelvin alone, unlike `ble`: this mode's frame asks the firmware for a
    // temperature rather than shipping its rendering.
    device
        .send("colortemp", &Args::new().int("kelvin", 4000))
        .await?;

    // `status` is the entry marked `role: status`; `raw` keeps the whole
    // `msg.data`, so undocumented fields stay reachable.
    let status = device.status().await?;
    println!("on {:?}, brightness {:?}", status.on, status.brightness);
    println!("raw {}", status.raw);

    // An undocumented read the SDK models nothing of. Its reply reaches the
    // caller through `DeviceHandle::read` on a mode that answers frames; over
    // `lan` the answer is JSON, so `raw_status` is sent and read back through
    // the status above rather than through `read`.
    device.send("raw_status", &Args::new()).await?;

    paint_zones(&device).await?;

    device.send("power", &Args::new().int("on", 0)).await?;
    Ok(())
}

/// The raw segment channel: armed once, then fed frames on a clock.
///
/// This mode carries every zone in one frame, so a repaint costs one write
/// whatever the picture holds — and it reaches the strip's native resolution,
/// past the zones the Govee app exposes.
async fn paint_zones(device: &govee_toolkit::DeviceHandle<'_>) -> Result<(), Error> {
    let stream = device
        .open_stream(StreamOptions {
            // Every addressable LED. It fails where nobody measured that count
            // on the unit: it belongs to the physical strip, not to the SKU.
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
