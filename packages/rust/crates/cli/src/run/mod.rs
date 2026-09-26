//! Turns one parsed invocation into one run.

use std::time::Duration;

use govee_toolkit::codec::Mode;
use govee_toolkit::exit::{Failure, Writer};
use govee_toolkit::{Config, DeviceId, Env, Govee, Identify, Music, Selector, Walk};

use crate::cli::{self, Cli, Command};

mod args;
mod describe;
mod devices;
mod identify;
#[cfg(feature = "ble")]
mod provision;
mod send;
mod status;
mod stream;
mod verbs;
mod watch;

pub(crate) async fn dispatch(cli: &Cli, writer: Writer) -> Result<(), Failure> {
    let config = load(cli)?;
    let target = cli
        .command
        .device()
        .map(|target| Selector::one(target, &config))
        .transpose()?;
    let members = cli
        .command
        .members()
        .map(|target| Selector::many(target, &config))
        .transpose()?
        .unwrap_or_default();
    let govee = Govee::start(configure(cli, config, target.as_ref())?).await?;
    let outcome = route(&govee, cli, writer, target.as_ref(), &members).await;
    // `ble` loses the last frame it wrote when nothing releases the adapter.
    let released = govee.shutdown().await.map_err(Failure::from);
    outcome.and(released)
}

async fn route(
    govee: &Govee,
    cli: &Cli,
    writer: Writer,
    target: Option<&DeviceId>,
    members: &[DeviceId],
) -> Result<(), Failure> {
    let restrict = cli.global.mode;
    if let Some(id) = target {
        discover(govee, id).await?;
    }
    // `Command::device` is `Some` for exactly the commands that read this.
    let one = || target.ok_or_else(|| Failure::internal("no device target was resolved"));

    match &cli.command {
        Command::Scan { .. } => devices::scan(govee, writer, restrict).await,
        Command::Devices { targets } => devices::list(govee, writer, targets, restrict),
        Command::Doctor => {
            devices::doctor(govee, writer);
            Ok(())
        }
        Command::Describe { target } => describe::run(govee, writer, target),
        Command::Identify {
            targets,
            color,
            wait_ms,
            hold_ms,
            keep,
        } => {
            let walk = walk(color, *wait_ms, *hold_ms, *keep, restrict)?;
            identify::run(govee, writer, targets, &walk).await
        }
        Command::Send { command, args, .. } => {
            send::run(govee, writer, one()?, command, args).await
        }
        Command::Status { .. } => status::run(govee, writer, one()?).await,
        Command::Watch { rescan_ms } => watch::run(govee, writer, *rescan_ms, restrict).await,
        Command::Stream {
            resolution,
            rate,
            gradient,
            ..
        } => stream::run(govee, writer, one()?, resolution, *rate, *gradient).await,
        #[cfg(feature = "ble")]
        Command::Provision {
            ssid,
            password,
            open,
            utc_offset_hours,
            utc_offset_minutes,
            ..
        } => {
            let secret = provision::Secret {
                password: password.as_deref(),
                open: *open,
            };
            provision::run(
                govee,
                writer,
                one()?,
                ssid.as_deref(),
                &secret,
                (*utc_offset_hours, *utc_offset_minutes),
            )
            .await
        }
        Command::Verb(verb) => play(govee, writer, members, restrict, verb).await,
    }
}

async fn play(
    govee: &Govee,
    writer: Writer,
    members: &[DeviceId],
    restrict: Option<Mode>,
    verb: &cli::Verb,
) -> Result<(), Failure> {
    let played = match verb {
        cli::Verb::On { .. } => verbs::Verb::Power(true),
        cli::Verb::Off { .. } => verbs::Verb::Power(false),
        cli::Verb::Brightness { value, .. } => verbs::Verb::Brightness(*value),
        cli::Verb::Color { color, .. } => verbs::Verb::Color(args::rgb(color)?),
        cli::Verb::Colortemp { kelvin, .. } => verbs::Verb::ColorTemp(*kelvin),
        cli::Verb::Gradient { state, .. } => verbs::Verb::Gradient((*state).into()),
        cli::Verb::Segment {
            zones,
            resolution,
            colors,
            gradient,
            ..
        } => segment(zones.as_deref(), resolution, colors, *gradient).await?,
        cli::Verb::Music {
            effect,
            sensitivity,
            soft,
            color,
            ..
        } => verbs::Verb::Music(music(*effect, *sensitivity, *soft, color.as_deref())?),
    };
    verbs::run(govee, writer, members, restrict, played).await
}

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

fn music(effect: i64, sensitivity: i64, soft: bool, color: Option<&str>) -> Result<Music, Failure> {
    Ok(Music {
        effect,
        sensitivity,
        soft,
        color: color.map(args::rgb).transpose()?,
    })
}

/// Make the device reachable before the subcommand sends anything. No mode
/// keeps a record across runs, so a fresh process must find the device first.
async fn discover(govee: &Govee, id: &DeviceId) -> Result<(), Failure> {
    govee.ensure_known(id).await?;
    Ok(())
}

/// The modes a run touches. `--mode` restricts the wire and not only what is
/// printed: a scan over another mode would send frames the caller ruled out.
fn modes(govee: &Govee, restrict: Option<Mode>) -> Vec<Mode> {
    restrict.map_or_else(|| govee.modes(), |mode| vec![mode])
}

fn configure(cli: &Cli, mut config: Config, target: Option<&DeviceId>) -> Result<Config, Failure> {
    if let Command::Scan { timeout_ms } = cli.command {
        config.lan.scan_window_ms = timeout_ms;
    }

    // `--mode` narrows what the configuration enables and adds nothing:
    // sending over another mode would substitute one in silence.
    let Some(mode) = cli.global.mode else {
        return Ok(config);
    };
    // `verbs::run` pins the members of a verb, so a member that does not
    // enable the mode fails alone.
    if let Some(id) = target {
        if !config.modes_for(id).contains(&mode) {
            return Err(Failure::from(govee_toolkit::Error::ModeNotEnabled {
                id: id.clone(),
                mode,
            }));
        }
        config.devices.entry(id.clone()).or_default().modes = Some(vec![mode]);
    }
    Ok(config)
}

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

fn walk(
    color: &str,
    wait_ms: u64,
    hold_ms: u64,
    keep: bool,
    restrict: Option<Mode>,
) -> Result<Walk, Failure> {
    Ok(Walk {
        pass: Identify {
            color: args::rgb(color)?,
            ..Identify::default()
        },
        wait: Duration::from_millis(wait_ms),
        hold: Duration::from_millis(hold_ms),
        keep,
        // The rig a walk answers for is the rig on the network. `--mode`
        // names another one, and never widens the walk to two.
        mode: restrict.unwrap_or(Mode::Lan),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic, clippy::expect_used)]

    use clap::Parser as _;

    use super::*;

    /// The walk `Walk::default()` documents as the one `govee identify` runs.
    #[test]
    fn identify_with_no_option_runs_the_default_walk() {
        let cli = Cli::try_parse_from(["govee", "identify"]).expect("parses");
        let Command::Identify {
            color,
            wait_ms,
            hold_ms,
            keep,
            ..
        } = &cli.command
        else {
            panic!("parses as identify");
        };
        let walk = walk(color, *wait_ms, *hold_ms, *keep, None).expect("the default color reads");
        assert_eq!(walk, Walk::default());
    }
}
