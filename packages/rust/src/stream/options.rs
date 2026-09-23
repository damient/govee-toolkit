//! How a stream is opened, and the names a person writes for it.
//!
//! [`Resolution`] and [`Rate`] read back from the same text they print.

use std::fmt;
use std::str::FromStr;

use thiserror::Error;

/// A text that names no [`Resolution`] and no [`Rate`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("`{value}` is not {subject}; write {expected}")]
pub struct ParseError {
    /// The text that was read.
    pub value: String,
    /// What the text must name.
    pub subject: &'static str,
    /// The forms that are accepted.
    pub expected: &'static str,
}

/// How many zones one paint or one stream states.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Resolution {
    /// What the Govee app exposes, from `capabilities.segments.count`.
    #[default]
    App,
    /// Every addressable LED, from `capabilities.segments.native_pixels`.
    /// Fails when nobody measured it, and on a mode that paints by zone mask.
    Native,
    /// One zone per group, from `capabilities.segments.groups`. A mode that
    /// paints by mask names the groups as its zones. A mode that states every
    /// zone in one frame carries `native_pixels`, and the stream paints each
    /// group over its own run of LEDs. Fails where the file declares no
    /// groups, or where such a mode reaches no measured LED count.
    Groups,
    /// A count the caller picks. A count the unit renders as a smaller one
    /// fails with
    /// [`Error::ResolutionNotDistinct`](crate::Error::ResolutionNotDistinct)
    /// where the file records `measurements.resolution_changepoints`.
    Exact(u16),
}

impl fmt::Display for Resolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::App => f.write_str("app"),
            Self::Native => f.write_str("native"),
            Self::Groups => f.write_str("groups"),
            Self::Exact(zones) => write!(f, "{zones}"),
        }
    }
}

impl FromStr for Resolution {
    type Err = ParseError;

    /// # Errors
    ///
    /// [`ParseError`] where the text is not `app`, `native`, `groups`, or a
    /// zone count from 0 to 65535.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "app" => Ok(Self::App),
            "native" => Ok(Self::Native),
            "groups" => Ok(Self::Groups),
            other => other
                .parse::<u16>()
                .map(Self::Exact)
                .map_err(|_| ParseError {
                    value: other.to_owned(),
                    subject: "a resolution",
                    expected: "`app`, `native`, `groups`, or a zone count",
                }),
        }
    }
}

/// How fast frames go out.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Rate {
    /// From the device file's `measurements.frame_rate` for the mode the
    /// stream opens on, falling back to
    /// [`FALLBACK_HZ`](crate::stream::FALLBACK_HZ) when it records none
    /// there. A rate measured over one mode is never carried to another.
    #[default]
    Measured,
    /// A rate the caller picks, in hertz.
    Fixed(f64),
}

impl fmt::Display for Rate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Measured => f.write_str("measured"),
            Self::Fixed(hz) => write!(f, "{hz}"),
        }
    }
}

impl FromStr for Rate {
    type Err = ParseError;

    /// # Errors
    ///
    /// [`ParseError`] where the text is neither `measured` nor a number. The
    /// range stays [`SegmentStream::open`](crate::stream::SegmentStream) to
    /// refuse: a rate of zero or less fails at the open.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "measured" => Ok(Self::Measured),
            other => other
                .parse::<f64>()
                .map(Self::Fixed)
                .map_err(|_| ParseError {
                    value: other.to_owned(),
                    subject: "a rate",
                    expected: "`measured`, or a rate in hertz",
                }),
        }
    }
}

/// How a stream is opened.
#[derive(Debug, Clone, Default)]
pub struct StreamOptions {
    /// How many zones to carry.
    pub resolution: Resolution,
    /// How fast to send.
    pub rate: Rate,
    /// Ask the firmware to interpolate between zones, and to wrap from the
    /// last zone back to the first. `true` is refused where the device file
    /// can carry the setting nowhere, rather than dropped.
    pub gradient: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_resolution_reads_by_name_or_by_count() {
        assert_eq!("app".parse(), Ok(Resolution::App));
        assert_eq!("native".parse(), Ok(Resolution::Native));
        assert_eq!("groups".parse(), Ok(Resolution::Groups));
        assert_eq!("30".parse(), Ok(Resolution::Exact(30)));
        assert!("many".parse::<Resolution>().is_err());
    }

    #[test]
    fn a_rate_reads_by_name_or_by_hertz() {
        assert_eq!("measured".parse(), Ok(Rate::Measured));
        assert_eq!("24.5".parse(), Ok(Rate::Fixed(24.5)));
        assert!("fast".parse::<Rate>().is_err());
    }

    #[test]
    fn what_a_name_prints_reads_back() {
        for resolution in [
            Resolution::App,
            Resolution::Native,
            Resolution::Groups,
            Resolution::Exact(30),
        ] {
            assert_eq!(resolution.to_string().parse(), Ok(resolution));
        }
        for rate in [Rate::Measured, Rate::Fixed(24.5)] {
            assert_eq!(rate.to_string().parse(), Ok(rate));
        }
    }

    #[test]
    fn a_refusal_names_what_to_write() {
        let failure = "many".parse::<Resolution>().err().map(|e| e.to_string());
        assert_eq!(
            failure.as_deref(),
            Some("`many` is not a resolution; write `app`, `native`, `groups`, or a zone count")
        );
    }
}
