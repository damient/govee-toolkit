//! Every `cloud` command of a device file, sent to a real device in order.
//!
//! It needs a Govee account, an API key and internet. One request per
//! interval per device, so the tour takes about a minute.
//!
//! The key comes from `.env` at the repository root, or from `GOVEE_API_KEY`.
//!
//! ```bash
//! cargo run --example cloud_tour --features cloud
//! GOVEE_TEST_SKU=H61A0 cargo run --example cloud_tour --features cloud
//! GOVEE_TEST_DEVICE=AA:BB:CC:DD:EE:FF cargo run --example cloud_tour --features cloud
//! ```
//!
//! `GOVEE_TEST_DEVICE` names one unit where two of the SKU answer. `.env`
//! carries both variables.

// The no-print lint is the library's rule; an example reports to its runner.
#![allow(clippy::print_stdout)]

use govee_toolkit::{Args, Config, Device, DeviceId, Error, Govee};

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Naming the mode keeps the user's configuration from changing this run.
    let config = Config {
        defaults: govee_toolkit::config::Defaults {
            modes: vec![govee_toolkit::Mode::Cloud],
        },
        ..Config::load()?
    };
    let sku = config
        .env
        .var("GOVEE_TEST_SKU")
        .unwrap_or("H61A0")
        .to_owned();
    let wanted = config.env.var("GOVEE_TEST_DEVICE").map(DeviceId::new);
    let govee = Govee::start(config).await?;

    println!("listing the account...");
    let matches = |d: &Device| wanted.as_ref().map_or(d.sku == sku, |id| &d.id == id);
    let Some(found) = govee.scan().await?.into_iter().find(matches) else {
        let target = wanted.map_or(sku, |id| id.to_string());
        println!("the account owns no {target}");
        return Ok(());
    };
    println!("{} — {} — modes {:?}", found.id, found.sku, found.modes);
    let device = govee.device(&found.id);

    device.send("power", &Args::new().int("on", 1)).await?;
    device
        .send("brightness", &Args::new().int("level", 60))
        .await?;

    // The same three arguments as the other modes; the device file packs them.
    device
        .send("color", &Args::new().int("r", 255).int("g", 40).int("b", 0))
        .await?;
    device
        .send("colortemp", &Args::new().int("kelvin", 4000))
        .await?;

    // The segment entries leave the zones the array does not name alone.
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

    // Music runs off the device's own microphone.
    device
        .send(
            "music",
            &Args::new().int("effect", 1).int("sensitivity", 50),
        )
        .await?;

    // `raw` keeps the capabilities the SDK models nothing of too.
    let status = device.status().await?;
    println!("on {:?}, brightness {:?}", status.on, status.brightness);
    println!("raw {}", status.raw);

    // Only another per-zone write clears a per-zone brightness, so the tour
    // puts back what it dimmed. Every other setting above is a whole-device
    // one that the next command replaces.
    device
        .send(
            "segment_brightness",
            &Args::new().zones("zones", vec![0, 1]).int("level", 100),
        )
        .await?;

    device.send("power", &Args::new().int("on", 0)).await?;
    Ok(())
}
