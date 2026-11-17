//! RFC 3339 UTC timestamps from `std::time`, with no date-time dependency.

use std::time::{SystemTime, UNIX_EPOCH};

const DAY: i64 = 86_400;

/// Whole seconds since the Unix epoch (negative before it) and the nanoseconds past them.
fn since_epoch(t: SystemTime) -> (i64, u32) {
    let whole = |secs: u64| i64::try_from(secs).unwrap_or(i64::MAX);
    match t.duration_since(UNIX_EPOCH) {
        Ok(d) => (whole(d.as_secs()), d.subsec_nanos()),
        Err(e) => {
            let d = e.duration();
            match d.subsec_nanos() {
                0 => (-whole(d.as_secs()), 0),
                n => (-whole(d.as_secs()) - 1, 1_000_000_000 - n),
            }
        }
    }
}

/// The proleptic Gregorian year, month and day of a day count from 1970-01-01.
fn civil(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let (era, doe) = (z.div_euclid(146_097), z.rem_euclid(146_097));
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// `t` as `YYYY-MM-DDTHH:MM:SS[.fff[fff[fff]]]+00:00`: the fraction is omitted when zero, else 3, 6 or 9 digits.
pub(crate) fn format(t: SystemTime) -> String {
    let (secs, nanos) = since_epoch(t);
    let (year, month, day) = civil(secs.div_euclid(DAY));
    let of_day = secs.rem_euclid(DAY);
    let (h, m, s) = (of_day / 3_600, of_day % 3_600 / 60, of_day % 60);
    let fraction = match nanos {
        0 => String::new(),
        n if n % 1_000_000 == 0 => format!(".{:03}", n / 1_000_000),
        n if n % 1_000 == 0 => format!(".{:06}", n / 1_000),
        n => format!(".{n:09}"),
    };
    format!("{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}{fraction}+00:00")
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn at(secs: u64, nanos: u32) -> String {
        format(UNIX_EPOCH + Duration::new(secs, nanos))
    }

    #[test]
    fn whole_seconds_have_no_fraction() {
        assert_eq!(at(0, 0), "1970-01-01T00:00:00+00:00");
        assert_eq!(at(1_736_694_058, 0), "2025-01-12T15:00:58+00:00");
    }

    #[test]
    fn the_fraction_takes_three_six_or_nine_digits() {
        assert_eq!(at(1, 500_000_000), "1970-01-01T00:00:01.500+00:00");
        assert_eq!(at(1, 1_000), "1970-01-01T00:00:01.000001+00:00");
        assert_eq!(at(1, 1), "1970-01-01T00:00:01.000000001+00:00");
    }

    #[test]
    fn leap_days_and_year_ends_fall_on_the_right_date() {
        assert_eq!(at(951_782_400, 0), "2000-02-29T00:00:00+00:00");
        assert_eq!(at(1_709_251_199, 0), "2024-02-29T23:59:59+00:00");
        assert_eq!(at(4_102_444_799, 0), "2099-12-31T23:59:59+00:00");
        assert_eq!(at(4_107_542_400, 0), "2100-03-01T00:00:00+00:00");
    }

    #[test]
    fn instants_before_the_epoch_count_backwards() {
        let before = |d: Duration| format(UNIX_EPOCH - d);
        assert_eq!(before(Duration::from_secs(1)), "1969-12-31T23:59:59+00:00");
        assert_eq!(
            before(Duration::from_millis(500)),
            "1969-12-31T23:59:59.500+00:00"
        );
        assert_eq!(
            before(Duration::from_secs(86_400)),
            "1969-12-31T00:00:00+00:00"
        );
    }
}
