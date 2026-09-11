//! Turns one parsed invocation into one run.

use govee_toolkit::codec::Mode;
use govee_toolkit::{Config, DeviceId, Govee};

use crate::cli::{Cli, Command};
use crate::output::{Failure, Writer};

mod args;
mod describe;
mod devices;
#[cfg(feature = "ble")]
mod provision;
mod send;
mod status;
mod stream;
mod verbs;
mod watch;

/// Run the subcommand the caller named.
pub(crate) async fn dispatch(cli: &Cli, writer: &Writer) -> Result<(), Failure> {
    let govee = Govee::start(configure(cli)?).await?;
    let outcome = route(&govee, cli, writer).await;
    // `ble` loses the last frame it wrote when nothing releases the adapter.
    let released = govee.shutdown().await.map_err(Failure::from);
    outcome.and(released)
}

async fn route(govee: &Govee, cli: &Cli, writer: &Writer) -> Result<(), Failure> {
    let restrict = cli.global.mode.map(Mode::from);
    if let Some(device) = device_of(&cli.command) {
        discover(govee, &DeviceId::new(device)).await?;
    }

    match &cli.command {
        Command::Scan { .. } => devices::scan(govee, writer, restrict).await,
        Command::Devices => {
            devices::list(govee, writer, restrict);
            Ok(())
        }
        Command::Doctor => {
            devices::doctor(govee, writer);
            Ok(())
        }
        Command::Describe { target } => describe::run(govee, writer, target),
        Command::Send {
            device,
            command,
            args,
        } => send::run(govee, writer, &DeviceId::new(device), command, args).await,
        Command::Status { device } => status::run(govee, writer, &DeviceId::new(device)).await,
        Command::On { device } | Command::Off { device } => {
            let on = matches!(cli.command, Command::On { .. });
            verb(govee, writer, device, verbs::Verb::Power(on)).await
        }
        Command::Brightness { device, value } => {
            verb(govee, writer, device, verbs::Verb::Brightness(*value)).await
        }
        Command::Color { device, color } => {
            let rgb = args::rgb(color)?;
            verb(govee, writer, device, verbs::Verb::Color(rgb)).await
        }
        Command::Segment {
            device,
            zones,
            color,
            gradient,
        } => {
            let verb_of = verbs::Verb::Segment {
                zones: zones.as_deref().map(list).transpose()?,
                rgb: args::rgb(color)?,
                gradient: *gradient,
            };
            verb(govee, writer, device, verb_of).await
        }
        Command::Watch { rescan_ms } => watch::run(govee, writer, *rescan_ms, restrict).await,
        Command::Stream {
            device,
            zones,
            rate,
            gradient,
        } => {
            stream::run(
                govee,
                writer,
                &DeviceId::new(device),
                zones,
                *rate,
                *gradient,
            )
            .await
        }
        #[cfg(feature = "ble")]
        Command::Provision {
            device,
            ssid,
            password,
            open,
            utc_offset_hours,
            utc_offset_minutes,
        } => {
            let secret = provision::Secret {
                password: password.as_deref(),
                open: *open,
            };
            provision::run(
                govee,
                writer,
                &DeviceId::new(device),
                ssid.as_deref(),
                &secret,
                (*utc_offset_hours, *utc_offset_minutes),
            )
            .await
        }
    }
}

/// Run one verb against one device.
async fn verb(
    govee: &Govee,
    writer: &Writer,
    device: &str,
    verb: verbs::Verb,
) -> Result<(), Failure> {
    verbs::run(govee, writer, &DeviceId::new(device), verb).await
}

/// Find the device where no transport of an enabled mode knows it yet.
///
/// `ble` relates a device to a handle through an advertisement alone, and
/// `cloud` lists the account at startup, so neither keeps anything across
/// runs: a command in a fresh process must discover the device first. The test
/// is per mode, not per device. A device the `lan` cache answers for is still
/// unknown to `cloud`, and a scan skipped on the strength of that cache would
/// fail the command with `UnknownDevice`.
async fn discover(govee: &Govee, id: &DeviceId) -> Result<(), Failure> {
    let known = govee.devices().iter().any(|device| {
        device.id == *id
            && device
                .modes
                .iter()
                .any(|mode| device.health.contains_key(mode))
    });
    if known {
        return Ok(());
    }
    // Only the modes this device enables: a scan on another one costs a
    // window and answers a question nobody asked.
    let modes = govee.config().modes_for(id).to_vec();
    govee.scan_on(&modes).await?;
    Ok(())
}

/// Read zone indices, zero-based and comma-separated.
fn list(text: &str) -> Result<Vec<u16>, Failure> {
    args::list(text)
        .map(|index| {
            index.parse::<u16>().map_err(|_| {
                Failure::usage(format!("`{index}` is not a zone index; they start at zero"))
            })
        })
        .collect()
}

/// The configuration the run works from, narrowed by `--mode`.
fn configure(cli: &Cli) -> Result<Config, Failure> {
    let mut config = load(cli)?;
    if let Command::Scan { timeout_ms } = cli.command {
        config.lan.scan_window_ms = timeout_ms;
    }

    // `--mode` narrows what the configuration enables for this device, and
    // adds nothing. A mode the configuration leaves out is refused here, since
    // sending over another one would substitute a mode in silence.
    let (Some(mode), Some(device)) = (cli.global.mode.map(Mode::from), device_of(&cli.command))
    else {
        return Ok(config);
    };
    let id = DeviceId::new(device);
    if !config.modes_for(&id).contains(&mode) {
        return Err(Failure::unsupported(format!(
            "`{id}` does not enable mode `{mode}`; the configuration decides which modes a device has"
        )));
    }
    config.devices.entry(id).or_default().modes = Some(vec![mode]);
    Ok(config)
}

/// The device one subcommand acts on, where it names one.
fn device_of(command: &Command) -> Option<&str> {
    match command {
        Command::Send { device, .. }
        | Command::Status { device }
        | Command::On { device }
        | Command::Off { device }
        | Command::Brightness { device, .. }
        | Command::Color { device, .. }
        | Command::Segment { device, .. }
        | Command::Stream { device, .. } => Some(device),
        #[cfg(feature = "ble")]
        Command::Provision { device, .. } => Some(device),
        Command::Scan { .. }
        | Command::Devices
        | Command::Doctor
        | Command::Describe { .. }
        | Command::Watch { .. } => None,
    }
}

/// Read the configuration the run works from.
fn load(cli: &Cli) -> Result<Config, Failure> {
    let config = match &cli.global.config {
        Some(path) => Config::load_from(path.clone()),
        None => Config::load(),
    }?;
    Ok(config)
}
