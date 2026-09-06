//! One frame to send, and the reply it expects.
//!
//! A command that only writes declares a `frame:`. A command that reads
//! declares a `reply:` beside it, or a `frames:` list of send/reply pairs when
//! one answer is not enough:
//!
//! ```yaml
//! frames:
//!   - send:  "AA <op:01> <pad:20> <xor>"
//!     reply: "AA 01 ${on}"
//!   - send:  "AA <op:04> <pad:20> <xor>"
//!     reply: "AA 04 ${level}"
//! ```
//!
//! The pairs go out in the order they are written, and their captures merge
//! into one set of fields. One entry reads several values, and no field name
//! appears in SDK code.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::codec::args::Args;
use crate::codec::error::Result;
use crate::codec::frame::Frame;
use crate::codec::reply::Layout;

/// What one command's exchanges build: a frame per exchange, and the layout
/// each expects an answer under.
pub type Built = (Vec<Vec<u8>>, Vec<Option<Layout>>);

/// One entry of a command's `frames:` list.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Step {
    /// The frame layout to send. See [`crate::codec::frame`].
    pub send: String,
    /// The layout of the reply it expects. See [`crate::codec::reply`].
    pub reply: String,
}

/// One send layout and, where the file declares one, the reply it expects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exchange {
    send: Frame,
    reply: Option<Layout>,
}

impl Exchange {
    /// The reply layout, where the file declares one.
    #[must_use]
    pub fn reply(&self) -> Option<&Layout> {
        self.reply.as_ref()
    }
}

/// A command's exchanges, tokenized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exchanges(Vec<Exchange>);

impl Exchanges {
    /// Parse what a command declares: a `frames:` list, or a `frame:` with the
    /// `reply:` beside it.
    ///
    /// `None` for a command that declares neither — an envelope-only command,
    /// or a chunked one.
    ///
    /// # Errors
    ///
    /// [`Error::FrameSyntax`](crate::codec::Error::FrameSyntax) or
    /// [`Error::ReplySyntax`](crate::codec::Error::ReplySyntax) if a layout
    /// does not parse.
    pub fn parse(
        command: &str,
        frame: Option<&str>,
        reply: Option<&str>,
        steps: &[Step],
    ) -> Result<Option<Self>> {
        if !steps.is_empty() {
            let mut out = Vec::with_capacity(steps.len());
            for step in steps {
                out.push(Exchange {
                    send: Frame::parse(command, &step.send)?,
                    reply: parse_reply(command, Some(step.reply.as_str()))?,
                });
            }
            return Ok(Some(Self(out)));
        }
        let Some(frame) = frame else {
            return Ok(None);
        };
        Ok(Some(Self(vec![Exchange {
            send: Frame::parse(command, frame)?,
            reply: parse_reply(command, reply)?,
        }])))
    }

    /// Every send layout.
    pub fn sends(&self) -> impl Iterator<Item = &Frame> {
        self.0.iter().map(|exchange| &exchange.send)
    }

    /// Every name any reply layout captures.
    pub fn capture_names(&self) -> impl Iterator<Item = &str> {
        self.0
            .iter()
            .filter_map(Exchange::reply)
            .flat_map(Layout::capture_names)
    }

    /// Build every frame, paired with the reply layout it expects.
    ///
    /// # Errors
    ///
    /// Whatever building one frame raises — see [`Frame::build`].
    pub fn build(&self, command: &str, args: &Args) -> Result<Built> {
        let mut frames = Vec::with_capacity(self.0.len());
        let mut replies = Vec::with_capacity(self.0.len());
        for exchange in &self.0 {
            frames.push(exchange.send.build(command, args)?);
            replies.push(exchange.reply.clone());
        }
        Ok((frames, replies))
    }
}

fn parse_reply(command: &str, source: Option<&str>) -> Result<Option<Layout>> {
    match source.map(str::trim) {
        None | Some("") => Ok(None),
        Some(source) => Layout::parse(command, source).map(Some),
    }
}

/// The tokenized exchanges, parsed on first use: the device file fixes the
/// layouts, so the send path parses them once and not once per command.
pub(crate) fn exchanges<'a>(
    command: &str,
    spec: &'a crate::codec::catalog::Command,
    cache: &'a OnceLock<Option<Exchanges>>,
) -> Result<Option<&'a Exchanges>> {
    if let Some(parsed) = cache.get() {
        return Ok(parsed.as_ref());
    }
    let parsed = Exchanges::parse(
        command,
        spec.frame.as_deref(),
        spec.reply.as_deref(),
        &spec.frames,
    )?;
    Ok(cache.get_or_init(|| parsed).as_ref())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn a_frame_with_no_reply_is_one_exchange_that_reads_nothing() {
        let exchanges = Exchanges::parse("x", Some("33 01 ${on} <pad:20> <xor>"), None, &[])
            .unwrap()
            .expect("one exchange");
        assert_eq!(exchanges.sends().count(), 1);
        assert_eq!(exchanges.capture_names().count(), 0);
    }

    #[test]
    fn a_list_goes_out_in_the_order_it_is_written() {
        let steps = [
            Step {
                send: "AA <op:01> <pad:20> <xor>".to_owned(),
                reply: "AA 01 ${on}".to_owned(),
            },
            Step {
                send: "AA <op:04> <pad:20> <xor>".to_owned(),
                reply: "AA 04 ${level}".to_owned(),
            },
        ];
        let exchanges = Exchanges::parse("x", None, None, &steps)
            .unwrap()
            .expect("two exchanges");
        let (frames, replies) = exchanges.build("x", &Args::new()).unwrap();

        assert_eq!(frames.len(), 2);
        assert_eq!(frames.first().and_then(|f| f.get(1)), Some(&0x01));
        assert_eq!(frames.get(1).and_then(|f| f.get(1)), Some(&0x04));
        assert!(replies.iter().all(Option::is_some));
        assert_eq!(
            exchanges.capture_names().collect::<Vec<_>>(),
            ["on", "level"]
        );
    }

    #[test]
    fn a_command_that_declares_no_layout_has_no_exchanges() {
        assert!(Exchanges::parse("x", None, None, &[]).unwrap().is_none());
    }
}
