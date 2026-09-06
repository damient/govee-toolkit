//! Every `ble` command of a device file, sent to a real device in order.
//!
//! What it needs: a Bluetooth adapter, and the device off the Govee app — one
//! connection at a time, and a connected device stops advertising.
//!
//! ```bash
//! cargo run --example ble_tour --features ble
//! GOVEE_SKU=H61A0 cargo run --example ble_tour --features ble
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

/// Every zone the H61A0's mask reaches. A zone past this is refused, not
/// trimmed: the firmware would drop the bit in silence.
const ALL_ZONES: [u16; 15] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];

#[tokio::main]
async fn main() -> Result<(), Error> {
    let sku = std::env::var("GOVEE_SKU").unwrap_or_else(|_| "H61A0".to_owned());

    // `ble` alone, so nothing can be served by another mode: what runs below is
    // this mode or it is an error.
    let config = Config {
        defaults: govee_toolkit::config::Defaults {
            modes: vec![govee_toolkit::Mode::Ble],
        },
        ..Config::load()?
    };
    let govee = Govee::start(config).await?;

    println!("scanning...");
    let Some(found) = govee.scan().await?.into_iter().find(|d| d.sku == sku) else {
        println!("no {sku} is advertising");
        return Ok(());
    };
    println!("{} — {} — modes {:?}", found.id, found.sku, found.modes);
    let device = govee.device(&found.id);

    // Power first: everything below paints a rope that is lit.
    device.send("power", &Args::new().int("on", 1)).await?;

    // One colour over the zones the mask names. `colors` is a list because the
    // same argument role serves a mode that carries every zone in one frame;
    // here the frame has room for one.
    device
        .send(
            "color",
            &Args::new()
                .rgb("colors", [[255, 0, 0]])
                .zones("zones", ALL_ZONES),
        )
        .await?;
    device
        .send("brightness", &Args::new().int("level", 60))
        .await?;

    // Painting a subset is what proves the mask is read at all: a saturated
    // mask looks exactly like an ignored one.
    device
        .send(
            "color",
            &Args::new()
                .rgb("colors", [[0, 0, 255]])
                .zones("zones", [0, 1, 2]),
        )
        .await?;

    // The firmware does not render a colour temperature: the caller ships the
    // kelvin value and its RGB rendering in the same frame. A frame carrying
    // only the kelvin value leaves the zones dark.
    device
        .send(
            "colortemp",
            &Args::new()
                .int("kelvin", 2700)
                .int("white_r", 255)
                .int("white_g", 169)
                .int("white_b", 87)
                .zones("zones", ALL_ZONES),
        )
        .await?;

    // Brightness of the zones the mask names, leaving the rest alone…
    device
        .send(
            "segment_brightness_masked",
            &Args::new().int("level", 10).zones("zones", [0, 1, 2, 3, 4]),
        )
        .await?;
    // …and every zone at once, one byte each, with no mask to leave any out.
    device
        .send(
            "segment_brightness",
            &Args::new().bytes(
                "levels",
                vec![100, 90, 80, 70, 60, 50, 40, 30, 20, 10, 10, 10, 10, 10, 10],
            ),
        )
        .await?;

    // Whether the firmware interpolates between zones. Not a fade over time:
    // two colours sent one after the other cut to each other either way.
    device.send("gradient", &Args::new().int("on", 1)).await?;

    read_everything(&device).await?;
    paint_zones(&device).await?;

    device.send("power", &Args::new().int("on", 0)).await?;
    Ok(())
}

/// What the device answers, through the `reply:` layouts its file declares.
///
/// No field name below lives in the SDK: the device file says which frame asks
/// for a value, which bytes carry it and under what name.
async fn read_everything(device: &govee_toolkit::DeviceHandle<'_>) -> Result<(), Error> {
    // `status` is the entry marked `role: status`, which fire-and-verify sends
    // on its own. Over `ble` it is two exchanges: no single frame reports both
    // power and brightness.
    let status = device.status().await?;
    println!("on {:?}, brightness {:?}", status.on, status.brightness);

    for command in [
        "read_segment_count",
        "read_ic_count",
        "read_wifi_mac",
        "read_hardware_version",
        "read_software_version",
        "read_dynamic_api",
    ] {
        let reply = device.read(command, &Args::new()).await?;
        println!("{command} -> {}", reply.fields.to_json());
    }
    Ok(())
}

/// The segment channel, armed once and fed frames on a clock.
///
/// Over a mode that paints by mask a repaint costs one write per distinct
/// colour, so a solid fill is one write and a picture of fifteen colours is
/// fifteen.
async fn paint_zones(device: &govee_toolkit::DeviceHandle<'_>) -> Result<(), Error> {
    let stream = device
        .open_stream(StreamOptions {
            // `Zones::Native` is refused here: a mask names zones and reaches
            // no pixel behind them.
            zones: Zones::Exact(15),
            rate: Rate::Measured,
            gradient: false,
        })
        .await?;

    stream.fill([0, 0, 40])?;
    for zone in 0..stream.zones() {
        stream.set_zone(zone, [255, 140, 0])?;
        tokio::time::sleep(Duration::from_millis(120)).await;
    }
    println!(
        "{} frames sent, {} superseded",
        stream.frames_sent(),
        stream.frames_superseded()
    );
    stream.close().await?;
    Ok(())
}
