//! Indices grouped by color, for a layout that names each color once.

use super::fields::too_long;
use crate::codec::error::{Error, Result};

/// The group count, then for each color the index count, the color and the
/// indices, one byte each. Groups follow the order in which each color first
/// appears, and indices the order the caller gave.
///
/// `colors` holds one color for every index, or one color per index.
pub(super) fn groups(
    command: &str,
    (cells_name, cells): (&str, &[u16]),
    (colors_name, colors): (&str, &[[u8; 3]]),
) -> Result<Vec<u8>> {
    if colors.len() != 1 && colors.len() != cells.len() {
        return Err(Error::RepeatCountMismatch {
            command: command.to_owned(),
            count_arg: cells_name.to_owned(),
            list_arg: colors_name.to_owned(),
            declared: cells.len(),
            actual: colors.len(),
        });
    }

    let mut seen = std::collections::BTreeSet::new();
    let mut grouped: Vec<([u8; 3], Vec<u8>)> = Vec::new();
    for (i, cell) in cells.iter().enumerate() {
        let index = u8::try_from(*cell).map_err(|_| Error::OutOfRange {
            command: command.to_owned(),
            arg: cells_name.to_owned(),
            value: i64::from(*cell),
            min: 0,
            max: 255,
        })?;
        // One index under two colors would leave the firmware to pick one.
        if !seen.insert(index) {
            return Err(Error::ArgSyntax {
                command: command.to_owned(),
                arg: cells_name.to_owned(),
                expected: "each index once".to_owned(),
                got: index.to_string(),
            });
        }
        let color = match colors {
            [one] => *one,
            many => many.get(i).copied().unwrap_or_default(),
        };
        match grouped.iter_mut().find(|(c, _)| *c == color) {
            Some((_, indices)) => indices.push(index),
            None => grouped.push((color, vec![index])),
        }
    }

    let count = u8::try_from(grouped.len())
        .map_err(|_| too_long(command, colors_name, grouped.len(), 255))?;
    let mut out = vec![count];
    for (color, indices) in grouped {
        let n = u8::try_from(indices.len())
            .map_err(|_| too_long(command, cells_name, indices.len(), 255))?;
        out.push(n);
        out.extend_from_slice(&color);
        out.extend_from_slice(&indices);
    }
    Ok(out)
}
