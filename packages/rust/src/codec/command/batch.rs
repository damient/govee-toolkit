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
    let mut batch: Vec<(Option<&serde_json::Value>, Encoded)> = Vec::new();
    for (command, args) in commands {
        let encoded = encode(device, mode, command, args)?;
        let template = shared(device, mode, command, &encoded);
        if let (Some(template), Some((Some(last_template), last))) = (template, batch.last_mut())
            && *last_template == template
            && last.cmd == encoded.cmd
        {
            last.frames.extend(encoded.frames);
            last.replies.extend(encoded.replies);
            let all = Frames {
                single: None,
                all: &last.frames,
            };
            let data = substitute(command, template, &Args::new(), &all)?;
            last.message = envelope(&last.cmd, &data);
            continue;
        }
        batch.push((template, encoded));
    }
    Ok(batch.into_iter().map(|(_, encoded)| encoded).collect())
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
