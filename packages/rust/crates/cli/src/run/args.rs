//! Values a verb takes on the command line. Nothing here guesses a type:
//! `codec::coerce` reads every one, and the range stays the codec's to check.
//! `send` hands its `name=value` pairs to the crate instead — see
//! [`govee_toolkit::DeviceHandle::args`].

use govee_toolkit::codec::coerce;
use govee_toolkit::exit::Failure;
use govee_toolkit::stream::{ParseError, Resolution};
use tokio::io::{AsyncBufReadExt, BufReader};

pub(super) fn zones(text: &str) -> Result<Vec<u16>, Failure> {
    coerce::zones(text)
        .ok_or_else(|| Failure::usage(format!("`{text}` is not zone indices, zero-based")))
}

pub(super) fn rgb(text: &str) -> Result<[u8; 3], Failure> {
    coerce::rgb(text)
        .ok_or_else(|| Failure::usage(format!("`{text}` is not a color; write `#RRGGBB`")))
}

pub(super) fn colors(text: &str) -> Result<Vec<[u8; 3]>, Failure> {
    coerce::list(text).map(rgb).collect()
}

// `-` reads the list from one line of stdin, so a list of 42 colors reaches
// the device from a file or a pipe rather than from the command line.
pub(super) async fn colors_or_stdin(text: &str) -> Result<Vec<[u8; 3]>, Failure> {
    if text.trim() != "-" {
        return colors(text);
    }
    let line = BufReader::new(tokio::io::stdin())
        .lines()
        .next_line()
        .await
        .map_err(|e| Failure::internal(e.to_string()))?
        .unwrap_or_default();
    colors(&line)
}

pub(super) fn resolution(text: &str) -> Result<Resolution, Failure> {
    text.parse()
        .map_err(|e: ParseError| Failure::usage(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_color_reads_with_or_without_the_hash() {
        assert_eq!(rgb("#FF8000").ok(), Some([255, 128, 0]));
        assert_eq!(rgb("ff8000").ok(), Some([255, 128, 0]));
        assert!(rgb("#FF80").is_err());
    }

    #[test]
    fn a_color_list_reads_by_comma_or_by_space() {
        let wanted = Some(vec![[255, 0, 0], [0, 255, 0]]);
        assert_eq!(colors("#ff0000,#00ff00").ok(), wanted);
        assert_eq!(colors("ff0000 00ff00").ok(), wanted);
        assert_eq!(colors("#ff0000").ok(), Some(vec![[255, 0, 0]]));
        assert!(colors("#ff0000,green").is_err());
    }

    // The names themselves are the core's, and `stream::options` tests them.
    #[test]
    fn a_resolution_that_names_nothing_is_a_usage_failure() {
        assert_eq!(resolution("app").ok(), Some(Resolution::App));
        assert!(resolution("many").is_err());
    }
}
