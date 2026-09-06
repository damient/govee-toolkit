//! Starting the facade: building the transports, applying the configuration,
//! and letting the user's own device files replace what the build shipped.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use tokio::sync::broadcast;

use crate::codec::{Catalog, Mode};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::govee::events::{Forwarder, forward};
use crate::govee::{Govee, Inner};
use crate::paths;
use crate::transport::Transport;

impl Govee {
    /// Start with the embedded catalog, plus the user's own device files if the
    /// configuration opts into them.
    ///
    /// # Errors
    ///
    /// [`Error::Codec`] if a device file does not parse,
    /// [`Error::LocalDevices`] if the local directory cannot be read,
    /// [`Error::Configuration`] if the configuration cannot be applied, or
    /// [`Error::Transport`] if a transport cannot be started.
    pub async fn start(config: Config) -> Result<Self> {
        let mut catalog = Catalog::embedded()?;
        if config.catalog.local_devices {
            overlay_local_devices(&mut catalog, &config)?;
        }
        Self::start_with(config, catalog).await
    }

    /// Start with a catalog the caller built.
    ///
    /// Every transport this build carries is started, whether or not a device
    /// enables it: which modes a device may use is per-device configuration,
    /// and starting one costs a socket at most — `ble` claims its adapter only
    /// when something needs it.
    ///
    /// # Errors
    ///
    /// See [`Govee::start`].
    pub async fn start_with(config: Config, catalog: Catalog) -> Result<Self> {
        let mut transports: Vec<Arc<dyn Transport>> = Vec::new();

        #[cfg(feature = "lan")]
        transports.push(Arc::new(
            crate::lan::Transport::start(config.lan.transport_options()?).await?,
        ));

        #[cfg(feature = "ble")]
        transports.push(Arc::new(crate::ble::Transport::start(
            crate::ble::Options::default(),
        )?));

        Self::attach(config, catalog, transports)
    }

    /// Use transports the caller already built.
    ///
    /// The seam for anything that has to bind its own sockets or hold its own
    /// adapter: a test against the simulator, or a host embedding the SDK
    /// beside other traffic.
    ///
    /// # Errors
    ///
    /// [`Error::Configuration`] if the configuration cannot be applied, or if
    /// two transports claim the same mode — one of them would never be
    /// reached, and silently dropping it is how a command ends up served by
    /// something the caller did not build.
    pub fn attach<I>(config: Config, catalog: Catalog, transports: I) -> Result<Self>
    where
        I: IntoIterator<Item = Arc<dyn Transport>>,
    {
        let problems = config.problems();
        if !problems.is_empty() {
            return Err(Error::Configuration(problems));
        }

        let mut by_mode: BTreeMap<Mode, Arc<dyn Transport>> = BTreeMap::new();
        for transport in transports {
            let mode = transport.mode();
            if by_mode.insert(mode, transport).is_some() {
                return Err(Error::Configuration(vec![crate::config::Problem {
                    device: None,
                    message: format!("two transports were attached for `{mode}`"),
                }]));
            }
        }

        let (events, _) = broadcast::channel(256);
        let inner = Arc::new(Inner {
            catalog,
            config,
            transports: by_mode,
            events: events.clone(),
            status_requests: Mutex::new(HashMap::new()),
        });

        let forwarder = Forwarder(
            inner
                .transports
                .values()
                .map(|transport| {
                    tokio::spawn(forward(
                        Arc::clone(&inner),
                        transport.events(),
                        events.clone(),
                    ))
                })
                .collect(),
        );

        let govee = Self {
            inner,
            _forwarder: Arc::new(forwarder),
        };

        // Whatever the caches already hold can be checked against the device
        // files right away, without waiting for a scan.
        let problems = govee.check_devices();
        for problem in &problems {
            tracing::warn!(%problem, "configuration");
        }
        Ok(govee)
    }
}

/// Let the user's own device files replace what the build shipped.
fn overlay_local_devices(catalog: &mut Catalog, config: &Config) -> Result<()> {
    let directory = config
        .catalog
        .directory
        .clone()
        .unwrap_or_else(paths::local_devices_dir);

    let entries = match std::fs::read_dir(&directory) {
        Ok(entries) => entries,
        // Opting in without having written any file yet is not an error.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => {
            return Err(Error::LocalDevices {
                path: directory.display().to_string(),
                reason: e.to_string(),
            });
        }
    };

    let mut files: Vec<(String, String)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "yaml") {
            continue;
        }
        let text = std::fs::read_to_string(&path).map_err(|e| Error::LocalDevices {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;
        files.push((path.display().to_string(), text));
    }
    files.sort();

    let replaced = catalog.overlay(files.iter().map(|(f, y)| (f.as_str(), y.as_str())))?;
    for overridden in replaced {
        // An override shadows what everyone else's build ships. It has to be
        // visible, every run.
        tracing::warn!(
            sku = %overridden.sku,
            was = %overridden.was,
            now = %overridden.now,
            "a local device file replaced the one shipped with this build"
        );
    }
    Ok(())
}
