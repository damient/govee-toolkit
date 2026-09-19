//! The instant a scan ran.
//!
//! UTC, because the file states one instant and a reader must not have to know
//! which offset wrote it. No date library: one format and one algorithm are
//! cheaper here than a dependency.

use std::time::{SystemTime, UNIX_EPOCH};

/// Now, as RFC 3339 in UTC: `2026-09-19T20:04:11Z`.
///
/// A clock before the epoch answers the epoch.
#[must_use]
pub fn now() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    stamp(seconds)
}

/// The instant `seconds` after the epoch, as RFC 3339 in UTC.
fn stamp(seconds: u64) -> String {
    let day = i64::try_from(seconds / 86_400).unwrap_or_default();
    let rest = seconds % 86_400;
    let (year, month, date) = civil(day);
    let (hour, minute, second) = (rest / 3600, (rest % 3600) / 60, rest % 60);
    format!("{year:04}-{month:02}-{date:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// The civil date `day` days after 1970-01-01.
///
/// Howard Hinnant's `civil_from_days`, which every date library carries. It
/// holds for every year a proleptic Gregorian calendar covers.
fn civil(day: i64) -> (i64, u32, u32) {
    let shifted = day + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let date = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = if month <= 2 { year + 1 } else { year };
    (
        year,
        u32::try_from(month).unwrap_or_default(),
        u32::try_from(date).unwrap_or_default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_epoch_and_a_leap_day_read_back() {
        assert_eq!(stamp(0), "1970-01-01T00:00:00Z");
        assert_eq!(stamp(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(stamp(1_789_846_992), "2026-09-19T19:43:12Z");
    }

    #[test]
    fn the_time_of_day_carries_every_field() {
        assert_eq!(stamp(86_399), "1970-01-01T23:59:59Z");
    }
}
