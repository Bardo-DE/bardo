//! Dates and instants as the stores write them: a proleptic Gregorian date, and a moment in
//! seconds since the Unix epoch, always in UTC. jiff does the calendar arithmetic.

use jiff::civil;
use jiff::tz::Offset;
use jiff::{Span, Timestamp};
use std::fmt;

/// A calendar date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    pub year: i32,
    /// 1 to 12.
    pub month: u8,
    /// 1 to 31.
    pub day: u8,
}

/// A moment, in seconds since 1970-01-01T00:00:00Z.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant(pub i64);

impl Date {
    /// The date as jiff's, or `None` for one jiff cannot name.
    fn exact(year: i32, month: u8, day: u8) -> Option<civil::Date> {
        let year = i16::try_from(year).ok()?;
        let month = i8::try_from(month).ok()?;
        let day = i8::try_from(day).ok()?;
        civil::Date::new(year, month, day).ok()
    }

    /// The date as jiff's. A `Date` built by hand with a day past its month's end counts on
    /// from the first of the month, as the arithmetic it replaces did; a month outside 1 to 12
    /// is the first day jiff can name.
    fn civil(self) -> civil::Date {
        Date::exact(self.year, self.month, self.day)
            .or_else(|| {
                let first = Date::exact(self.year, self.month, 1)?;
                let later = Span::new().days(i64::from(self.day.saturating_sub(1)));
                first.checked_add(later).ok()
            })
            .unwrap_or(civil::Date::MIN)
    }

    fn of(date: civil::Date) -> Self {
        Date {
            year: i32::from(date.year()),
            month: u8::try_from(date.month()).unwrap_or(1),
            day: u8::try_from(date.day()).unwrap_or(1),
        }
    }
}

impl Date {
    /// The date written `YYYY-MM-DD`, or `None` for anything else or a day the month lacks.
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.trim().splitn(3, '-');
        let year = parts.next()?.parse().ok()?;
        let month: u8 = parts.next()?.parse().ok()?;
        let day: u8 = parts.next()?.parse().ok()?;
        Date::exact(year, month, day).map(Date::of)
    }

    /// The day after.
    #[must_use]
    pub fn next(self) -> Self {
        Date::of(self.civil().tomorrow().unwrap_or(civil::Date::MAX))
    }

    /// How many days from `self` to `other`: 1 for the next day.
    #[must_use]
    pub fn days_until(self, other: Date) -> i64 {
        self.civil()
            .until(other.civil())
            .map_or(0, |span| i64::from(span.get_days()))
    }

    /// Midnight UTC at the start of the day.
    #[must_use]
    pub fn midnight(self) -> Instant {
        let at = Offset::UTC
            .to_timestamp(self.civil().to_datetime(civil::Time::midnight()))
            .unwrap_or(Timestamp::MIN);
        Instant(at.as_second())
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl Instant {
    /// The date and time of day in UTC, written `YYYY-MM-DDTHH:MM:SS` with no zone: what Graph
    /// takes beside `"timeZone": "UTC"` and JMAP beside `Etc/UTC`.
    #[must_use]
    pub fn local_utc(self) -> String {
        let at = Offset::UTC.to_datetime(Timestamp::from_second(self.0).unwrap_or(Timestamp::MIN));
        format!(
            "{}T{:02}:{:02}:{:02}",
            Date::of(at.date()),
            at.hour(),
            at.minute(),
            at.second()
        )
    }

    /// RFC 3339 in UTC: `YYYY-MM-DDTHH:MM:SSZ`.
    #[must_use]
    pub fn rfc3339(self) -> String {
        format!("{}Z", self.local_utc())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_are_read_written_and_counted() {
        let cases = [
            ("1970-01-01", Some((1970, 1, 1))),
            ("2024-02-29", Some((2024, 2, 29))),
            ("2023-02-29", None),
            ("2026-13-01", None),
            ("26-1-1", Some((26, 1, 1))),
            ("tomorrow", None),
        ];
        for (text, want) in cases {
            let got = Date::parse(text).map(|d| (d.year, d.month, d.day));
            assert_eq!(got, want, "{text}");
        }
        let leap = Date::parse("2024-02-28").unwrap();
        assert_eq!(leap.next().to_string(), "2024-02-29");
        assert_eq!(leap.next().next().to_string(), "2024-03-01");
        assert_eq!(
            Date::parse("2026-12-31").unwrap().next().to_string(),
            "2027-01-01"
        );
        assert_eq!(leap.days_until(Date::parse("2024-03-01").unwrap()), 2);
    }

    #[test]
    fn instants_are_written_in_utc() {
        assert_eq!(Instant(0).rfc3339(), "1970-01-01T00:00:00Z");
        assert_eq!(Instant(1_793_000_000).rfc3339(), "2026-10-26T07:33:20Z");
        assert_eq!(Instant(-1).local_utc(), "1969-12-31T23:59:59");
        assert_eq!(
            Date::parse("2026-10-26").unwrap().midnight().rfc3339(),
            "2026-10-26T00:00:00Z"
        );
    }
}
