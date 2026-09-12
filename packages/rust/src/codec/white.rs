//! The RGB rendering of a white temperature, for the modes whose frame carries
//! it beside the kelvin value — `docs/protocol/ble.md` 2.3.
//!
//! The curve approximates the Planckian locus, sampled every 500 K from Tanner
//! Helland's formula and interpolated between samples. It is not the vendor's
//! rendering: nobody captured what the Govee app sends for a temperature.

/// Ordered by kelvin, and never empty: [`rgb`] reads the ends of it.
const CURVE: [(i64, [u8; 3]); 15] = [
    (2000, [255, 137, 14]),
    (2500, [255, 159, 70]),
    (3000, [255, 177, 110]),
    (3500, [255, 193, 141]),
    (4000, [255, 206, 166]),
    (4500, [255, 218, 187]),
    (5000, [255, 228, 206]),
    (5500, [255, 237, 222]),
    (6000, [255, 246, 237]),
    (6500, [255, 254, 250]),
    (7000, [243, 242, 255]),
    (7500, [230, 235, 255]),
    (8000, [221, 230, 255]),
    (8500, [215, 226, 255]),
    (9000, [210, 223, 255]),
];

/// What `kelvin` looks like in RGB. A temperature outside the curve gets the
/// nearest end of it: the accepted range is the device file's, checked where
/// the argument is encoded, so this answers for every input.
#[must_use]
pub fn rgb(kelvin: i64) -> [u8; 3] {
    let mut below = CURVE[0];
    for sample in CURVE {
        if sample.0 >= kelvin {
            return blend(below, sample, kelvin);
        }
        below = sample;
    }
    below.1
}

fn blend(low: (i64, [u8; 3]), high: (i64, [u8; 3]), kelvin: i64) -> [u8; 3] {
    let span = high.0 - low.0;
    if span <= 0 {
        return low.1;
    }
    let into = kelvin - low.0;
    let mut out = low.1;
    for (channel, (from, to)) in out.iter_mut().zip(low.1.into_iter().zip(high.1)) {
        let step = (i64::from(to) - i64::from(from)) * into / span;
        *channel = u8::try_from(i64::from(from) + step).unwrap_or(*channel);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sampled_temperature_is_the_sample() {
        assert_eq!(rgb(4000), [255, 206, 166]);
        assert_eq!(rgb(2000), [255, 137, 14]);
        assert_eq!(rgb(9000), [210, 223, 255]);
    }

    #[test]
    fn a_temperature_between_samples_lands_between_them() {
        let out = rgb(4250);
        assert_eq!(out[0], 255);
        assert!((206..=218).contains(&out[1]), "green {}", out[1]);
        assert!((166..=187).contains(&out[2]), "blue {}", out[2]);
    }

    #[test]
    fn a_temperature_outside_the_curve_takes_the_nearest_end() {
        assert_eq!(rgb(0), rgb(2000));
        assert_eq!(rgb(i64::MAX), rgb(9000));
    }

    #[test]
    fn a_warmer_temperature_carries_less_blue() {
        for kelvin in (2000..6500).step_by(250) {
            assert!(
                rgb(kelvin)[2] <= rgb(kelvin + 250)[2],
                "blue fell at {kelvin} K"
            );
        }
    }
}
