//! Every `ble` command of a device file, sent to a real device in order.
//!
//! It needs a Bluetooth adapter, and the device closed in the Govee app. The
//! radio accepts one connection at a time, and a connected device does not
//! advertise.
//!
//! ```bash
//! cargo run --example ble_tour --features ble
//! GOVEE_SKU=H61A0 cargo run --example ble_tour --features ble
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

/// Every zone the H61A0's mask reaches. A zone past this is refused, not
/// trimmed: the firmware would drop the bit in silence.
const ALL_ZONES: [u16; 15] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];

#[tokio::main]
async fn main() -> Result<(), Error> {
    let sku = std::env::var("GOVEE_SKU").unwrap_or_else(|_| "H61A0".to_owned());

    // `ble` alone, so no other mode can serve what runs below.
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

    // Power first: every command below paints a lit strip.
    device.send("power", &Args::new().int("on", 1)).await?;

    // `colors` is a list because the same argument role serves a mode that
    // carries every zone in one frame. This frame has room for one color.
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

    // A subset proves the device reads the mask: a full mask looks the same as
    // an ignored one.
    device
        .send(
            "color",
            &Args::new()
                .rgb("colors", [[0, 0, 255]])
                .zones("zones", [0, 1, 2]),
        )
        .await?;

    // The firmware does not render a color temperature. The caller sends the
    // kelvin value and its RGB rendering in the same frame; a frame with only
    // the kelvin value leaves the zones dark.
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

    // Brightness for the zones the mask names; the other zones keep theirs.
    device
        .send(
            "segment_brightness_masked",
            &Args::new().int("level", 10).zones("zones", [0, 1, 2, 3, 4]),
        )
        .await?;
    // Every zone at once, one byte each, with no mask.
    device
        .send(
            "segment_brightness",
            &Args::new().bytes(
                "levels",
                vec![100, 90, 80, 70, 60, 50, 40, 30, 20, 10, 10, 10, 10, 10, 10],
            ),
        )
        .await?;

    // This flag sets interpolation between zones, not a fade over time. Two
    // colors one after the other cut to each other either way.
    device.send("gradient", &Args::new().int("on", 1)).await?;

    read_everything(&device).await?;
    paint_zones(&device).await?;

    device.send("power", &Args::new().int("on", 0)).await?;
    Ok(())
}

/// What the device answers, through the `reply:` layouts its file declares.
///
/// No field name below lives in the SDK. The device file names the frame, the
/// bytes and the field.
async fn read_everything(device: &govee_toolkit::DeviceHandle<'_>) -> Result<(), Error> {
    // `status` is the entry marked `role: status`, which fire-and-verify sends
    // on its own. Over `ble` it takes two exchanges: no single frame reports
    // both power and brightness.
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
/// A mode that paints by mask costs one write per distinct color. A solid
/// fill takes one write; fifteen colors take fifteen.
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
