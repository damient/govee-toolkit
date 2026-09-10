//! The state every [`Transport`](super::Transport) clone shares, and the send
//! path that reads it.
//!
//! Routing answers from memory, as it does for every mode. The link is what
//! differs: a device answers only over a connection, so the send path can have
//! to open one, and it opens one connection per device at a time.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::{broadcast, watch};

use crate::ble::link::{Link, adapter as adapter_error};
use crate::ble::pace::{Budget, Pacer};
use crate::ble::transport::Options;
use crate::ble::transport::drain::Drains;
use crate::ble::transport::links::Links;
use crate::ble::wire::{Adapter, Peripheral};
use crate::codec::{Encoded, Mode};
use crate::transport::DeviceId;
use crate::transport::breaker::{Breaker, Policy};
use crate::transport::error::{Error, Result};
use crate::transport::events::Event;
use crate::transport::registry::Devices;
use crate::transport::status::DeviceStatus;

/// One device, as the transport tracks it.
pub(super) struct Tracked {
    /// The handle the platform addresses the peripheral by. Not the identity:
    /// see [`super::Transport::bind`].
    pub(super) endpoint: String,
    pub(super) sku: String,
    pub(super) breaker: Breaker,
    pub(super) status: watch::Sender<Option<DeviceStatus>>,
    /// One budget per device, because the limit is one firmware's.
    pub(super) pacer: Arc<Pacer>,
    /// When verification last ran, so a burst of commands does not turn into a
    /// burst of probes.
    pub(super) verified_at: Option<Instant>,
}

impl crate::transport::registry::Tracked for Tracked {
    fn sku(&self) -> &str {
        &self.sku
    }

    fn breaker(&self) -> &Breaker {
        &self.breaker
    }

    fn breaker_mut(&mut self) -> &mut Breaker {
        &mut self.breaker
    }

    fn status(&self) -> &watch::Sender<Option<DeviceStatus>> {
        &self.status
    }

    fn verified_at(&mut self) -> &mut Option<Instant> {
        &mut self.verified_at
    }
}

impl Tracked {
    pub(super) fn new(endpoint: String, sku: String, policy: Policy, budget: Budget) -> Self {
        Self {
            endpoint,
            sku,
            breaker: Breaker::new(policy),
            status: watch::Sender::new(None),
            pacer: Arc::new(Pacer::new(budget)),
            verified_at: None,
        }
    }
}

/// Where to send, and what to send it with.
pub(super) struct Route {
    pub(super) endpoint: String,
    pub(super) pacer: Arc<Pacer>,
    /// Whether this command pays for a verification.
    pub(super) verifying: bool,
}

pub(super) struct Shared {
    pub(super) options: Options,
    /// The write budget for a device whose file records none, checked once when
    /// the transport was built.
    budget: Budget,
    /// One budget per SKU, from the device files, checked at the same time.
    per_sku: BTreeMap<String, Budget>,
    /// How long a link stays open after the last write, per SKU.
    pub(super) drains: Drains,
    /// The radio every command goes out on.
    pub(super) adapter: Arc<dyn Adapter>,
    pub(super) devices: Devices<Tracked>,
    /// One open connection per device, reused across commands. A device
    /// accepts only one, and a new connection costs seconds.
    pub(super) links: Links,
    pub(super) events: broadcast::Sender<Event>,
}

impl Shared {
    pub(super) fn new(
        options: Options,
        budget: Budget,
        per_sku: BTreeMap<String, Budget>,
        drains: Drains,
        events: broadcast::Sender<Event>,
        adapter: Arc<dyn Adapter>,
    ) -> Self {
        Self {
            options,
            drains,
            budget,
            per_sku,
            adapter,
            devices: Devices::new(),
            links: Links::new(),
            events,
        }
    }

    /// The budget for a device that advertises `sku`.
    ///
    /// A SKU no device file records a rate for is written at the fallback: no
    /// rate crosses from the unit it was measured on to another device.
    pub(super) fn budget_for(&self, sku: &str) -> Budget {
        self.per_sku
            .get(sku)
            .or_else(|| self.per_sku.get(&sku.to_uppercase()))
            .copied()
            .unwrap_or(self.budget)
    }

    /// Where to send, decided from recorded state alone.
    ///
    /// Claiming marks the device verified, so a burst of commands produces one
    /// probe.
    pub(super) fn route_and_claim(
        &self,
        id: &DeviceId,
        now: Instant,
        claim: bool,
    ) -> Result<Route> {
        let interval = if claim {
            self.options.verify_interval
        } else {
            None
        };
        let ((endpoint, pacer), verifying) =
            self.devices
                .route_and_claim(id, Mode::Ble, now, interval, |tracked| {
                    (tracked.endpoint.clone(), Arc::clone(&tracked.pacer))
                })?;
        Ok(Route {
            endpoint,
            pacer,
            verifying,
        })
    }

    /// The open connection to a device, or a new one if the slot is empty.
    ///
    /// The slot is not probed. A connection the device dropped is found by the
    /// write that fails on it, so the first command after a device goes away
    /// fails and the next one opens a connection.
    ///
    /// # Errors
    ///
    /// [`Error::ShutDown`] if the slot table is poisoned,
    /// [`Error::Unreachable`] if the connection takes longer than
    /// [`Options::connect_timeout`], or [`Error::Io`] if nothing advertises at
    /// the handle after a scan, or if the connection fails.
    pub(super) async fn link(&self, id: &DeviceId, endpoint: &str) -> Result<Arc<Link>> {
        let slot = self.links.slot(id)?;
        let mut open = slot.lock().await;
        if let Some(link) = open.as_ref() {
            return Ok(Arc::clone(link));
        }

        // A handle is good only while the platform still holds the peripheral
        // behind it, and macOS drops that when a link goes down. The device
        // must advertise again before anything can connect to it. A scan costs
        // seconds, so it runs only once the handle is gone.
        let peripheral = match self.peripheral(endpoint).await? {
            Some(peripheral) => peripheral,
            None => self.rediscover(endpoint).await?,
        };
        // A peripheral that never answers leaves `connect` pending for as long
        // as the platform waits, and every command to this device waits with
        // it.
        let timeout = self.options.connect_timeout;
        let link = tokio::time::timeout(timeout, Link::open(peripheral, endpoint))
            .await
            .map_err(|_| Error::Unreachable {
                id: id.clone(),
                endpoint: endpoint.to_owned(),
                timeout_ms: crate::transport::millis(timeout),
            })??;
        let link = Arc::new(link);
        *open = Some(Arc::clone(&link));
        Ok(link)
    }

    /// Forget a device's connection, so the next command opens a new one.
    pub(super) fn drop_link(&self, id: &DeviceId, link: &Arc<Link>) {
        self.links.forget(id, link);
    }

    /// Scan again, and answer with the peripheral that came back.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the scan fails, or if nothing is advertising at the
    /// handle once it has run.
    async fn rediscover(&self, endpoint: &str) -> Result<Arc<dyn Peripheral>> {
        self.scan(self.options.rescan_window).await?;
        self.peripheral(endpoint).await?.ok_or_else(|| {
            Error::io(
                format!("{endpoint}: nothing at this handle is advertising"),
                std::io::ErrorKind::NotFound.into(),
            )
        })
    }

    /// The peripheral behind a handle, among those the adapter has seen, or
    /// `None` if the adapter is not holding one.
    ///
    /// A handle that several peripherals carry names no device, and a
    /// connection to one of them would write the command into whatever the
    /// adapter listed first.
    ///
    /// # Errors
    ///
    /// [`Error::Io`] if the adapter cannot be listed, or if more than one
    /// peripheral carries the handle.
    async fn peripheral(&self, endpoint: &str) -> Result<Option<Arc<dyn Peripheral>>> {
        self.adapter
            .peripheral(endpoint)
            .await
            .map_err(|e| adapter_error(endpoint, "listing known peripherals", e))
    }

    /// Write every frame of a command, at the device's budget.
    ///
    /// A failed write drops the link: the connection is gone or the device
    /// went away, and the next command must open a new one rather than write
    /// into a dead handle.
    pub(super) async fn write_frames(
        &self,
        id: &DeviceId,
        route: &Route,
        link: &Arc<Link>,
        command: &Encoded,
    ) -> Result<()> {
        check_frames(command)?;
        for frame in &command.frames {
            self.write_frame(id, route, link, &command.cmd, frame)
                .await?;
        }
        Ok(())
    }

    /// Hand a status to the device's watchers and to the event stream.
    pub(super) fn publish_status(&self, status: DeviceStatus) {
        self.devices.publish_status(&self.events, Mode::Ble, status);
    }

    /// Feed the breaker and publish the transition, if there was one.
    pub(super) fn record(&self, id: &DeviceId, answered: bool, now: Instant) {
        self.devices
            .record(&self.events, id, Mode::Ble, answered, now);
    }

    /// [`Shared::link`], with a failure recorded against the breaker.
    ///
    /// A device that will not take a connection is unreachable, and the record
    /// spares the next command the same wait.
    ///
    /// # Errors
    ///
    /// As for [`Shared::link`].
    pub(super) async fn connect(&self, id: &DeviceId, endpoint: &str) -> Result<Arc<Link>> {
        match self.link(id, endpoint).await {
            Ok(link) => Ok(link),
            Err(e) => {
                self.record(id, false, Instant::now());
                Err(e)
            }
        }
    }
}

/// Which identity a handle is tracked under, if any.
pub(super) fn id_at(devices: &HashMap<DeviceId, Tracked>, endpoint: &str) -> Option<DeviceId> {
    devices.iter().find_map(|(id, tracked)| {
        tracked
            .endpoint
            .eq_ignore_ascii_case(endpoint)
            .then(|| id.clone())
    })
}

/// Refuse a command this wire has nothing to send for.
///
/// # Errors
///
/// [`Error::Serialize`] if the command carries no frames: `ble` writes frames
/// and nothing else, so an envelope-only command is a device file describing
/// another mode.
pub(super) fn check_frames(command: &Encoded) -> Result<()> {
    if command.frames.is_empty() {
        return Err(Error::Serialize {
            cmd: command.cmd.clone(),
            reason: "the command carries no frames, and this wire carries nothing else".to_owned(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use super::*;

    #[test]
    fn a_command_with_nothing_to_write_is_refused() {
        let command = Encoded {
            cmd: "power".to_owned(),
            message: Some(serde_json::json!({"msg": {}})),
            frames: Vec::new(),
            replies: Vec::new(),
            roles: std::collections::BTreeMap::new(),
            request: None,
        };
        let error = check_frames(&command).expect_err("nothing to write");
        assert_eq!(error.code(), "serialize");

        let carried = Encoded {
            frames: vec![vec![0; 20]],
            ..command
        };
        assert!(check_frames(&carried).is_ok());
    }
}
