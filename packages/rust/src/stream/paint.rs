//! The colors of one repaint, turned into the arguments of the frames that
//! carry them.

use crate::codec::Args;
use crate::error::{Error, Result};
use crate::stream::resolve::Painter;

/// The argument sets one repaint sends, in the order they go out.
///
/// # Errors
///
/// [`Error::ZoneOutOfRange`] for a zone the mask cannot name. The zone count is
/// bounded when the stream opens, so this is unreachable from a stream.
pub(super) fn frames(painter: &Painter, colors: Vec<[u8; 3]>) -> Result<Vec<Args>> {
    match painter {
        // The repeat count is left out on purpose: the codec derives it from
        // the list, which is the one place it cannot disagree with the colors
        // actually sent.
        Painter::Whole {
            colors: name,
            gradient,
            ..
        } => {
            let args = Args::new().rgb(name.as_str(), colors);
            Ok(vec![match gradient {
                Some((gradient, value)) => args.int(gradient.as_str(), *value),
                None => args,
            }])
        }
        Painter::Masked {
            colors: name,
            zones,
            ..
        } => masked(name, zones, &colors),
    }
}

/// One frame per distinct color, each naming every zone that uses it.
///
/// Zones that share a color go in one frame even when they are not adjacent.
/// The frames go out in the order the colors first appear.
fn masked(color_arg: &str, zone_arg: &str, colors: &[[u8; 3]]) -> Result<Vec<Args>> {
    let mut runs: Vec<([u8; 3], Vec<u16>)> = Vec::new();
    for (index, color) in colors.iter().enumerate() {
        let bit = u16::try_from(index).map_err(|_| Error::ZoneOutOfRange {
            index,
            zones: colors.len(),
        })?;
        match runs.iter_mut().find(|(seen, _)| seen == color) {
            Some((_, zones)) => zones.push(bit),
            None => runs.push((*color, vec![bit])),
        }
    }
    Ok(runs
        .into_iter()
        .map(|(color, zones)| {
            Args::new()
                .rgb(color_arg, vec![color])
                .zones(zone_arg, zones)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use super::*;
    use crate::codec::{ArgValue, Catalog, Mode};

    fn masked_painter() -> Painter {
        Painter::Masked {
            command: "seg".to_owned(),
            colors: "color".to_owned(),
            zones: "mask".to_owned(),
            limit: 15,
        }
    }

    fn zones_of(args: &Args) -> Vec<u16> {
        match args.get("mask") {
            Some(ArgValue::Zones(zones)) => zones.clone(),
            other => unreachable!("expected a zone list, got {other:?}"),
        }
    }

    #[test]
    fn one_color_over_every_zone_is_one_frame() {
        let frames = frames(&masked_painter(), vec![[255, 0, 0]; 4]).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(zones_of(&frames[0]), vec![0, 1, 2, 3]);
    }

    #[test]
    fn zones_sharing_a_color_travel_in_one_frame_even_when_apart() {
        let colors = [[255, 0, 0], [0, 0, 255], [255, 0, 0]];
        let frames = frames(&masked_painter(), colors.to_vec()).unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(zones_of(&frames[0]), vec![0, 2]);
        assert_eq!(zones_of(&frames[1]), vec![1]);
        assert_eq!(
            frames[0].get("color"),
            Some(&ArgValue::Rgb(vec![[255, 0, 0]]))
        );
    }

    #[test]
    fn a_masked_frame_carries_its_color_and_its_mask() {
        const MASKED: &str = include_str!("../../tests/fixtures/masked-zones.yaml");

        let catalog = Catalog::from_sources([("masked-zones.yaml", MASKED)]).expect("parses");
        let device = catalog.device("HTEST3").expect("the SKU resolves");
        let colors = [[255, 0, 0], [0, 0, 255], [255, 0, 0]];
        let frames = frames(&masked_painter(), colors.to_vec()).unwrap();

        let bytes: Vec<Vec<u8>> = frames
            .iter()
            .map(|args| {
                crate::codec::command::encode(device, Mode::Ble, "paint", args)
                    .expect("the device file encodes it")
                    .frames
                    .concat()
            })
            .collect();
        // Zones 0 and 2 in the first mask, zone 1 in the second.
        assert_eq!(bytes[0][4..9], [255, 0, 0, 0b0000_0101, 0]);
        assert_eq!(bytes[1][4..9], [0, 0, 255, 0b0000_0010, 0]);
    }

    #[test]
    fn a_whole_frame_painter_sends_every_color_at_once() {
        let painter = Painter::Whole {
            command: "seg".to_owned(),
            colors: "colors".to_owned(),
            gradient: Some(("gradient".to_owned(), 1)),
        };
        let frames = frames(&painter, vec![[1, 2, 3], [4, 5, 6]]).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(
            frames[0].get("colors"),
            Some(&ArgValue::Rgb(vec![[1, 2, 3], [4, 5, 6]]))
        );
        assert_eq!(frames[0].get("gradient"), Some(&ArgValue::Int(1)));
    }
}
