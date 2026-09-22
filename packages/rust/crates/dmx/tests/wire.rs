//! The whole chain, with no hardware: a datagram from a desk goes on the
//! socket, and the bytes come back off the device.
//!
//! Every other test here pushes a look into the send path. This one starts at
//! the wire: the node binds the loopback, `crates/sim` answers on it, and the
//! assertions read the datagrams the device took.
//!
//! One fixture is the simulated device and the other never answered, so the
//! run also carries a fixture that fails every write.

#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

mod common;

use std::net::{Ipv4Addr, SocketAddr};

use govee_toolkit::Govee;
use govee_toolkit::codec::Catalog;
use govee_toolkit_dmx::apply::Counts;
use govee_toolkit_dmx::input::socket::Listener;
use govee_toolkit_dmx::node::{Node, Observer};
use govee_toolkit_dmx::patch::Rig;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use self::common::{
    REACHED, artdmx, cmds, govee, payload, rig, simulator, timing, wait_for, wait_for_lit,
};

/// The node reports through an observer, and the assertions read the device.
struct Quiet;

impl Observer for Quiet {}

/// One universe of slots for the two fixtures of the patch: the simulated one
/// at address 1, and the one that never answered at address 7.
///
/// The dimmer at 255 is the top of the pair the device file declares, and the
/// three color slots go to the wire as they are.
fn slots() -> Vec<u8> {
    let mut slots = vec![0u8; 12];
    slots[0] = 255;
    slots[3] = 10;
    slots[4] = 20;
    slots[5] = 30;
    slots
}

/// The node, on its own task, and the desk that sends to it.
struct Running {
    /// Where a desk sends `ArtDmx`.
    address: SocketAddr,
    stop: oneshot::Sender<()>,
    task: JoinHandle<Vec<Counts>>,
}

impl Running {
    /// Stop the receive loop and answer what each device did.
    async fn close(self) -> Vec<Counts> {
        let _ = self.stop.send(());
        self.task.await.expect("the node task")
    }
}

fn loopback() -> SocketAddr {
    SocketAddr::from((Ipv4Addr::LOCALHOST, 0))
}

/// A live node, listening on an ephemeral loopback port.
///
/// The `ArtPollReply` goes to the loopback and not to the broadcast address,
/// so the test puts no packet on the network.
fn start(govee: &Govee, rig: Rig) -> Running {
    let listener = Listener::bind(loopback()).expect("the node socket binds");
    let address = listener.local_addr().expect("a bound address");
    let mut node = Node::live(rig, govee, timing()).replies_to(loopback());
    let (stop, stopped) = oneshot::channel();
    let task = tokio::spawn(async move {
        node.run(
            &listener,
            async {
                let _ = stopped.await;
            },
            &mut Quiet,
        )
        .await
        .expect("the receive loop");
        node.close().await
    });
    Running {
        address,
        stop,
        task,
    }
}

/// The desk: a socket that sends the datagrams a lighting console sends.
fn desk() -> Listener {
    Listener::bind(loopback()).expect("the desk socket binds")
}

/// A packet from a desk lights the device, and the bytes that arrive are the
/// ones the device file declares.
#[tokio::test]
async fn a_packet_from_a_desk_reaches_the_device() {
    let simulator = simulator().await;
    let govee = govee(&simulator).await;
    let catalog = Catalog::embedded().expect("the embedded catalog parses");
    let node = start(&govee, rig(&catalog, "hold"));
    let desk = desk();
    simulator.clear();

    desk.send_to(&artdmx(0, 1, &slots()), node.address)
        .await
        .expect("the send");
    wait_for_lit(&simulator).await;

    assert_eq!(
        payload(&simulator, "turn"),
        Some(serde_json::json!({ "value": 1 })),
    );
    assert_eq!(
        payload(&simulator, "brightness"),
        Some(serde_json::json!({ "value": 100 })),
    );
    assert_eq!(
        payload(&simulator, "colorwc"),
        Some(serde_json::json!({
            "color": { "r": 10, "g": 20, "b": 30 },
            "colorTemInKelvin": 0
        })),
    );
    node.close().await;
}

/// The values did not change, so the second packet puts nothing on the
/// device.
#[tokio::test]
async fn a_desk_that_holds_a_look_writes_the_device_once() {
    let simulator = simulator().await;
    let govee = govee(&simulator).await;
    let catalog = Catalog::embedded().expect("the embedded catalog parses");
    let node = start(&govee, rig(&catalog, "hold"));
    let desk = desk();
    simulator.clear();

    desk.send_to(&artdmx(0, 1, &slots()), node.address)
        .await
        .expect("the send");
    wait_for_lit(&simulator).await;
    let lit = cmds(&simulator);

    for sequence in 2..=5 {
        desk.send_to(&artdmx(0, sequence, &slots()), node.address)
            .await
            .expect("the send");
    }
    // Nothing is expected, so the wait has to run out: there is no arrival to
    // wait for.
    let more = wait_for(|| (cmds(&simulator).len() > lit.len()).then_some(())).await;
    assert!(
        more.is_none(),
        "the held look wrote again: {:?}",
        cmds(&simulator)
    );

    let counts = node.close().await;
    let reached = counts
        .iter()
        .find(|count| count.id == govee_toolkit::DeviceId::new(REACHED))
        .expect("the reached fixture reports its counts");
    assert_eq!(reached.frames_sent, 3, "power, brightness and color");
}
