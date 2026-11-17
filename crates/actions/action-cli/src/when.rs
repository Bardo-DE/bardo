//! Dates, instants, durations and decimals as a terminal writes them.
//!
//! A date or an instant is RFC 3339 (`2026-10-03`, `2026-10-03T09:30:00+02:00`). The launcher's
//! natural-date parser ("tomorrow at nine") is the shell's and cannot be reached from here; that is an
//! interface ask, and until it lands a natural date is a usage error that says so.

use action_vocab::{CivilDate, Decimal, Scale, Seconds};
use provenance::UnixSeconds;

/// Why a word is not the value wanted.
#[derive(Debug, Clone, thiserror::Error)]
pub enum WhenError {
    /// Not RFC 3339.
    #[error(
        "expected RFC 3339 (2026-10-03 or 2026-10-03T09:30:00Z); natural dates are not understood here"
    )]
    NotRfc3339(#[source] action_vocab::when::NotRfc3339Error),
    /// Not a duration.
    #[error("expected seconds (90) or units (1h30m, 2d, 45s)")]
    NotDuration,
    /// A number in a duration is too large.
    #[error("a number in the duration is too large")]
    DurationNumber(#[source] std::num::ParseIntError),
    /// The duration does not fit in the seconds a duration holds.
    #[error("the duration is too long")]
    DurationLong(#[source] std::num::TryFromIntError),
    /// Not a decimal with this many digits.
    #[error("expected a decimal number with at most {0} digits after the point")]
    NotDecimal(u8),
    /// The digits of a decimal are too many for the number it holds.
    #[error("the decimal number is too large")]
    DecimalLarge(#[source] std::num::ParseIntError),
}

fn rfc3339<T>(read: Result<T, action_vocab::when::NotRfc3339Error>) -> Result<T, WhenError> {
    read.map_err(WhenError::NotRfc3339)
}

/// A calendar date: `2026-10-03`, or the date part of an RFC 3339 instant as written.
///
/// # Errors
///
/// Returns [`WhenError`] when the text is not a value of this kind.
pub fn date(text: &str) -> Result<CivilDate, WhenError> {
    rfc3339(action_vocab::when::date(text))
}

/// An instant: RFC 3339, or a bare date taken as midnight UTC.
///
/// # Errors
///
/// Returns [`WhenError`] when the text is not a value of this kind.
pub fn instant(text: &str) -> Result<UnixSeconds, WhenError> {
    rfc3339(action_vocab::when::instant(text))
}

/// A length of time: whole seconds, or `d`, `h`, `m` and `s` units (`1h30m`).
///
/// # Errors
///
/// Returns [`WhenError`] when the text is not a value of this kind.
pub fn duration(text: &str) -> Result<Seconds, WhenError> {
    let mut total: u64 = 0;
    let mut number = String::new();
    let mut saw_unit = false;
    for c in text.chars() {
        if c.is_ascii_digit() {
            number.push(c);
            continue;
        }
        let unit = match c {
            'd' => 86_400,
            'h' => 3_600,
            'm' => 60,
            's' => 1,
            _ => return Err(WhenError::NotDuration),
        };
        // A unit with no number before it (`soon`, `h`) is not a duration; digits that do not
        // fit are one that is too large.
        if number.is_empty() {
            return Err(WhenError::NotDuration);
        }
        let n: u64 = number.parse().map_err(WhenError::DurationNumber)?;
        total = total.saturating_add(n.saturating_mul(unit));
        number.clear();
        saw_unit = true;
    }
    if !number.is_empty() {
        if saw_unit {
            return Err(WhenError::NotDuration);
        }
        total = number.parse().map_err(WhenError::DurationNumber)?;
    }
    if text.is_empty() {
        return Err(WhenError::NotDuration);
    }
    u32::try_from(total)
        .map(Seconds)
        .map_err(WhenError::DurationLong)
}

/// A fixed-point number with exactly `scale` digits after the point (missing ones are zeros).
///
/// # Errors
///
/// Returns [`WhenError`] when the text is not a value of this kind.
pub fn decimal(text: &str, scale: Scale) -> Result<Decimal, WhenError> {
    let fault = WhenError::NotDecimal(scale.0);
    let (negative, body) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (whole, frac) = body.split_once('.').unwrap_or((body, ""));
    let plain = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    if whole.is_empty() || !plain(whole) || !plain(frac) || frac.len() > usize::from(scale.0) {
        return Err(fault);
    }
    let padded = format!("{whole}{frac:0<width$}", width = usize::from(scale.0));
    let units: i64 = padded.parse().map_err(WhenError::DecimalLarge)?;
    Ok(Decimal {
        units: if negative { -units } else { units },
        scale,
    })
}
