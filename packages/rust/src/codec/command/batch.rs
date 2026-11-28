//! Several commands of one device, with the frames of commands that share an
//! envelope carried in one envelope.

use super::resolve::{Frames, placeholder, substitute};
use super::{Encoded, encode, envelope};
use crate::codec::Args;
use crate::codec::catalog::{Device, Mode};
use crate::codec::error::Result;

/// Encode several commands of one device, in order.
///
/// Consecutive commands go out in one envelope where all of these hold:
///
/// - They carry the same `cmd` and the same `payload:` template.
/// - The template names `${frames}` and no other placeholder.
/// - No frame expects an answer.
///
/// Every other command keeps an envelope of its own. A mode with no envelope
/// keeps one [`Encoded`] per command.
///
/// # Errors
///
/// As for [`encode`].
pub fn encode_batch(
    device: &Device,
    mode: Mode,
    commands: &[(&str, &Args)],
) -> Result<Vec<Encoded>> {
    let mut batch: Vec<Run<'_>> = Vec::new();
    for (command, args) in commands {
        let encoded = encode(device, mode, command, args)?;
        let template = shared(device, mode, command, &encoded);
        if let (Some(template), Some(last)) = (template, batch.last_mut())
            && last.template == Some(template)
            && last.encoded.cmd == encoded.cmd
        {
            last.encoded.frames.extend(encoded.frames);
            last.encoded.replies.extend(encoded.replies);
            last.merged = true;
            continue;
        }
        batch.push(Run {
            command,
            template,
            encoded,
            merged: false,
        });
    }
    batch.into_iter().map(Run::finish).collect()
}

/// Consecutive commands that share one envelope.
struct Run<'a> {
    /// The first command, which names the run in an error.
    command: &'a str,
    template: Option<&'a serde_json::Value>,
    encoded: Encoded,
    /// More than one command: the message is built again from every frame.
    merged: bool,
}

impl Run<'_> {
    fn finish(mut self) -> Result<Encoded> {
        if let (true, Some(template)) = (self.merged, self.template) {
            let all = Frames {
                single: None,
                all: &self.encoded.frames,
            };
            let data = substitute(self.command, template, &Args::new(), &all)?;
            self.encoded.message = envelope(&self.encoded.cmd, &data);
        }
        Ok(self.encoded)
    }
}

/// The template of a command that can share its envelope, and `None` for
/// every other command.
fn shared<'a>(
    device: &'a Device,
    mode: Mode,
    command: &str,
    encoded: &Encoded,
) -> Option<&'a serde_json::Value> {
    if mode == Mode::Cloud || encoded.message.is_none() || encoded.answers() {
        return None;
    }
    let template = &device.commands.get(mode).get(command)?.payload;
    let mut names = Vec::new();
    placeholders(template, &mut names);
    (!names.is_empty() && names.iter().all(|name| *name == "frames")).then_some(template)
}

fn placeholders<'a>(value: &'a serde_json::Value, names: &mut Vec<&'a str>) {
    match value {
        serde_json::Value::String(s) => names.extend(placeholder(s)),
        serde_json::Value::Object(map) => map.values().for_each(|v| placeholders(v, names)),
        serde_json::Value::Array(items) => items.iter().for_each(|v| placeholders(v, names)),
        _ => {}
    }
}
