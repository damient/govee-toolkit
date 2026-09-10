//! Turns one parsed invocation into one run.

use govee_toolkit::codec::Mode;
use govee_toolkit::{Config, DeviceId, Govee};

use crate::cli::{Cli, Command};
use crate::output::{Failure, Writer};

mod devices;
mod verbs;

/// Run the subcommand the caller named.
pub(crate) async fn dispatch(cli: &Cli, writer: &Writer) -> Result<(), Failure> {
    let restrict = cli.global.mode.map(Mode::from);

    match &cli.command {
        Command::Scan { timeout_ms } => {
            let mut config = load(cli)?;
            config.lan.scan_window_ms = *timeout_ms;
            let govee = start(config).await?;
            devices::scan(&govee, writer, restrict).await
        }
        Command::Devices => {
            let govee = start(load(cli)?).await?;
            devices::list(&govee, writer, restrict);
            Ok(())
        }
        Command::Describe { .. } => Err(pending("describe")),
        Command::Send { .. } => Err(pending("send")),
        Command::Status { .. } => Err(pending("status")),
        Command::On { device } | Command::Off { device } => {
            let on = matches!(cli.command, Command::On { .. });
            verb(cli, writer, device, verbs::Verb::Power(on)).await
        }
        Command::Brightness { device, value } => {
            verb(cli, writer, device, verbs::Verb::Brightness(*value)).await
        }
        Command::Color { device, color } => {
            let rgb = verbs::rgb(color)?;
            verb(cli, writer, device, verbs::Verb::Color(rgb)).await
        }
        Command::Segment { .. } => Err(pending("segment")),
    }
}

/// Run one verb against one device.
async fn verb(cli: &Cli, writer: &Writer, device: &str, verb: verbs::Verb) -> Result<(), Failure> {
    let id = DeviceId::new(device);
    let mut config = load(cli)?;

    // `--mode` narrows what the configuration enables for this device, and
    // adds nothing. A mode the configuration leaves out is refused here, since
    // sending over another one would substitute a mode in silence.
    if let Some(mode) = cli.global.mode.map(Mode::from) {
        if !config.modes_for(&id).contains(&mode) {
            return Err(Failure::unsupported(format!(
                "`{id}` does not enable mode `{mode}`; the configuration decides which modes a device has"
            )));
        }
        config.devices.entry(id.clone()).or_default().modes = Some(vec![mode]);
    }

    verbs::run(&start(config).await?, writer, &id, verb).await
}

/// Read the configuration the run works from.
fn load(cli: &Cli) -> Result<Config, Failure> {
    let config = match &cli.global.config {
        Some(path) => Config::load_from(path.clone()),
        None => Config::load(),
    }?;
    Ok(config)
}

/// Bring the transports up.
async fn start(config: Config) -> Result<Govee, Failure> {
    Ok(Govee::start(config).await?)
}

fn pending(name: &str) -> Failure {
    Failure::unsupported(format!("`{name}` is declared but not implemented yet"))
}
