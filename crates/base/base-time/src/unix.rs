//! An instant as seconds since the Unix epoch.

use serde::{Deserialize, Serialize};

/// Seconds since the Unix epoch, UTC. Instants on the wire and in stores; the clock that makes them
/// is injected ([`crate::Clock`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UnixSeconds(pub i64);

impl UnixSeconds {
    /// The instant an RFC 3339 time names (`2026-10-06T10:00:00.123-07:00`, `…Z`), as the
    /// storage APIs write a file's last change. A fraction is dropped and a leap second counts as
    /// the second before it; `None` for anything else.
    #[must_use]
    pub fn from_rfc3339(text: &str) -> Option<Self> {
        if !rfc3339_shape(text) {
            return None;
        }
        let time: jiff::Timestamp = text.parse().ok()?;
        Some(UnixSeconds(time.as_second()))
    }

    /// `YYYY-MM-DD` of the UTC day this instant falls in; `None` for an instant outside the
    /// years jiff can write.
    #[must_use]
    pub fn utc_date(self) -> Option<String> {
        let day = jiff::Timestamp::from_second(self.0)
            .ok()?
            .to_zoned(jiff::tz::TimeZone::UTC)
            .date();
        Some(format!(
            "{:04}-{:02}-{:02}",
            day.year(),
            day.month(),
            day.day()
        ))
    }
}

/// Whether `text` has the exact shape of an RFC 3339 instant (`T`, `t` or a space between the
/// date and the time, a fraction of one digit or more, `Z` or a `±hh:mm` offset). The calendar is
/// jiff's to check; jiff alone would also take an offset without its colon or one of 24 hours.
fn rfc3339_shape(text: &str) -> bool {
    let b = text.as_bytes();
    let digits = |at: usize| {
        b.get(at..at + 2)
            .is_some_and(|d| d.iter().all(u8::is_ascii_digit))
    };
    let number = |at: usize| (b[at] - b'0') * 10 + (b[at + 1] - b'0');
    let fixed = b.len() >= 20
        && b[..4].iter().all(u8::is_ascii_digit)
        && b[4] == b'-'
        && digits(5)
        && b[7] == b'-'
        && digits(8)
        && matches!(b[10], b'T' | b't' | b' ')
        && digits(11)
        && b[13] == b':'
        && digits(14)
        && b[16] == b':'
        && digits(17);
    if !fixed || number(11) > 23 || number(14) > 59 || number(17) > 60 {
        return false;
    }
    let mut rest = &b[19..];
    if let [b'.', tail @ ..] = rest {
        let n = tail.iter().take_while(|c| c.is_ascii_digit()).count();
        if n == 0 {
            return false;
        }
        rest = &tail[n..];
    }
    match rest {
        [b'Z' | b'z'] => true,
        [b'+' | b'-', ..] if rest.len() == 6 => {
            let off = |at: usize| {
                rest.get(at..at + 2)
                    .is_some_and(|d| d.iter().all(u8::is_ascii_digit))
            };
            off(1)
                && rest[3] == b':'
                && off(4)
                && (rest[1] - b'0') * 10 + (rest[2] - b'0') <= 23
                && (rest[4] - b'0') * 10 + (rest[5] - b'0') <= 59
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_rfc3339_time_is_read_as_seconds_since_the_epoch() {
        const CASES: &[(&str, Option<i64>)] = &[
            ("1970-01-01T00:00:00Z", Some(0)),
            ("2026-10-24T05:42:18Z", Some(1_792_820_538)),
            ("2026-10-24T05:42:18.123Z", Some(1_792_820_538)),
            ("2026-10-24T13:42:18+08:00", Some(1_792_820_538)),
            ("2026-10-23T22:42:18-07:00", Some(1_792_820_538)),
            ("2000-02-29T00:00:00Z", Some(951_782_400)),
            ("1969-12-31T23:59:59Z", Some(-1)),
            ("2026-10-24", None),
            ("2026-10-24T05:42:18", None),
            ("2026-13-01T00:00:00Z", None),
            // RFC 3339 section 5.6 allows a space for the T; jiff reads it, as do the APIs' own clients.
            ("2026-10-24 05:42:18Z", Some(1_792_820_538)),
            ("2026-10-24t05:42:18z", Some(1_792_820_538)),
            ("2026-02-31T00:00:00Z", None),
            ("2026-04-31T00:00:00Z", None),
            ("2026-10-24T24:00:00Z", None),
            ("2026-10-24T05:42:18+24:00", None),
            ("2026-10-24T05:42:18.Z", None),
            ("not a time", None),
        ];
        for (text, want) in CASES {
            assert_eq!(
                UnixSeconds::from_rfc3339(text),
                want.map(UnixSeconds),
                "{text}"
            );
        }
    }

    #[test]
    fn the_utc_day_of_a_unix_time() {
        let cases = [
            (0, "1970-01-01"),
            (86_399, "1970-01-01"),
            (86_400, "1970-01-02"),
            (951_782_400, "2000-02-29"),
            (1_790_000_000, "2026-09-21"),
            (-86_400, "1969-12-31"),
        ];
        for (at, want) in cases {
            assert_eq!(UnixSeconds(at).utc_date().as_deref(), Some(want), "{at}");
        }
        assert_eq!(UnixSeconds(i64::MAX).utc_date(), None);
    }
}
