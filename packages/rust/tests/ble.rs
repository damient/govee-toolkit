//! Dispatch to a mode that is not `lan`, through a transport that is not a
//! radio.
//!
//! The `ble` transport talks to hardware, and CI has none. These tests cover
//! what the facade does around it:
//!
//! - the transport that claims the enabled mode serves the device;
//! - the frames the codec built reach it, with nothing wrapped around them;
//! - fire-and-verify follows.

#![allow(
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
use govee_toolkit::{Args, Catalog, Config, Govee, Mode, State};
use tokio::sync::{broadcast, watch};

const DEVICE_FILE: &str = include_str!("fixtures/ble-device.yaml");
const SKU: &str = "HTEST4";
const MAC: &str = "AA:BB:CC:DD:EE:FF";

/// The power frame the fixture declares, at `on = 1`: two literal bytes, the
/// argument, zeros to twenty bytes, and the checksum.
const POWER_ON: [u8; 20] = [
    0x33, 0x01, 0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x33,
];
/// The two frames of the entry the fixture marks `role: status`.
const POWER_READ: [u8; 20] = [
    0xaa, 0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xab,
];
const BRIGHTNESS_READ: [u8; 20] = [
    0xaa, 0x04, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xae,
];

/// What the device answers, by the byte that names the request. A real radio
/// carries these on the notify characteristic; the fixture's layouts read them
/// either way.
fn answer(frame: &[u8]) -> Vec<u8> {
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

fn id() -> DeviceId {
    DeviceId::new(MAC)
}

/// A transport that claims `ble` and records every frame it receives.
#[derive(Debug)]
struct Fake {
    known: Option<DeviceId>,
    written: Mutex<Vec<Vec<u8>>>,
    verified: Mutex<Vec<Vec<u8>>>,
    events: broadcast::Sender<Event>,
    status: watch::Sender<Option<DeviceStatus>>,
}

impl Fake {
    fn knowing(id: &DeviceId) -> Arc<Self> {
        Arc::new(Self {
            known: Some(id.clone()),
            written: Mutex::new(Vec::new()),
            verified: Mutex::new(Vec::new()),
            events: broadcast::channel(16).0,
            status: watch::Sender::new(None),
        })
    }

    fn knowing_nothing() -> Arc<Self> {
        Arc::new(Self {
            known: None,
            written: Mutex::new(Vec::new()),
            verified: Mutex::new(Vec::new()),
            events: broadcast::channel(16).0,
            status: watch::Sender::new(None),
        })
    }

    fn written(&self) -> Vec<Vec<u8>> {
        self.written.lock().unwrap().clone()
    }

    fn verified(&self) -> Vec<Vec<u8>> {
        self.verified.lock().unwrap().clone()
    }

    fn holds(&self, id: &DeviceId) -> bool {
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

    async fn scan(&self, _window: Duration) -> Result<Vec<Discovered>> {
        Ok(Vec::new())
    }

    async fn send(&self, id: &DeviceId, command: &Encoded, verify: Verify<'_>) -> Result<Sent> {
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

    fn save_cache(&self) -> Result<()> {
        Ok(())
    }
}

fn catalog() -> Catalog {
    let mut catalog = Catalog::embedded().expect("catalog");
    catalog
        .overlay([("ble-device.yaml", DEVICE_FILE)])
        .expect("the fixture parses");
    catalog
}

fn govee(transport: &Arc<Fake>, yaml: &str) -> Govee {
    let config: Config = serde_norway::from_str(yaml).expect("the configuration parses");
    Govee::attach(
        config,
        catalog(),
        [Arc::clone(transport) as Arc<dyn Transport>],
    )
    .expect("the configuration applies")
}

fn enabling_ble() -> String {
    format!("defaults:\n  modes: [ble]\ndevices:\n  \"{MAC}\":\n    sku: \"{SKU}\"\n")
}

#[tokio::test]
async fn a_command_is_served_by_the_transport_claiming_the_enabled_mode() {
    let ble = Fake::knowing(&id());
    let govee = govee(&ble, &enabling_ble());

    let served = govee
        .device(&id())
        .send("power", &Args::new().int("on", 1))
        .await
        .expect("the command goes out");

    assert_eq!(served.mode, Mode::Ble);
    assert_eq!(served.command, "power");
    // Nothing on this wire carries a name: the envelope belongs to `lan`.
    assert_eq!(served.cmd, "");
}

#[tokio::test]
async fn what_reaches_the_transport_is_the_frames_and_nothing_around_them() {
    let ble = Fake::knowing(&id());
    let govee = govee(&ble, &enabling_ble());

    govee
        .device(&id())
        .send("power", &Args::new().int("on", 1))
        .await
        .expect("the command goes out");

    assert_eq!(ble.written(), vec![POWER_ON.to_vec()]);
}

#[tokio::test]
async fn a_command_carries_the_verification_the_device_file_names() {
    let ble = Fake::knowing(&id());
    let govee = govee(&ble, &enabling_ble());

    govee
        .device(&id())
        .send("power", &Args::new().int("on", 1))
        .await
        .expect("the command goes out");

    assert_eq!(
        ble.verified(),
        vec![POWER_READ.to_vec(), BRIGHTNESS_READ.to_vec()]
    );
}

#[tokio::test]
async fn a_status_request_is_the_entry_marked_with_the_role() {
    let ble = Fake::knowing(&id());
    let govee = govee(&ble, &enabling_ble());

    let status = govee.device(&id()).status().await.expect("it answers");

    // One entry, two exchanges: the file names both, and neither name is here.
    assert_eq!(
        ble.written(),
        vec![POWER_READ.to_vec(), BRIGHTNESS_READ.to_vec()]
    );
    assert_eq!(status.on, Some(true));
    assert_eq!(status.brightness, Some(100));
}

#[tokio::test]
async fn a_read_returns_the_fields_the_device_file_names() {
    let ble = Fake::knowing(&id());
    let govee = govee(&ble, &enabling_ble());

    let reply = govee
        .device(&id())
        .read("software", &Args::new())
        .await
        .expect("it answers");

    assert_eq!(
        reply.fields.get("version"),
        Some(&govee_toolkit::codec::ArgValue::Text("2.06.02".to_owned()))
    );
}

#[tokio::test]
async fn a_command_that_declares_no_reply_has_nothing_to_read() {
    let ble = Fake::knowing(&id());
    let govee = govee(&ble, &enabling_ble());

    let error = govee
        .device(&id())
        .read("power", &Args::new().int("on", 1))
        .await
        .expect_err("the fixture declares no reply for it");

    assert_eq!(error.code(), "no_reply_layout");
}

#[tokio::test]
async fn an_argument_out_of_range_is_refused_before_anything_is_written() {
    let ble = Fake::knowing(&id());
    let govee = govee(&ble, &enabling_ble());

    let error = govee
        .device(&id())
        .send("power", &Args::new().int("on", 2))
        .await
        .expect_err("2 is outside the declared range");

    assert_eq!(error.code(), "out_of_range");
    assert!(ble.written().is_empty());
}

#[tokio::test]
async fn a_device_the_transport_has_not_heard_of_is_not_reached_another_way() {
    let ble = Fake::knowing_nothing();
    let govee = govee(&ble, &enabling_ble());

    let error = govee
        .device(&id())
        .send("power", &Args::new().int("on", 1))
        .await
        .expect_err("nothing knows this device");

    assert_eq!(error.code(), "unknown_device");
    assert!(ble.written().is_empty());
}

#[tokio::test]
async fn a_mode_no_transport_serves_says_so_rather_than_choosing_another() {
    let ble = Fake::knowing(&id());
    let govee = govee(
        &ble,
        &format!("defaults:\n  modes: [lan]\ndevices:\n  \"{MAC}\":\n    sku: \"{SKU}\"\n"),
    );

    let error = govee
        .device(&id())
        .send("power", &Args::new().int("on", 1))
        .await
        .expect_err("this build attached no lan transport");

    assert_eq!(error.code(), "mode_not_implemented");
    assert!(ble.written().is_empty());
}
