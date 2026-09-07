//! The fixture device, and a transport that speaks to it instead of a radio.
//!
//! Answers from `tests/fixtures/ble-device.yaml` and records every frame it
//! is handed, so the suites around the facade need no hardware.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use govee_toolkit::codec::{Captured, Encoded};
use govee_toolkit::transport::{
    DeviceId, DeviceStatus, Discovered, Event, Health, KnownDevice, Reply, Result, Sent, Transport,
    Verify,
};
use govee_toolkit::{Catalog, Config, Govee, Mode, State};
use tokio::sync::{broadcast, watch};

pub(crate) const DEVICE_FILE: &str = include_str!("../fixtures/ble-device.yaml");
pub(crate) const SKU: &str = "HTEST4";
pub(crate) const MAC: &str = "AA:BB:CC:DD:EE:FF";

/// What the device answers, by the byte that names the request.
pub(crate) fn answer(frame: &[u8]) -> Vec<u8> {
    let mut reply = match frame[1] {
        0x01 => vec![0xaa, 0x01, 0x01],
        0x04 => vec![0xaa, 0x04, 0x64],
        0x21 => {
            let mut bytes = vec![0xaa, 0x21];
            bytes.extend_from_slice(b"2.06.02");
            bytes
        }
        other => panic!("the fixture declares no request {other:#04x}"),
    };
    reply.resize(20, 0);
    reply
}

/// The window this transport asks for, and no other mode's.
pub(crate) const SCAN_WINDOW: Duration = Duration::from_millis(7);

pub(crate) fn id() -> DeviceId {
    DeviceId::new(MAC)
}

/// A transport that claims `ble` and records every frame it receives.
#[derive(Debug)]
pub(crate) struct Fake {
    known: Option<DeviceId>,
    written: Mutex<Vec<Vec<u8>>>,
    scanned: Mutex<Vec<Duration>>,
    verified: Mutex<Vec<Vec<u8>>>,
    events: broadcast::Sender<Event>,
    status: watch::Sender<Option<DeviceStatus>>,
}

impl Fake {
    pub(crate) fn knowing(id: &DeviceId) -> Arc<Self> {
        Arc::new(Self {
            known: Some(id.clone()),
            written: Mutex::new(Vec::new()),
            scanned: Mutex::new(Vec::new()),
            verified: Mutex::new(Vec::new()),
            events: broadcast::channel(16).0,
            status: watch::Sender::new(None),
        })
    }

    pub(crate) fn knowing_nothing() -> Arc<Self> {
        Arc::new(Self {
            known: None,
            written: Mutex::new(Vec::new()),
            scanned: Mutex::new(Vec::new()),
            verified: Mutex::new(Vec::new()),
            events: broadcast::channel(16).0,
            status: watch::Sender::new(None),
        })
    }

    pub(crate) fn written(&self) -> Vec<Vec<u8>> {
        self.written.lock().unwrap().clone()
    }

    pub(crate) fn scanned(&self) -> Vec<Duration> {
        self.scanned.lock().unwrap().clone()
    }

    pub(crate) fn verified(&self) -> Vec<Vec<u8>> {
        self.verified.lock().unwrap().clone()
    }

    pub(crate) fn holds(&self, id: &DeviceId) -> bool {
        self.known.as_ref() == Some(id)
    }
}

#[async_trait]
impl Transport for Fake {
    fn mode(&self) -> Mode {
        Mode::Ble
    }

    fn events(&self) -> broadcast::Receiver<Event> {
        self.events.subscribe()
    }

    fn devices(&self) -> Vec<KnownDevice> {
        self.known
            .iter()
            .map(|id| KnownDevice {
                id: id.clone(),
                endpoint: "11:22:33:44:55:66".to_owned(),
                sku: SKU.to_owned(),
                health: Health {
                    state: State::Ok,
                    failures: 0,
                    available: true,
                },
            })
            .collect()
    }

    fn sku(&self, id: &DeviceId) -> Option<String> {
        self.holds(id).then(|| SKU.to_owned())
    }

    fn health(&self, id: &DeviceId) -> Option<Health> {
        self.holds(id).then_some(Health {
            state: State::Ok,
            failures: 0,
            available: true,
        })
    }

    fn last_status(&self, _id: &DeviceId) -> Option<DeviceStatus> {
        self.status.borrow().clone()
    }

    fn watch_status(&self, id: &DeviceId) -> Option<watch::Receiver<Option<DeviceStatus>>> {
        self.holds(id).then(|| self.status.subscribe())
    }

    fn scan_window(&self) -> Duration {
        SCAN_WINDOW
    }

    async fn scan(&self, window: Duration) -> Result<Vec<Discovered>> {
        self.scanned.lock().unwrap().push(window);
        Ok(Vec::new())
    }

    async fn send(&self, id: &DeviceId, command: &Encoded, verify: Verify) -> Result<Sent> {
        self.written
            .lock()
            .unwrap()
            .extend(command.frames.iter().cloned());
        if let Verify::With(request) = verify {
            self.verified
                .lock()
                .unwrap()
                .extend(request.frames.iter().cloned());
        }
        Ok(Sent {
            id: id.clone(),
            mode: Mode::Ble,
            cmd: command.cmd.clone(),
            endpoint: "11:22:33:44:55:66".to_owned(),
        })
    }

    async fn status(&self, id: &DeviceId, request: &Encoded) -> Result<DeviceStatus> {
        let reply = self.read(id, request).await?;
        Ok(DeviceStatus::from_captured(
            id.clone(),
            &reply.fields,
            &request.roles,
        ))
    }

    async fn read(&self, id: &DeviceId, request: &Encoded) -> Result<Reply> {
        self.written
            .lock()
            .unwrap()
            .extend(request.frames.iter().cloned());
        let exchanges = request.reads();
        if exchanges.is_empty() {
            return Err(govee_toolkit::transport::Error::NoReplyLayout {
                mode: Mode::Ble,
                reason: "the fixture declares no `reply:` for this command".to_owned(),
            });
        }
        let mut fields = Captured::new();
        for (frame, layout) in exchanges {
            fields.merge(
                layout
                    .read(&request.cmd, &answer(frame))
                    .expect("the answer matches the layout the fixture declares"),
            );
        }
        Ok(Reply {
            id: id.clone(),
            fields,
        })
    }
}

pub(crate) fn catalog() -> Catalog {
    let mut catalog = Catalog::embedded().expect("catalog");
    catalog
        .overlay([("ble-device.yaml", DEVICE_FILE)])
        .expect("the fixture parses");
    catalog
}

pub(crate) fn govee(transport: &Arc<Fake>, yaml: &str) -> Govee {
    let config: Config = serde_norway::from_str(yaml).expect("the configuration parses");
    Govee::attach(
        config,
        catalog(),
        [Arc::clone(transport) as Arc<dyn Transport>],
    )
    .expect("the configuration applies")
}

pub(crate) fn enabling_ble() -> String {
    format!("defaults:\n  modes: [ble]\ndevices:\n  \"{MAC}\":\n    sku: \"{SKU}\"\n")
}
