//! The physical size of the model, for a host that places devices in a space.
//!
//! A property of the SKU: another length of one product is another SKU. The
//! size of the unit that the numbers under `measurements:` come from stays
//! there.

use std::fmt;

use serde::de::value::MapAccessDeserializer;
use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

/// A device file's `geometry:` block. Metres throughout, each size finite and
/// above zero.
///
/// The device file fails to load where the block declares another shape or
/// another size, and the error names the field.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum Geometry {
    /// A strip or a rope.
    Line {
        /// The lit length.
        length_m: f64,
    },
    /// A panel or another flat shape.
    Surface {
        /// The width.
        width_m: f64,
        /// The height.
        height_m: f64,
    },
}

/// The block as the file writes it, before the shape is known.
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[expect(
    clippy::struct_field_names,
    reason = "the fields are the keys a device file writes"
)]
struct Declared {
    length_m: Option<f64>,
    width_m: Option<f64>,
    height_m: Option<f64>,
}

impl TryFrom<Declared> for Geometry {
    type Error = String;

    fn try_from(declared: Declared) -> Result<Self, String> {
        let geometry = match (declared.length_m, declared.width_m, declared.height_m) {
            (Some(length_m), None, None) => Self::Line { length_m },
            (None, Some(width_m), Some(height_m)) => Self::Surface { width_m, height_m },
            (None, None, None) => {
                return Err("declares nothing; give `length_m`, or `width_m` and \
                            `height_m`, or leave the block out"
                    .to_owned());
            }
            (Some(_), ..) => {
                return Err("a line has no width or height; declare `length_m` or the \
                            pair, not both"
                    .to_owned());
            }
            (None, ..) => {
                return Err("a surface declares `width_m` and `height_m` together".to_owned());
            }
        };
        let sizes: &[(&str, f64)] = match geometry {
            Self::Line { length_m } => &[("length_m", length_m)],
            Self::Surface { width_m, height_m } => &[("width_m", width_m), ("height_m", height_m)],
        };
        for &(field, value) in sizes {
            if !value.is_finite() || value <= 0.0 {
                return Err(format!(
                    "`{field}` is {value}; a size must be finite and above zero"
                ));
            }
        }
        Ok(geometry)
    }
}

impl<'de> Deserialize<'de> for Geometry {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(Shape)
    }
}

/// Checks the shape inside `visit_map`: an error raised there carries the
/// path and the position, and an error raised after the block does not.
struct Shape;

impl<'de> Visitor<'de> for Shape {
    type Value = Geometry;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("`length_m`, or `width_m` and `height_m`")
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Geometry, A::Error> {
        let declared = Declared::deserialize(MapAccessDeserializer::new(map))?;
        Geometry::try_from(declared).map_err(de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::Geometry;

    #[derive(Debug, serde::Deserialize)]
    struct File {
        geometry: Geometry,
    }

    fn read(block: &str) -> Result<Geometry, String> {
        serde_norway::from_str::<File>(&format!("geometry: {block}"))
            .map(|file| file.geometry)
            .map_err(|e| e.to_string())
    }

    #[test]
    fn a_line_or_a_surface_reads_as_its_shape() {
        assert_eq!(
            read("{ length_m: 3 }"),
            Ok(Geometry::Line { length_m: 3.0 })
        );
        assert_eq!(
            read("{ width_m: 0.3, height_m: 0.2 }"),
            Ok(Geometry::Surface {
                width_m: 0.3,
                height_m: 0.2
            })
        );
    }

    #[test]
    fn a_shape_writes_back_the_fields_it_read() {
        let line = serde_json::to_value(Geometry::Line { length_m: 3.0 }).expect("serializes");
        assert_eq!(line, serde_json::json!({ "length_m": 3.0 }));
    }

    #[test]
    fn a_shape_that_mixes_or_misses_a_size_is_refused() {
        for yaml in ["{}", "{ length_m: 3, width_m: 1 }", "{ width_m: 1 }"] {
            assert!(read(yaml).is_err(), "{yaml}");
        }
    }

    #[test]
    fn a_size_at_or_below_zero_is_refused_by_name_and_place() {
        let refused = read("{ length_m: 0 }").expect_err("zero is no length");
        assert!(refused.contains("geometry: `length_m` is 0"), "{refused}");
        assert!(refused.contains(" at line 1 column "), "{refused}");
        let refused = read("{ width_m: 0.3, height_m: .nan }").expect_err("NaN is no height");
        assert!(refused.contains("`height_m`"), "{refused}");
    }

    #[test]
    fn a_field_the_block_does_not_know_is_refused() {
        assert!(read("{ length_m: 3, depth_m: 1 }").is_err());
    }
}
