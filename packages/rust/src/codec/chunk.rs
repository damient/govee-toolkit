//! Splitting one payload across several frames.
//!
//! A mode whose frames are a fixed size carries a longer payload as a start
//! frame, a run of data frames and an end frame. The device file writes the
//! three layouts and the slice size; this module cuts the body and fills in the
//! three values the layouts need and the caller cannot know: how many data
//! frames there are, which one this is, and what it carries.
//!
//! ```yaml
//! body: "${ssid:str8} ${password:str8}"
//! chunk:
//!   size: 16
//!   header: "A1 <op:11> 00 ${count} 00 <pad:20> <xor>"
//!   data:   "A1 <op:11> ${index} ${chunk:bytes} <pad:20> <xor>"
//!   footer: "A1 <op:11> FF <pad:20> <xor>"
//! ```

use std::sync::OnceLock;

use serde::Deserialize;

use crate::codec::args::Args;
use crate::codec::error::{Error, Result};
use crate::codec::frame::Frame;

/// How many data frames follow the header.
pub const COUNT: &str = "count";
/// Which data frame this is, counting from 1.
pub const INDEX: &str = "index";
/// The slice of the body one data frame carries.
pub const CHUNK: &str = "chunk";

/// The names this module supplies. A command declaring one of them as an
/// argument would be writing over a value the codec fills in, so the validator
/// refuses it.
pub const RESERVED: [&str; 3] = [COUNT, INDEX, CHUNK];

/// A command's `chunk:` block.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Chunk {
    /// Bytes of the body one data frame carries.
    pub size: usize,
    /// The frame that opens the transfer.
    pub header: String,
    /// The frame that carries one slice.
    pub data: String,
    /// The frame that closes the transfer.
    pub footer: String,
}

/// A `chunk:` block and its `body:`, tokenized.
#[derive(Debug, Clone)]
pub struct Layout {
    size: usize,
    body: Frame,
    header: Frame,
    data: Frame,
    footer: Frame,
}

impl Layout {
    /// Parse the body layout and the three frame layouts.
    ///
    /// # Errors
    ///
    /// [`Error::ChunkSyntax`] if `size` is zero, and
    /// [`Error::FrameSyntax`] if one of the four layouts does not parse.
    pub fn parse(command: &str, body: &str, chunk: &Chunk) -> Result<Self> {
        if chunk.size == 0 {
            return Err(Error::ChunkSyntax {
                command: command.to_owned(),
                reason: "`size` must be at least one byte".to_owned(),
            });
        }
        Ok(Self {
            size: chunk.size,
            body: Frame::parse(command, body)?,
            header: Frame::parse(command, &chunk.header)?,
            data: Frame::parse(command, &chunk.data)?,
            footer: Frame::parse(command, &chunk.footer)?,
        })
    }

    /// The layout of the payload being split.
    #[must_use]
    pub fn body(&self) -> &Frame {
        &self.body
    }

    /// Every layout in the block, the body included.
    #[must_use]
    pub fn frames(&self) -> [&Frame; 4] {
        [&self.body, &self.header, &self.data, &self.footer]
    }

    /// Build the header, one data frame per slice of the body, and the footer.
    ///
    /// # Errors
    ///
    /// Whatever building one frame raises — see [`Frame::build`].
    pub fn build(&self, command: &str, args: &Args) -> Result<Vec<Vec<u8>>> {
        let body = self.body.build(command, args)?;
        let pieces: Vec<&[u8]> = body.chunks(self.size).collect();
        let count = i64::try_from(pieces.len()).unwrap_or(i64::MAX);

        let mut frames = Vec::with_capacity(pieces.len() + 2);
        frames.push(
            self.header
                .build(command, &args.clone().int(COUNT, count))?,
        );
        for (i, piece) in pieces.iter().enumerate() {
            let index = i64::try_from(i + 1).unwrap_or(i64::MAX);
            let args = args.clone().int(INDEX, index).bytes(CHUNK, *piece);
            frames.push(self.data.build(command, &args)?);
        }
        frames.push(self.footer.build(command, args)?);
        Ok(frames)
    }
}

/// The tokenized layout, parsed on first use.
pub(crate) fn layout<'a>(
    command: &str,
    body: &str,
    chunk: &Chunk,
    cache: &'a OnceLock<Layout>,
) -> Result<&'a Layout> {
    if let Some(layout) = cache.get() {
        return Ok(layout);
    }
    let parsed = Layout::parse(command, body, chunk)?;
    Ok(cache.get_or_init(|| parsed))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::format_collect)]

    use super::*;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn wifi() -> Layout {
        Layout::parse(
            "provision",
            "${ssid:str8} ${password:str8} ${run_mode} ${tz_hours} ${iot_version} ${tz_minutes}",
            &Chunk {
                size: 16,
                header: "A1 <op:11> 00 ${count} 00 <pad:20> <xor>".to_owned(),
                data: "A1 <op:11> ${index} ${chunk:bytes} <pad:20> <xor>".to_owned(),
                footer: "A1 <op:11> FF <pad:20> <xor>".to_owned(),
            },
        )
        .expect("the layouts parse")
    }

    /// SSID `Test`, password `abc`, UTC+2, no API block.
    #[test]
    fn the_worked_provisioning_example_matches_byte_for_byte() {
        let args = Args::new()
            .text("ssid", "Test")
            .text("password", "abc")
            .int("run_mode", 0)
            .int("tz_hours", 2)
            .int("iot_version", 0)
            .int("tz_minutes", 0);
        let frames = wifi().build("provision", &args).unwrap();

        assert_eq!(
            frames.iter().map(|f| hex(f)).collect::<Vec<_>>(),
            [
                "a1110001000000000000000000000000000000b1",
                "a1110104546573740361626300020000000000e2",
                "a111ff000000000000000000000000000000004f",
            ]
        );
        assert!(frames.iter().all(|f| f.len() == 20));
    }

    #[test]
    fn a_body_longer_than_one_slice_takes_one_data_frame_each() {
        let args = Args::new()
            .text("ssid", "0123456789abcdef")
            .text("password", "0123456789")
            .int("run_mode", 0)
            .int("tz_hours", 0)
            .int("iot_version", 0)
            .int("tz_minutes", 0);
        let frames = wifi().build("provision", &args).unwrap();

        // 1 + 16 + 1 + 10 + 4 = 32 body bytes, so two data frames.
        assert_eq!(frames.len(), 4);
        assert_eq!(frames.get(1).map(|f| f.get(2).copied()), Some(Some(1)));
        assert_eq!(frames.get(2).map(|f| f.get(2).copied()), Some(Some(2)));
        assert_eq!(frames.first().map(|f| f.get(3).copied()), Some(Some(2)));
    }

    #[test]
    fn a_slice_size_of_zero_would_never_terminate() {
        let err = Layout::parse("x", "${b:bytes}", &Chunk::default()).expect_err("no size");
        assert_eq!(err.code(), "chunk_syntax");
    }
}
