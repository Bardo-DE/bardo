//! RFC 3339 dates and instants: `2026-10-03`, `2026-10-03T09:30:00+02:00`.
//!
//! Pure; the terminal (`bardo-do`) and the quarantined reader (a `Date` or `DateTime` answer) both
//! read them, and the edges that print a UTC time or day ask here too. The calendar is jiff's;
//! this file only fixes which texts are accepted.

use crate::value::CivilDate;
use jiff::Timestamp;
use jiff::civil::Date;
use provenance::UnixSeconds;

/// The text is not an RFC 3339 date or instant.
#[derive(Debug, Clone, thiserror::Error)]
pub enum NotRfc3339Error {
    /// The text is not shaped like a date or instant.
    #[error("not an RFC 3339 date or instant")]
    Shape,
    /// The date part is not a day on the calendar.
    #[error("not an RFC 3339 date or instant")]
    Day(#[source] jiff::Error),
    /// The text is not an RFC 3339 instant.
    #[error("not an RFC 3339 date or instant")]
    Instant,
    /// A part of the date does not fit the number the date holds.
    #[error("a part of the date is out of range")]
    Range(#[source] std::num::TryFromIntError),
}

/// The date part, `YYYY-MM-DD` exactly: jiff's own reader would also take a sign or a short
/// month.
fn day(head: &str) -> Result<Date, NotRfc3339Error> {
    let shaped = head.bytes().enumerate().all(|(at, b)| match at {
        4 | 7 => b == b'-',
        _ => b.is_ascii_digit(),
    });
    if !shaped {
        return Err(NotRfc3339Error::Shape);
    }
    head.parse().map_err(NotRfc3339Error::Day)
}

/// The text read as a date alone or as a full instant (the separator may be `T`, `t` or a space,
/// the offset `Z`, `z` or numeric).
enum Read {
    Day(Date),
    /// The date as written, and the instant: whole seconds, a leap second already counted.
    Instant(Date, UnixSeconds),
}

fn read(text: &str) -> Result<Read, NotRfc3339Error> {
    let head = text.get(..10).ok_or(NotRfc3339Error::Shape)?;
    let date = day(head)?;
    if text.len() == 10 {
        return Ok(Read::Day(date));
    }
    let UnixSeconds(seconds) = UnixSeconds::from_rfc3339(text).ok_or(NotRfc3339Error::Instant)?;
    // The reader has checked the shape, so the seconds are the two bytes at 17; jiff clamps `:60`
    // to `:59`.
    let leap = i64::from(text.as_bytes().get(17..19) == Some(b"60"));
    Ok(Read::Instant(date, UnixSeconds(seconds + leap)))
}

/// A calendar date: `2026-10-03`, or the date part of an RFC 3339 instant as written.
///
/// # Errors
///
/// Returns `NotRfc3339Error` when `text` is not an RFC 3339 date.
pub fn date(text: &str) -> Result<CivilDate, NotRfc3339Error> {
    let (Read::Day(date) | Read::Instant(date, _)) = read(text)?;
    Ok(CivilDate {
        year: date.year(),
        month: u8::try_from(date.month()).map_err(NotRfc3339Error::Range)?,
        day: u8::try_from(date.day()).map_err(NotRfc3339Error::Range)?,
    })
}

/// An instant: RFC 3339, or a bare date taken as midnight UTC. A leap second (`:60`) counts as
/// the second after `:59`.
///
/// # Errors
///
/// Returns `NotRfc3339Error` when `text` is neither an RFC 3339 instant nor a bare date.
pub fn instant(text: &str) -> Result<UnixSeconds, NotRfc3339Error> {
    Ok(match read(text)? {
        Read::Day(date) => UnixSeconds(
            date.at(0, 0, 0, 0)
                .to_zoned(jiff::tz::TimeZone::UTC)
                .map_err(NotRfc3339Error::Day)?
                .timestamp()
                .as_second(),
        ),
        Read::Instant(_, at) => at,
    })
}

/// `at` as an RFC 3339 UTC time, `2026-10-03T09:30:00Z`; none when it is beyond the calendar.
#[must_use]
pub fn utc_instant(at: UnixSeconds) -> Option<String> {
    Timestamp::from_second(at.0).ok().map(|t| t.to_string())
}
