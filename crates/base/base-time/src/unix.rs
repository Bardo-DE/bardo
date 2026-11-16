//! An instant as seconds since the Unix epoch.

use serde::{Deserialize, Serialize};

/// Seconds since the Unix epoch, UTC. Instants on the wire and in stores; the clock that makes them
/// is injected ([`crate::Clock`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UnixSeconds(pub i64);

impl UnixSeconds {
    /// The instant an RFC 3339 time names (`2026-10-06T10:00:00.123-07:00`, `…Z`), as the
    /// storage APIs write a file's last change. A fraction is dropped; `None` for anything else.
    #[must_use]
    pub fn from_rfc3339(text: &str) -> Option<Self> {
        let time = chrono::DateTime::parse_from_rfc3339(text).ok()?;
        Some(UnixSeconds(time.timestamp()))
    }

    /// `YYYY-MM-DD` of the UTC day this instant falls in; `None` for an instant outside the
    /// years chrono can write.
    #[must_use]
    pub fn utc_date(self) -> Option<String> {
        use chrono::Datelike;
        let day = chrono::DateTime::from_timestamp(self.0, 0)?.date_naive();
        Some(format!(
            "{:04}-{:02}-{:02}",
            day.year(),
            day.month(),
            day.day()
        ))
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
            // RFC 3339 section 5.6 allows a space for the T; chrono reads it, as do the APIs' own clients.
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
