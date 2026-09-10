//! Every `cloud` command of a device file, sent to a real device in order.
//!
//! It needs a Govee account, an API key and internet. The device does not have
//! to be on the same network.
//!
//! ```bash
//! GOVEE_API_KEY=… cargo run --example cloud_tour --features cloud
//! GOVEE_API_KEY=… GOVEE_SKU=H61A0 cargo run --example cloud_tour --features cloud
//! ```
//!
//! It spends one request per command, and the transport holds each device to
//! one request per interval, so the tour takes about a minute. It walks the
//! H61A0's table; another SKU names its own commands and arguments.

// The no-print lint is the library's rule. An example reports to the person who
// runs it.
#![allow(clippy::print_stdout)]

use govee_toolkit::{Args, Config, Error, Govee};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let sku = std::env::var("GOVEE_SKU").unwrap_or_else(|_| "H61A0".to_owned());

    // This example names the mode, so that the user's configuration cannot
    // change what runs below.
    let config = Config {
        defaults: govee_toolkit::config::Defaults {
            modes: vec![govee_toolkit::Mode::Cloud],
        },
        ..Config::load()?
    };
    let govee = Govee::start(config).await?;

    println!("listing the account...");
    let Some(found) = govee.scan().await?.into_iter().find(|d| d.sku == sku) else {
        println!("the account owns no {sku}");
        return Ok(());
    };
    println!("{} — {} — modes {:?}", found.id, found.sku, found.modes);
    let device = govee.device(&found.id);

    device.send("power", &Args::new().int("on", 1)).await?;
    device
        .send("brightness", &Args::new().int("level", 60))
        .await?;

    // The same three arguments as the other modes. This API carries them as
    // one integer, and the device file packs them.
    device
        .send("color", &Args::new().int("r", 255).int("g", 40).int("b", 0))
        .await?;
    device
        .send("colortemp", &Args::new().int("kelvin", 4000))
        .await?;

    // The segment entries take an array of zones. This mode paints the zones
    // that array names and leaves the rest alone.
    device
        .send(
            "segment_color",
            &Args::new()
                .zones("zones", vec![0, 1])
                .int("r", 0)
                .int("g", 255)
                .int("b", 0),
        )
        .await?;
    device
        .send(
            "segment_brightness",
            &Args::new().zones("zones", vec![0, 1]).int("level", 10),
        )
        .await?;

    // Music runs off the device's own microphone. The effect identifiers are
    // the API's own, and the device file names them.
    device
        .send(
            "music",
            &Args::new().int("effect", 1).int("sensitivity", 50),
        )
        .await?;

    // `status` is the entry marked `role: status`. `raw` keeps every
    // capability the account reported, including the ones the SDK models
    // nothing of.
    let status = device.status().await?;
    println!("on {:?}, brightness {:?}", status.on, status.brightness);
    println!("raw {}", status.raw);

    device.send("power", &Args::new().int("on", 0)).await?;
    Ok(())
}
