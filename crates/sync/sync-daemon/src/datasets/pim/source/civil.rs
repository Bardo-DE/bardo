//! Calendar dates and local times as the sources write them, read by jiff and written the way
//! iCalendar wants them (`20261006`, `20261006T100000`). Pure.

use jiff::civil::{Date, DateTime};

/// `20261006` from `2026-10-06` (anything after the date, such as a time, is ignored). A day the
/// calendar does not have (the 31st of April, the 29th of February in a common year) is `None`.
pub(super) fn compact_date(text: &str) -> Option<String> {
    let date: Date = text.get(..10)?.parse().ok()?;
    (date.year() >= 1).then(|| format!("{:04}{:02}{:02}", date.year(), date.month(), date.day()))
}

/// The wall clock the first 19 bytes of `text` write, `2026-10-06T10:00:00` (the `T` may be `t` or
/// a space). A leap second counts as the last second of its minute. Whatever follows is not read.
/// A date or a time that does not exist, or one before year 1, is `None`.
pub(super) fn wall_clock(text: &str) -> Option<DateTime> {
    let b = text.as_bytes().get(..19)?;
    let shaped = b[4] == b'-'
        && b[7] == b'-'
        && matches!(b[10], b'T' | b't' | b' ')
        && b[13] == b':'
        && b[16] == b':';
    if !shaped {
        return None;
    }
    let number = |from: usize, to: usize| -> Option<i16> {
        b[from..to].iter().try_fold(0_i16, |n, d| {
            d.is_ascii_digit().then(|| n * 10 + i16::from(d - b'0'))
        })
    };
    let small = |from: usize| i8::try_from(number(from, from + 2)?).ok();
    let second = small(17)?;
    if second > 60 {
        return None;
    }
    let year = number(0, 4)?;
    let at = DateTime::new(
        year,
        small(5)?,
        small(8)?,
        small(11)?,
        small(14)?,
        second.min(59),
        0,
    )
    .ok()?;
    (year >= 1).then_some(at)
}

/// `20261006T100000` from `2026-10-06T10:00:00.0000000` (a fraction, a `Z` or an offset after
/// the seconds is ignored). A date or a time that does not exist is `None`.
pub(super) fn compact_local(text: &str) -> Option<String> {
    wall_clock(text).map(|at| written(&at))
}

/// `20261006T100000` of `at`.
pub(super) fn written(at: &DateTime) -> String {
    format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}",
        at.year(),
        at.month(),
        at.day(),
        at.hour(),
        at.minute(),
        at.second()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_date_is_compacted_and_one_the_calendar_lacks_is_refused() {
        assert_eq!(compact_date("2026-10-06").as_deref(), Some("20261006"));
        assert_eq!(compact_date("2024-02-29").as_deref(), Some("20240229"));
        assert_eq!(
            compact_date("2026-10-06T00:00:00.000Z").as_deref(),
            Some("20261006")
        );
        for text in [
            "",
            "2026-1-06",
            "2026/10/06",
            "2026-00-06",
            "2026-10-00",
            "2026-02-29",
            "2026-02-31",
            "2026-04-31",
            "2026-06-31",
            "2026-09-31",
            "2026-11-31",
            "0000-01-01",
            "abcd-ef-gh",
        ] {
            assert_eq!(compact_date(text), None, "{text:?}");
        }
    }

    #[test]
    fn a_local_time_is_compacted_and_a_bad_one_refused() {
        assert_eq!(
            compact_local("2026-10-06T10:00:00.0000000").as_deref(),
            Some("20261006T100000")
        );
        assert_eq!(
            compact_local("2026-10-05T12:30:00Z").as_deref(),
            Some("20261005T123000")
        );
        for text in [
            "",
            "2026-10-06",
            "2026-99-99T99:99:99",
            "2026-10-06T24:00:00",
            "2026-10-06T10:60:00",
            "2026-02-30T10:00:00",
            "2026-10-06T10:00",
        ] {
            assert_eq!(compact_local(text), None, "{text:?}");
        }
    }
}
