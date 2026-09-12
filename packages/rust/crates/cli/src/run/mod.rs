//! Turns one parsed invocation into one run.

use govee_toolkit::codec::Mode;
use govee_toolkit::{Config, DeviceId, Env, Govee, Music};

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
        Command::Watch { rescan_ms } => watch::run(govee, writer, *rescan_ms, restrict).await,
        Command::Stream {
            device,
            resolution,
            rate,
            gradient,
        } => {
            stream::run(
                govee,
                writer,
                &DeviceId::new(device),
                resolution,
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
        other => one_verb(govee, writer, other).await,
    }
}

/// One verb a person types, with the values read under their types.
///
/// Every arm names a device, so [`device_of`] has already discovered it.
async fn one_verb(govee: &Govee, writer: &Writer, command: &Command) -> Result<(), Failure> {
    let (device, played) = match command {
        Command::On { device } => (device, verbs::Verb::Power(true)),
        Command::Off { device } => (device, verbs::Verb::Power(false)),
        Command::Brightness { device, value } => (device, verbs::Verb::Brightness(*value)),
        Command::Color { device, color } => (device, verbs::Verb::Color(args::rgb(color)?)),
        Command::Colortemp { device, kelvin } => (device, verbs::Verb::ColorTemp(*kelvin)),
        Command::Gradient { device, state } => (device, verbs::Verb::Gradient((*state).into())),
        Command::Segment {
            device,
            zones,
            resolution,
            colors,
            gradient,
        } => (
            device,
            segment(zones.as_deref(), resolution, colors, *gradient).await?,
        ),
        Command::Music {
            device,
            effect,
            sensitivity,
            soft,
            color,
        } => (
            device,
            verbs::Verb::Music(music(*effect, *sensitivity, *soft, color.as_deref())?),
        ),
        other => return Err(Failure::internal(format!("{other:?} is not a verb"))),
    };
    verbs::run(govee, writer, &DeviceId::new(device), played).await
}

/// What a person typed for one painting, with the zones, the colors and the
/// resolution read under their types.
async fn segment(
    zones: Option<&str>,
    resolution: &str,
    colors: &str,
    gradient: bool,
) -> Result<verbs::Verb, Failure> {
    Ok(verbs::Verb::Segment {
        zones: zones.map(args::zones).transpose()?,
        colors: args::colors_or_stdin(colors).await?,
        resolution: args::resolution(resolution)?,
        gradient,
    })
}

/// What a person typed for one music effect, with the color read as a color.
fn music(effect: i64, sensitivity: i64, soft: bool, color: Option<&str>) -> Result<Music, Failure> {
    Ok(Music {
        effect,
        sensitivity,
        soft,
        color: color.map(args::rgb).transpose()?,
    })
}

/// Make the device reachable before the subcommand sends anything.
///
/// `ble` relates a device to a handle through an advertisement alone, and
/// `cloud` lists the account at startup, so neither keeps anything across
/// runs: a command in a fresh process must find the device first. A device no
/// enabled mode finds is reported here rather than by the command.
async fn discover(govee: &Govee, id: &DeviceId) -> Result<(), Failure> {
    govee.ensure_known(id).await?;
    Ok(())
}

/// The modes a run touches: the one `--mode` names, or every enabled mode.
///
/// `--mode` restricts the wire, and not only what is printed: a scan over
/// another mode would send frames the caller ruled out.
fn modes(govee: &Govee, restrict: Option<Mode>) -> Vec<Mode> {
    restrict.map_or_else(|| govee.modes(), |mode| vec![mode])
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
        | Command::Colortemp { device, .. }
        | Command::Segment { device, .. }
        | Command::Gradient { device, .. }
        | Command::Music { device, .. }
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
    let env = match (&cli.global.env_file, cli.global.no_env) {
        (Some(path), _) => Env::from_file(path)?,
        (None, true) => Env::process(),
        (None, false) => Env::load()?,
    };
    let path = match &cli.global.config {
        Some(path) => path.clone(),
        None => govee_toolkit::paths::config_file_from(&env),
    };
    Ok(Config::load_from_with(path, env)?)
}
