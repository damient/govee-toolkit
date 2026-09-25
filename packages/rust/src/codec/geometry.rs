//! The physical size of the model, for a host that places devices in a space.
//!
//! A property of the SKU: another length of one product is another SKU. The
//! size of the unit that the numbers under `measurements:` come from stays
//! there.

use serde::{Deserialize, Serialize};

/// A device file's `geometry:` block. Metres throughout.
///
/// A line declares `length_m` alone. A surface declares `width_m` and
/// `height_m`, and no `length_m`. [`crate::codec::validate`] checks that.
#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Geometry {
    /// The lit length of a strip or a rope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length_m: Option<f64>,
    /// The width of a surface.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width_m: Option<f64>,
    /// The height of a surface.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height_m: Option<f64>,
}

impl Geometry {
    /// Every problem with the block, as `(field, message)`. Empty means that
    /// the block is well-formed.
    #[must_use]
    pub fn problems(&self) -> Vec<(&'static str, String)> {
        let mut problems = Vec::new();
        for (field, value) in [
            ("length_m", self.length_m),
            ("width_m", self.width_m),
            ("height_m", self.height_m),
        ] {
            if let Some(value) = value
                && (!value.is_finite() || value <= 0.0)
            {
                problems.push((
                    field,
                    format!("is {value}; a size must be finite and above zero"),
                ));
            }
        }
        match (self.length_m, self.width_m, self.height_m) {
            (None, None, None) => problems.push((
                "",
                "declares nothing; give `length_m`, or `width_m` and `height_m`, or leave the \
                 block out"
                    .to_owned(),
            )),
            (Some(_), Some(_), _) | (Some(_), _, Some(_)) => problems.push((
                "length_m",
                "a line has no width or height; declare `length_m` or the pair, not both"
                    .to_owned(),
            )),
            (None, Some(_), None) | (None, None, Some(_)) => problems.push((
                "",
                "a surface declares `width_m` and `height_m` together".to_owned(),
            )),
            _ => {}
        }
        problems
    }
}

#[cfg(test)]
mod tests {
    use super::Geometry;

    fn fields(geometry: Geometry) -> Vec<&'static str> {
        geometry
            .problems()
            .into_iter()
            .map(|(field, _)| field)
            .collect()
    }

    #[test]
    fn a_line_or_a_surface_is_well_formed() {
        let line = Geometry {
            length_m: Some(3.0),
            ..Geometry::default()
        };
        let surface = Geometry {
            width_m: Some(0.3),
            height_m: Some(0.2),
            ..Geometry::default()
        };
        assert!(line.problems().is_empty());
        assert!(surface.problems().is_empty());
    }

    #[test]
    fn a_shape_that_mixes_or_misses_a_size_is_refused() {
        assert_eq!(fields(Geometry::default()), [""]);
        assert_eq!(
            fields(Geometry {
                length_m: Some(3.0),
                width_m: Some(1.0),
                ..Geometry::default()
            }),
            ["length_m"]
        );
        assert_eq!(
            fields(Geometry {
                width_m: Some(1.0),
                ..Geometry::default()
            }),
            [""]
        );
    }

    #[test]
    fn a_size_at_or_below_zero_is_refused() {
        assert_eq!(
            fields(Geometry {
                length_m: Some(0.0),
                ..Geometry::default()
            }),
            ["length_m"]
        );
        assert_eq!(
            fields(Geometry {
                length_m: Some(f64::NAN),
                ..Geometry::default()
            }),
            ["length_m"]
        );
    }
}
