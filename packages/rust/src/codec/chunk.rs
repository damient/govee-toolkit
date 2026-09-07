//! One payload split across several frames.
//!
//! A mode whose frames are a fixed size carries a longer payload as a start
//! frame, a run of data frames and an end frame. The device file writes the
//! three layouts and the slice size; this module cuts the body and fills in
//! `count`, `total`, `index` and `chunk` — see `devices/schema.yaml`.
//!
//! ```yaml
//! body: "${ssid:str8} ${password:str8}"
//! chunk:
//!   size: 16
//!   header: "A1 <op:11> 00 ${count} 00 <pad:20> <xor>"
//!   data:   "A1 <op:11> ${index} ${chunk:bytes} <pad:20> <xor>"
//!   footer: "A1 <op:11> FF <pad:20> <xor>"
//! ```
//!
//! Where the body starts and stops is the layouts' to say, because two
//! dialects of one wire differ: a `head_size:` puts the first slice in the
//! header, and a footer that reads `${chunk:bytes}` carries the last. A
//! `then:` frame goes out after the transfer, for a wire that stores what was
//! transferred and plays it with a second frame.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::codec::args::Args;
use crate::codec::error::{Error, Result};
use crate::codec::frame::Frame;

/// How many data frames follow the header.
pub const COUNT: &str = "count";
/// How many frames the transfer is, the header and the footer included.
pub const TOTAL: &str = "total";
/// Which data frame this is, counting from 1.
pub const INDEX: &str = "index";
/// The slice of the body one frame carries.
pub const CHUNK: &str = "chunk";

/// The names this module supplies. The validator refuses a command that
/// declares one of them as an argument: it would overwrite a value the codec
/// fills in.
pub const RESERVED: [&str; 4] = [COUNT, TOTAL, INDEX, CHUNK];

/// A command's `chunk:` block.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Chunk {
    /// Bytes of the body one data frame carries.
    pub size: usize,
    /// Bytes of the body the header carries, before the first data frame.
    /// Zero means the header carries none.
    pub head_size: usize,
    /// The frame that opens the transfer.
    pub header: String,
    /// The frame that carries one slice.
    pub data: String,
    /// The frame that closes the transfer.
    pub footer: String,
    /// A frame sent after the transfer. Empty where the transfer is the whole
    /// command.
    pub then: String,
}

/// A `chunk:` block and its `body:`, tokenized.
#[derive(Debug, Clone)]
pub struct Layout {
    size: usize,
    head_size: usize,
    body: Frame,
    header: Frame,
    data: Frame,
    footer: Frame,
    then: Option<Frame>,
}

impl Layout {
    /// Parse the body layout and the three frame layouts.
    ///
    /// # Errors
    ///
    /// [`Error::ChunkSyntax`] if `size` is zero, and if `head_size:` and the
    /// header layout disagree on whether the header carries body bytes.
    /// [`Error::FrameSyntax`] if one of the layouts does not parse.
    pub fn parse(command: &str, body: &str, chunk: &Chunk) -> Result<Self> {
        let bad = |reason: &str| Error::ChunkSyntax {
            command: command.to_owned(),
            reason: reason.to_owned(),
        };
        if chunk.size == 0 {
            return Err(bad("`size` must be at least one byte"));
        }
        let header = Frame::parse(command, &chunk.header)?;
        let carries_body = header.arg_names().any(|arg| arg == CHUNK);
        if carries_body && chunk.head_size == 0 {
            return Err(bad(
                "the header reads `${chunk:bytes}`, so `head_size:` must say how many bytes it takes",
            ));
        }
        if !carries_body && chunk.head_size > 0 {
            return Err(bad(
                "`head_size:` is set, but the header layout reads no `${chunk:bytes}`",
            ));
        }
        Ok(Self {
            size: chunk.size,
            head_size: chunk.head_size,
            body: Frame::parse(command, body)?,
            header,
            data: Frame::parse(command, &chunk.data)?,
            footer: Frame::parse(command, &chunk.footer)?,
            then: (!chunk.then.trim().is_empty())
                .then(|| Frame::parse(command, &chunk.then))
                .transpose()?,
        })
    }

    /// The layout of the payload to split.
    #[must_use]
    pub fn body(&self) -> &Frame {
        &self.body
    }

    /// Every layout in the block, the body included.
    #[must_use]
    pub fn frames(&self) -> Vec<&Frame> {
        let mut out = vec![&self.body, &self.header, &self.data, &self.footer];
        out.extend(self.then.as_ref());
        out
    }

    /// Build the header, one data frame per slice of the body, the footer, and
    /// the `then:` frame where the block declares one.
    ///
    /// # Errors
    ///
    /// Whatever building one frame raises — see [`Frame::build`].
    pub fn build(&self, command: &str, args: &Args) -> Result<Vec<Vec<u8>>> {
        let body = self.body.build(command, args)?;
        let (in_header, rest) = body.split_at(self.head_size.min(body.len()));
        let pieces: Vec<&[u8]> = rest.chunks(self.size).collect();

        // A footer that reads the slice takes the last piece, so it is not a
        // data frame. With no piece at all it carries an empty one.
        let (data, in_footer): (&[&[u8]], &[u8]) = if self.footer.arg_names().any(|a| a == CHUNK) {
            pieces.split_last().map_or((&[], &[]), |(l, d)| (d, l))
        } else {
            (&pieces, &[])
        };
        let count = i64::try_from(data.len()).unwrap_or(i64::MAX);
        let total = count.saturating_add(2);
        let filled = |args: Args| args.int(COUNT, count).int(TOTAL, total);

        let mut frames = Vec::with_capacity(data.len() + 3);
        frames.push(
            self.header
                .build(command, &filled(args.clone()).bytes(CHUNK, in_header))?,
        );
        for (i, piece) in data.iter().enumerate() {
            let index = i64::try_from(i + 1).unwrap_or(i64::MAX);
            let args = filled(args.clone()).int(INDEX, index).bytes(CHUNK, *piece);
            frames.push(self.data.build(command, &args)?);
        }
        frames.push(
            self.footer
                .build(command, &filled(args.clone()).bytes(CHUNK, in_footer))?,
        );
        if let Some(then) = &self.then {
            frames.push(then.build(command, args)?);
        }
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
                ..Chunk::default()
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

    /// The dialect of `docs/protocol/ble.md` 6: the header carries the first
    /// thirteen bytes, the closing frame carries the last piece, and the count
    /// in the header is every frame of the transfer.
    fn music() -> Layout {
        Layout::parse(
            "music",
            "${colors_count} (${colors}:rgb)×${colors_count} ${tail:bytes}",
            &Chunk {
                size: 17,
                head_size: 13,
                header: "A3 00 01 ${total} 41 ${effect} ${chunk:bytes} <pad:20> <xor>".to_owned(),
                data: "A3 ${index} ${chunk:bytes} <pad:20> <xor>".to_owned(),
                footer: "A3 FF ${chunk:bytes} <pad:20> <xor>".to_owned(),
                then: "33 05 13 ${effect} ${sensitivity} <pad:20> <xor>".to_owned(),
            },
        )
        .expect("the layouts parse")
    }

    /// The seven colours the vendor app sends with no saved palette.
    fn palette() -> Args {
        Args::new().rgb(
            "colors",
            [
                [255, 0, 0],
                [255, 127, 0],
                [255, 255, 0],
                [0, 255, 0],
                [0, 0, 255],
                [0, 255, 255],
                [139, 0, 255],
            ],
        )
    }

    /// Effect 50, sent to an H61A0 and rendered by it. A 25-byte body fits in
    /// the header and the closing frame, so no data frame goes out.
    #[test]
    fn a_body_that_fits_the_header_and_the_footer_sends_two_frames_and_a_then() {
        let args = palette()
            .int("colors_count", 7)
            .bytes("tail", [3, 0, 99])
            .int("effect", 50)
            .int("sensitivity", 99);
        let frames = music().build("music", &args).unwrap();

        assert_eq!(
            frames.iter().map(|f| hex(f)).collect::<Vec<_>>(),
            [
                "a3000102413207ff0000ff7f00ffff0000ff0054",
                "a3ff0000ff00ffff8b00ff0300630000000000b7",
                "3305133263000000000000000000000000000074",
            ]
        );
        assert!(frames.iter().all(|f| f.len() == 20));
    }

    /// Effect 51, the same way. A 31-byte body leaves 18 bytes after the
    /// header: one full data frame, and one byte in the closing frame.
    #[test]
    fn a_body_past_one_piece_puts_the_last_piece_in_the_closing_frame() {
        let args = palette()
            .int("colors_count", 7)
            .bytes("tail", [1, 1, 1, 25, 98, 1, 3, 6, 17])
            .int("effect", 51)
            .int("sensitivity", 99);
        let frames = music().build("music", &args).unwrap();

        assert_eq!(
            frames.iter().map(|f| hex(f)).collect::<Vec<_>>(),
            [
                "a3000103413307ff0000ff7f00ffff0000ff0054",
                "a3010000ff00ffff8b00ff010101196201030657",
                "a3ff11000000000000000000000000000000004d",
                "3305133363000000000000000000000000000075",
            ]
        );
    }

    #[test]
    fn a_header_that_carries_body_bytes_has_to_say_how_many() {
        let err = Layout::parse(
            "music",
            "${tail:bytes}",
            &Chunk {
                size: 17,
                header: "A3 00 ${chunk:bytes} <pad:20> <xor>".to_owned(),
                data: "A3 ${index} ${chunk:bytes} <pad:20> <xor>".to_owned(),
                footer: "A3 FF <pad:20> <xor>".to_owned(),
                ..Chunk::default()
            },
        )
        .expect_err("head_size is missing");
        assert_eq!(err.code(), "chunk_syntax");
    }

    #[test]
    fn a_head_size_with_no_slice_in_the_header_is_refused() {
        let err = Layout::parse(
            "music",
            "${tail:bytes}",
            &Chunk {
                size: 17,
                head_size: 13,
                header: "A3 00 <pad:20> <xor>".to_owned(),
                data: "A3 ${index} ${chunk:bytes} <pad:20> <xor>".to_owned(),
                footer: "A3 FF <pad:20> <xor>".to_owned(),
                ..Chunk::default()
            },
        )
        .expect_err("the header takes no slice");
        assert_eq!(err.code(), "chunk_syntax");
    }
}
