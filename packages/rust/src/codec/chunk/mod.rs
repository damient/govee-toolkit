//! One payload split across several frames.
//!
//! A mode whose frames are a fixed size carries a longer payload as a start
//! frame, a run of data frames and an end frame. The device file writes the
//! layouts and the slice size; this module cuts the body and fills in the
//! reserved names. See `devices/schema.yaml`, `body:` and `chunk:`.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::codec::args::{ArgValue, Args};
use crate::codec::error::{Error, Result};
use crate::codec::frame::Frame;
use crate::codec::reply::Layout as ReplyLayout;

/// How many data frames follow the header.
pub const COUNT: &str = "count";
/// How many frames the transfer is, the header and the footer included.
pub const TOTAL: &str = "total";
/// Which data frame this is, counting from 1.
pub const INDEX: &str = "index";
/// The slice of the body one frame carries.
pub const CHUNK: &str = "chunk";

/// The names this module supplies. The validator refuses a command that
/// declares one of them as an argument.
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
    /// A frame sent after the transfer. Absent where the transfer is the
    /// whole command.
    pub then: Option<String>,
    /// The layout of the acknowledgement the transfer expects. Absent where
    /// nobody read one back.
    pub reply: Option<String>,
}

/// What a chunked command builds: every frame, and the layout that reads the
/// answer to it where one is expected.
pub type Built = (Vec<Vec<u8>>, Vec<Option<ReplyLayout>>);

/// A `chunk:` block and its `body:`, tokenized.
#[derive(Debug, Clone)]
pub struct Layout {
    size: usize,
    head_size: usize,
    body: Frame,
    header: Frame,
    data: Frame,
    footer: Frame,
    footer_takes_slice: bool,
    then: Option<Frame>,
    reply: Option<ReplyLayout>,
}

impl Layout {
    /// Parse the body layout and the three frame layouts.
    ///
    /// # Errors
    ///
    /// [`Error::ChunkSyntax`] if `size` is zero, and if `head_size:` and the
    /// header layout disagree on whether the header carries body bytes.
    /// [`Error::FrameSyntax`] if one of the layouts does not parse, and
    /// [`Error::ReplySyntax`] if the `reply:` layout does not.
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
        let footer = Frame::parse(command, &chunk.footer)?;
        let footer_takes_slice = footer.arg_names().any(|arg| arg == CHUNK);
        Ok(Self {
            size: chunk.size,
            head_size: chunk.head_size,
            body: Frame::parse(command, body)?,
            header,
            data: Frame::parse(command, &chunk.data)?,
            footer_takes_slice,
            footer,
            then: chunk
                .then
                .as_deref()
                .map(|then| Frame::parse(command, then))
                .transpose()?,
            reply: chunk
                .reply
                .as_deref()
                .map(|reply| ReplyLayout::parse(command, reply))
                .transpose()?,
        })
    }

    /// The layout of the payload to split.
    #[must_use]
    pub fn body(&self) -> &Frame {
        &self.body
    }

    /// Every layout in the block, the body included.
    pub fn frames(&self) -> impl Iterator<Item = &Frame> {
        [&self.body, &self.header, &self.data, &self.footer]
            .into_iter()
            .chain(self.then.as_ref())
    }

    /// Every name the `reply:` layout captures.
    pub fn capture_names(&self) -> impl Iterator<Item = &str> {
        self.reply.iter().flat_map(ReplyLayout::capture_names)
    }

    /// Build every frame of the transfer. Each one carries the layout that
    /// reads its answer, which only the footer has.
    ///
    /// # Errors
    ///
    /// Whatever building one frame raises — see [`Frame::build`].
    pub fn build(&self, command: &str, args: &Args) -> Result<Built> {
        let body = self.body.build(command, args)?;
        let (in_header, rest) = body.split_at(self.head_size.min(body.len()));
        let pieces: Vec<&[u8]> = rest.chunks(self.size).collect();

        let (data, in_footer): (&[&[u8]], &[u8]) = if self.footer_takes_slice {
            pieces.split_last().map_or((&[], &[]), |(l, d)| (d, l))
        } else {
            (&pieces, &[])
        };
        let count = i64::try_from(data.len()).unwrap_or(i64::MAX);
        let mut frames = Vec::with_capacity(data.len() + 3);
        let total = i64::try_from(data.len() + 2).unwrap_or(i64::MAX);

        // Replaced in place below: one clone of the arguments, not one per
        // slice.
        let mut filled = args.clone().int(COUNT, count).int(TOTAL, total);
        filled.insert(CHUNK, ArgValue::Bytes(in_header.to_vec()));
        frames.push(self.header.build(command, &filled)?);
        for (i, piece) in data.iter().enumerate() {
            let index = i64::try_from(i + 1).unwrap_or(i64::MAX);
            filled.insert(INDEX, ArgValue::Int(index));
            filled.insert(CHUNK, ArgValue::Bytes((*piece).to_vec()));
            frames.push(self.data.build(command, &filled)?);
        }
        filled.insert(CHUNK, ArgValue::Bytes(in_footer.to_vec()));
        frames.push(self.footer.build(command, &filled)?);
        let mut replies = vec![None; frames.len()];
        if let Some(last) = replies.last_mut() {
            last.clone_from(&self.reply);
        }
        if let Some(then) = &self.then {
            frames.push(then.build(command, args)?);
            replies.push(None);
        }
        Ok((frames, replies))
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
mod tests;
