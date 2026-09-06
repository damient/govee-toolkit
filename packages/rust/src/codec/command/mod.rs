//! Turns a device file entry and its arguments into something sendable.
//!
//! Nothing here knows a SKU or a command name: the device file supplies the
//! `cmd`, the `payload` template, the `frame` layout and the `reply` layout,
//! and this module only validates and substitutes.

use std::collections::{BTreeMap, BTreeSet};

use crate::codec::catalog::{ArgRole, Device, Mode};
use crate::codec::error::{Error, Result};
use crate::codec::reply::Layout as ReplyLayout;
use crate::codec::{Args, chunk, exchange};

mod resolve;

pub(crate) use resolve::placeholder;
use resolve::{resolve, substitute};

/// A command ready to send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Encoded {
    /// The value carried in `msg.cmd`, empty where the wire carries no
    /// envelope.
    pub cmd: String,
    /// The whole `{"msg":{"cmd":…,"data":…}}` envelope. `None` where the mode
    /// puts the frames on the wire with nothing wrapped around them.
    pub message: Option<serde_json::Value>,
    /// The raw frames, in the order they go out: one for a command that
    /// declares a `frame:`, several for a chunked one or for a list of
    /// exchanges, none for a command that travels in its envelope alone. A
    /// single frame also reaches `message` as base64 when the payload asks for
    /// it; the bytes stay here for tests and captures.
    pub frames: Vec<Vec<u8>>,
    /// The reply each frame expects, parallel to `frames`. Empty for a command
    /// that only writes.
    pub replies: Vec<Option<ReplyLayout>>,
    /// The role each captured field carries, where `args:` marks one: a
    /// transport builds a status from a reply without a field name reaching
    /// this crate.
    pub roles: BTreeMap<String, ArgRole>,
}

impl Encoded {
    /// The datagram payload: the envelope, serialized.
    ///
    /// # Errors
    ///
    /// [`Error::NoEnvelope`] for a command that carries none — a caller that
    /// asks for one is on the wrong wire, and a silent empty payload would take
    /// that as far as the device.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let message = self.message.as_ref().ok_or_else(|| Error::NoEnvelope {
            command: self.cmd.clone(),
        })?;
        serde_json::to_vec(message).map_err(|e| Error::Serialize {
            command: self.cmd.clone(),
            reason: e.to_string(),
        })
    }

    /// Every frame that expects an answer, paired with the layout to read it
    /// with. Empty for a command that only writes.
    #[must_use]
    pub fn reads(&self) -> Vec<(&[u8], &ReplyLayout)> {
        self.frames
            .iter()
            .zip(&self.replies)
            .filter_map(|(frame, reply)| Some((frame.as_slice(), reply.as_ref()?)))
            .collect()
    }
}

/// Encode one command of one device.
///
/// # Errors
///
/// See [`Error`]. Out-of-range values are rejected, never clamped: the firmware
/// clamps in silence, and hiding that behind a successful call would make the
/// SDK lie about what the device did.
pub fn encode(device: &Device, mode: Mode, command: &str, args: &Args) -> Result<Encoded> {
    use crate::codec::catalog::Support;

    // `Unknown` is not refused here: nobody probed the mode, so "unsupported"
    // would be a claim. The command table answers instead, and an empty one
    // produces `UnknownCommand`.
    if device.modes.get(mode).support == Support::None {
        return Err(Error::ModeUnsupported {
            sku: device.sku.clone(),
            mode,
        });
    }

    let spec = device
        .commands
        .get(mode)
        .get(command)
        .ok_or_else(|| Error::UnknownCommand {
            sku: device.sku.clone(),
            mode,
            command: command.to_owned(),
        })?;

    let exchanges = exchange::exchanges(command, spec, &spec.parsed_exchanges)?;
    let chunked = match (spec.body.as_deref(), spec.chunk.as_ref()) {
        (Some(body), Some(chunk)) => Some(chunk::layout(command, body, chunk, &spec.parsed_chunk)?),
        _ => None,
    };

    let sends: Vec<&crate::codec::Frame> = match (exchanges, chunked) {
        (Some(exchanges), _) => exchanges.sends().collect(),
        (None, Some(layout)) => vec![layout.body()],
        (None, None) => Vec::new(),
    };
    let captured: BTreeSet<&str> = exchanges
        .map(|e| e.capture_names().collect())
        .unwrap_or_default();
    let resolved = resolve(command, spec, &sends, &captured, args)?;

    let (frames, replies) = match (exchanges, chunked) {
        (Some(exchanges), _) => exchanges.build(command, &resolved)?,
        (None, Some(layout)) => (layout.build(command, &resolved)?, Vec::new()),
        (None, None) => (Vec::new(), Vec::new()),
    };

    // `${frame}` names one frame, so a chunked command and a list of exchanges
    // have none to name; the validator refuses a payload that asks for it.
    let single = (spec.frame.is_some() && spec.frames.is_empty())
        .then(|| frames.first().map(Vec::as_slice))
        .flatten();
    let data = substitute(command, &spec.payload, &resolved, single)?;

    let roles = captured
        .iter()
        .filter_map(|name| {
            let role = spec.args.get(*name)?.role()?;
            Some(((*name).to_owned(), role))
        })
        .collect();

    Ok(Encoded {
        cmd: spec.cmd.clone(),
        message: (!spec.cmd.is_empty())
            .then(|| serde_json::json!({ "msg": { "cmd": spec.cmd, "data": data } })),
        frames,
        replies,
        roles,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use crate::codec::args::{ArgValue, Args};
    use crate::codec::catalog::{ArgRole, Mode};
    use crate::codec::{Catalog, Encoded};

    /// A mode whose frames are a fixed size, with no envelope around them.
    const CHUNKED: &str = r#"
schema_version: 1
sku: HTEST
family: test
name: Test
capabilities: {}
commands:
  ble:
    provision:
      documented: true
      body: "${ssid:str8}"
      chunk:
        size: 16
        header: "A1 <op:11> 00 ${count} 00 <pad:20> <xor>"
        data: "A1 <op:11> ${index} ${chunk:bytes} <pad:20> <xor>"
        footer: "A1 <op:11> FF <pad:20> <xor>"
      args:
        ssid: { type: string, max_len: 32 }
"#;

    /// One entry that reads two values, neither named anywhere but here.
    const READS: &str = r#"
schema_version: 1
sku: HTEST
family: test
name: Test
capabilities: {}
commands:
  ble:
    state:
      documented: true
      role: status
      frames:
        - send: "AA <op:01> <pad:20> <xor>"
          reply: "AA 01 ${lit}"
        - send: "AA <op:04> <pad:20> <xor>"
          reply: "AA 04 ${level}"
      args:
        lit: { type: int, range: [0, 1], role: "on" }
        level: { type: int, range: [0, 100], role: brightness }
"#;

    fn encode(source: &str, command: &str, args: &Args) -> crate::codec::Result<Encoded> {
        let catalog = Catalog::from_sources([("HTEST.yaml", source)]).expect("the file parses");
        let device = catalog.device("HTEST").expect("the SKU resolves");
        super::encode(device, Mode::Ble, command, args)
    }

    #[test]
    fn a_chunked_command_encodes_to_a_frame_per_slice_and_no_envelope() {
        let encoded = encode(CHUNKED, "provision", &Args::new().text("ssid", "Test"))
            .expect("the command encodes");

        assert_eq!(encoded.message, None);
        assert_eq!(encoded.frames.len(), 3);
        assert!(encoded.frames.iter().all(|f| f.len() == 20));
        assert!(encoded.reads().is_empty());
        assert_eq!(encoded.to_bytes().unwrap_err().code(), "no_envelope");
    }

    #[test]
    fn a_string_past_the_cap_the_file_declares_is_refused() {
        let err = encode(
            CHUNKED,
            "provision",
            &Args::new().text("ssid", "0".repeat(33)),
        )
        .expect_err("the cap is enforced");
        assert_eq!(err.code(), "out_of_range");
    }

    #[test]
    fn a_list_of_exchanges_encodes_one_frame_and_one_layout_each() {
        let encoded = encode(READS, "state", &Args::new()).expect("the command encodes");

        assert_eq!(encoded.frames.len(), 2);
        assert_eq!(encoded.reads().len(), 2);
        assert_eq!(
            encoded.roles.get("lit").copied(),
            Some(ArgRole::On),
            "{:?}",
            encoded.roles
        );
        assert_eq!(
            encoded.roles.get("level").copied(),
            Some(ArgRole::Brightness)
        );
    }

    /// The device fills a captured field in, so the caller passes nothing for
    /// it. The file still declares it, so the reply has somewhere to land.
    #[test]
    fn a_captured_argument_is_not_one_the_caller_supplies() {
        let encoded = encode(READS, "state", &Args::new()).expect("nothing to supply");
        let reads = encoded.reads();
        let (_, layout) = reads.first().expect("the first exchange");
        let captured = layout
            .read("state", &[0xaa, 0x01, 0x01])
            .expect("it matches");
        assert_eq!(captured.get("lit"), Some(&ArgValue::Int(1)));
    }
}
