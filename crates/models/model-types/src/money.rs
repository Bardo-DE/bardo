//! An amount of money a provider reported, and which providers report one.
//!
//! An amount is a whole number of 10^-18 of the currency's main unit: exact for any price a
//! provider writes in decimal digits, with room for about 10^20 main units, `Copy`, `Eq`, and with
//! no third-party number type in the public API. Text and JSON write it as decimal digits, so a
//! stored amount reads the same as it always did.

use core::cmp::Ordering;
use core::fmt;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Digits after the point an amount keeps.
const DIGITS: u32 = 18;

/// Parts in one main unit of a currency.
const PARTS: u128 = 10_u128.pow(DIGITS);

/// A currency, by its three-letter ISO 4217 code (`USD`). The code is checked for shape only:
/// three capital ASCII letters. Nothing is converted between currencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Currency([u8; 3]);

impl Currency {
    /// US dollars.
    pub const USD: Currency = Currency(*b"USD");

    /// Euros.
    pub const EUR: Currency = Currency(*b"EUR");

    /// The currency written `code`, when `code` is three capital ASCII letters.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Currency> {
        let bytes: [u8; 3] = code.as_bytes().try_into().ok()?;
        bytes
            .iter()
            .all(u8::is_ascii_uppercase)
            .then_some(Currency(bytes))
    }

    /// The three-letter code.
    #[must_use]
    pub fn code(&self) -> &str {
        // The bytes were checked to be ASCII letters when the value was made.
        core::str::from_utf8(&self.0).unwrap_or("???")
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl Serialize for Currency {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.code())
    }
}

impl<'de> Deserialize<'de> for Currency {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let code = String::deserialize(deserializer)?;
        Currency::from_code(&code).ok_or_else(|| {
            de::Error::invalid_value(de::Unexpected::Str(&code), &"an ISO 4217 code")
        })
    }
}

/// An exact amount in one currency. It carries only what a provider said: nothing here is
/// computed from a price table.
///
/// Amounts of different currencies are never added or ordered. Building an amount from digits
/// finer than the eighteenth rounds away from zero, so a charge is never undercounted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Money {
    parts: i128,
    currency: Currency,
}

/// Why two amounts could not be added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MoneyError {
    /// The amounts are in different currencies.
    #[error("the amounts are in different currencies")]
    CurrencyMismatch,
    /// The result does not fit.
    #[error("the sum is too large to hold")]
    Overflow,
}

impl Money {
    /// `mantissa` shifted `scale` digits behind the point (`Money::new(95, 5, c)` is 0.00095), in
    /// `currency`. A scale beyond eighteen digits rounds away from zero.
    #[must_use]
    pub fn new(mantissa: i64, scale: u32, currency: Currency) -> Self {
        let mantissa = i128::from(mantissa);
        let parts = match scale.checked_sub(DIGITS) {
            None => mantissa * 10_i128.pow(DIGITS - scale),
            Some(extra) => shift_down(mantissa, extra),
        };
        Self { parts, currency }
    }

    /// An amount in US dollars (`OpenRouter`'s credits are dollars); see [`Money::new`].
    #[must_use]
    pub fn usd(mantissa: i64, scale: u32) -> Self {
        Self::new(mantissa, scale, Currency::USD)
    }

    /// Nothing, in `currency`.
    #[must_use]
    pub const fn zero(currency: Currency) -> Self {
        Self { parts: 0, currency }
    }

    /// The largest amount there is, in `currency`: a cap that never binds.
    #[must_use]
    pub const fn max(currency: Currency) -> Self {
        Self {
            parts: i128::MAX,
            currency,
        }
    }

    /// The amount written in decimal digits (`0.0042`, `12`, `1.5e-3`), in `currency`. `None` when
    /// the text is not a number or the amount does not fit.
    #[must_use]
    pub fn parse(text: &str, currency: Currency) -> Option<Self> {
        Some(Self {
            parts: parse_parts(text)?,
            currency,
        })
    }

    /// The currency.
    #[must_use]
    pub fn currency(&self) -> Currency {
        self.currency
    }

    /// Whether the amount is nothing.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.parts == 0
    }

    /// Whether the amount is below nothing.
    #[must_use]
    pub fn is_negative(&self) -> bool {
        self.parts < 0
    }

    /// The sum of two amounts of the same currency; never a panic.
    ///
    /// # Errors
    ///
    /// Returns [`MoneyError::CurrencyMismatch`] when the currencies differ and
    /// [`MoneyError::Overflow`] when the sum does not fit.
    pub fn checked_add(self, other: Money) -> Result<Money, MoneyError> {
        self.combine(other, i128::checked_add)
    }

    /// The difference of two amounts of the same currency; never a panic. It may be negative.
    ///
    /// # Errors
    ///
    /// Returns [`MoneyError::CurrencyMismatch`] when the currencies differ and
    /// [`MoneyError::Overflow`] when the difference does not fit.
    pub fn checked_sub(self, other: Money) -> Result<Money, MoneyError> {
        self.combine(other, i128::checked_sub)
    }

    fn combine(
        self,
        other: Money,
        op: fn(i128, i128) -> Option<i128>,
    ) -> Result<Money, MoneyError> {
        if self.currency != other.currency {
            return Err(MoneyError::CurrencyMismatch);
        }
        op(self.parts, other.parts)
            .map(|parts| Money { parts, ..self })
            .ok_or(MoneyError::Overflow)
    }

    /// `numerator / denominator` of the amount, rounded toward zero at the eighteenth digit. `None`
    /// when the denominator is zero or the product does not fit.
    #[must_use]
    pub fn scaled(self, numerator: u32, denominator: u32) -> Option<Money> {
        let parts = self
            .parts
            .checked_mul(i128::from(numerator))?
            .checked_div(i128::from(denominator))?;
        Some(Money { parts, ..self })
    }

    /// The amount rounded to `scale` digits behind the point, halves away from zero. A scale
    /// beyond eighteen digits leaves it as it is.
    #[must_use]
    pub fn round(self, scale: u32) -> Money {
        let Some(extra) = DIGITS.checked_sub(scale).filter(|extra| *extra > 0) else {
            return self;
        };
        let unit = 10_i128.pow(extra);
        let rest = self.parts % unit;
        let floor = self.parts - rest;
        let away = if rest.abs() * 2 >= unit {
            unit * self.parts.signum()
        } else {
            0
        };
        Money {
            parts: floor.saturating_add(away),
            ..self
        }
    }

    /// The amount in units of `10^-scale` of the main unit (`2` for cents), rounded up so a part
    /// of a unit counts as one. `None` for a scale beyond eighteen digits.
    #[must_use]
    pub fn round_up_to(&self, scale: u32) -> Option<i128> {
        let unit = 10_i128.pow(DIGITS.checked_sub(scale)?);
        let whole = self.parts / unit;
        Some(if self.parts % unit > 0 {
            whole + 1
        } else {
            whole
        })
    }
}

/// `value / 10^digits`, rounded away from zero.
fn shift_down(value: i128, digits: u32) -> i128 {
    let Some(unit) = 10_i128.checked_pow(digits) else {
        return i128::from(value != 0) * value.signum();
    };
    let whole = value / unit;
    match value % unit {
        0 => whole,
        _ => whole + value.signum(),
    }
}

/// The parts of the amount written `text`: an optional sign, digits with an optional point, and
/// an optional exponent.
fn parse_parts(text: &str) -> Option<i128> {
    let (negative, rest) = match text.as_bytes().first()? {
        b'-' => (true, &text[1..]),
        b'+' => (false, &text[1..]),
        _ => (false, text),
    };
    let (number, exponent) = match rest.find(['e', 'E']) {
        Some(at) => (&rest[..at], rest[at + 1..].parse::<i32>().ok()?),
        None => (rest, 0),
    };
    let (whole, fraction) = number.split_once('.').unwrap_or((number, ""));
    if whole.is_empty() && fraction.is_empty() {
        return None;
    }
    let mut digits: i128 = 0;
    for byte in whole.bytes().chain(fraction.bytes()) {
        if !byte.is_ascii_digit() {
            return None;
        }
        digits = digits
            .checked_mul(10)?
            .checked_add(i128::from(byte - b'0'))?;
    }
    // digits * 10^(exponent - fraction digits) main units, so that many digits plus eighteen in parts.
    let fraction_digits = i32::try_from(fraction.len()).ok()?;
    let shift = i32::try_from(DIGITS)
        .ok()?
        .checked_add(exponent)?
        .checked_sub(fraction_digits)?;
    let magnitude = match u32::try_from(shift) {
        Ok(up) => digits.checked_mul(10_i128.checked_pow(up)?)?,
        Err(_) => shift_down(digits, shift.unsigned_abs()),
    };
    Some(if negative { -magnitude } else { magnitude })
}

impl PartialOrd for Money {
    /// Amounts of one currency order by size; amounts of different currencies do not order.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        (self.currency == other.currency).then(|| self.parts.cmp(&other.parts))
    }
}

impl fmt::Display for Money {
    /// The amount's decimal digits without trailing zeros, a space and the code: `0.0095 USD`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", digits_of(self.parts), self.currency)
    }
}

/// `parts` as decimal digits of the main unit, without trailing zeros.
fn digits_of(parts: i128) -> String {
    let sign = if parts < 0 { "-" } else { "" };
    let magnitude = parts.unsigned_abs();
    let (whole, fraction) = (magnitude / PARTS, magnitude % PARTS);
    if fraction == 0 {
        return format!("{sign}{whole}");
    }
    let fraction = format!("{fraction:018}");
    format!("{sign}{whole}.{}", fraction.trim_end_matches('0'))
}

/// How an amount is written: its digits as text, and its code.
#[derive(Serialize, Deserialize)]
struct Written {
    amount: Digits,
    currency: Currency,
}

/// An amount's digits; read from text or from a plain number.
struct Digits(i128);

impl Serialize for Digits {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&digits_of(self.0))
    }
}

impl<'de> Deserialize<'de> for Digits {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Reader;
        impl Visitor<'_> for Reader {
            type Value = Digits;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an amount in decimal digits")
            }

            fn visit_str<E: de::Error>(self, text: &str) -> Result<Digits, E> {
                parse_parts(text)
                    .map(Digits)
                    .ok_or_else(|| E::invalid_value(de::Unexpected::Str(text), &self))
            }

            fn visit_i64<E: de::Error>(self, number: i64) -> Result<Digits, E> {
                self.visit_str(&number.to_string())
            }

            fn visit_u64<E: de::Error>(self, number: u64) -> Result<Digits, E> {
                self.visit_str(&number.to_string())
            }

            fn visit_f64<E: de::Error>(self, number: f64) -> Result<Digits, E> {
                self.visit_str(&number.to_string())
            }
        }
        deserializer.deserialize_any(Reader)
    }
}

impl Serialize for Money {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        Written {
            amount: Digits(self.parts),
            currency: self.currency,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Money {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let Written { amount, currency } = Written::deserialize(deserializer)?;
        Ok(Money {
            parts: amount.0,
            currency,
        })
    }
}

/// Whether a provider says what each reply cost. Only what the provider reports is ever shown;
/// there is no price table to fall back on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Charges {
    /// Every reply carries its charge (`TurnUsage::charge`), though one may still arrive without.
    PerReply,
    /// The provider reports tokens only (or nothing); `TurnUsage::charge` is always `None`.
    NotReported,
}
