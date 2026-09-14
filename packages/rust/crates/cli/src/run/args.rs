//! Values a person types, read against the type the device file declares.
//! Nothing here guesses a type: `codec::coerce` reads every value under the
//! declared `ArgSpec`, and the range stays the codec's to check.

use govee_toolkit::codec::coerce::{self, Supplied};
use govee_toolkit::codec::{ArgSpec, ArgValue};
use govee_toolkit::stream::Resolution;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::output::Failure;

// The name `devices/schema.yaml` uses, so a message names the type the file
// names.
pub(super) fn kind(spec: &ArgSpec) -> &'static str {
    match spec {
        ArgSpec::Int { .. } => "int",
        ArgSpec::RgbList { .. } => "rgb_list",
        ArgSpec::String { .. } => "string",
        ArgSpec::Zones { .. } => "zones",
        ArgSpec::Bytes { .. } => "bytes",
    }
}

// A value of the right type but outside the declared range reaches the codec,
// which refuses it there.
pub(super) fn parse(
    command: &str,
    name: &str,
    spec: &ArgSpec,
    text: &str,
) -> Result<ArgValue, Failure> {
    coerce::read(command, name, spec, Supplied::Text(text.to_owned()))
        .map_err(|e| Failure::usage(e.to_string()))
}

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
    match text {
        "app" => Ok(Resolution::App),
        "native" => Ok(Resolution::Native),
        other => other.parse::<u16>().map(Resolution::Exact).map_err(|_| {
            Failure::usage(format!(
                "`{other}` is not a zone count; write `app`, `native`, or a number"
            ))
        }),
    }
}

#[cfg(test)]
mod tests {
    use govee_toolkit::codec::Bounds;

    use super::*;

    fn int() -> ArgSpec {
        ArgSpec::Int {
            range: Some(Bounds::Literal([0, 100])),
            role: None,
        }
    }

    #[test]
    fn a_value_is_read_under_the_declared_type() {
        assert_eq!(
            parse("brightness", "level", &int(), "50").ok(),
            Some(ArgValue::Int(50))
        );
        assert_eq!(
            parse(
                "segment",
                "zones",
                &ArgSpec::Zones {
                    count: None,
                    role: None
                },
                "0, 2,3"
            )
            .ok(),
            Some(ArgValue::Zones(vec![0, 2, 3]))
        );
    }

    #[test]
    fn a_value_of_the_wrong_type_is_refused() {
        assert!(parse("brightness", "level", &int(), "bright").is_err());
    }

    #[test]
    fn a_range_belongs_to_the_codec_and_is_not_checked_here() {
        assert_eq!(
            parse("brightness", "level", &int(), "400").ok(),
            Some(ArgValue::Int(400))
        );
    }

    #[test]
    fn bytes_read_with_or_without_separators() {
        let spec = ArgSpec::Bytes {
            max_len: None,
            role: None,
        };
        let wanted = Some(ArgValue::Bytes(vec![0xAA, 0x0F, 0x01]));
        assert_eq!(parse("raw", "data", &spec, "aa 0f 01").ok(), wanted);
        assert_eq!(parse("raw", "data", &spec, "aa:0f:01").ok(), wanted);
        assert_eq!(parse("raw", "data", &spec, "aa0f01").ok(), wanted);
        assert!(parse("raw", "data", &spec, "aa0").is_err());
    }

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

    #[test]
    fn a_resolution_reads_by_name_or_by_number() {
        assert_eq!(resolution("app").ok(), Some(Resolution::App));
        assert_eq!(resolution("native").ok(), Some(Resolution::Native));
        assert_eq!(resolution("30").ok(), Some(Resolution::Exact(30)));
        assert!(resolution("many").is_err());
    }
}
