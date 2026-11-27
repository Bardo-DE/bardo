//! `Money`: exact sums of one currency, typed refusals, and a stable JSON form.

use model_types::{Currency, Money, MoneyError, TurnUsage};

#[test]
fn amounts_of_one_currency_add_exactly() {
    let a = Money::usd(1, 1);
    let b = Money::usd(2, 1);
    assert_eq!(a.checked_add(b), Ok(Money::usd(3, 1)));
    assert_eq!(b.checked_sub(a), Ok(a));
}

#[test]
fn different_currencies_and_overflow_are_refused() {
    let usd = Money::usd(1, 0);
    let eur = Money::new(1, 0, Currency::EUR);
    assert_eq!(usd.checked_add(eur), Err(MoneyError::CurrencyMismatch));
    assert_eq!(usd.partial_cmp(&eur), None);
    let top = Money::max(Currency::USD);
    assert_eq!(top.checked_add(usd), Err(MoneyError::Overflow));
}

#[test]
fn the_scale_does_not_change_the_amount() {
    assert_eq!(Money::usd(950, 6), Money::usd(95, 5));
    assert!(Money::usd(1, 3) < Money::usd(2, 3));
}

#[test]
fn digits_are_read_and_written_without_trailing_zeros() {
    let read = |text: &str| Money::parse(text, Currency::USD);
    assert_eq!(read("0.00095"), Some(Money::usd(95, 5)));
    assert_eq!(read("9.5e-4"), Some(Money::usd(95, 5)));
    assert_eq!(read("12"), Some(Money::usd(12, 0)));
    assert_eq!(read("-1.5"), Some(Money::usd(-15, 1)));
    assert_eq!(read(""), None);
    assert_eq!(read("1.2.3"), None);
    assert_eq!(read("abc"), None);
    assert_eq!(Money::usd(9500, 6).to_string(), "0.0095 USD");
    assert_eq!(Money::usd(0, 6).to_string(), "0 USD");
    assert_eq!(Money::new(3, 0, Currency::EUR).to_string(), "3 EUR");
}

#[test]
fn digits_finer_than_the_eighteenth_round_away_from_zero() {
    let read = |text: &str| Money::parse(text, Currency::USD).unwrap();
    assert_eq!(read("0.0000000000000000001"), Money::usd(1, 18));
    assert_eq!(read("-0.0000000000000000001"), Money::usd(-1, 18));
    assert_eq!(Money::usd(1, 25), Money::usd(1, 18));
    assert_eq!(read("0.000001234567"), Money::usd(1_234_567, 12));
    assert_eq!(read("1.4e-7"), Money::usd(14, 8));
}

#[test]
fn scaling_and_rounding_up() {
    assert_eq!(Money::usd(5, 3).scaled(1000, 4), Some(Money::usd(125, 2)));
    assert_eq!(Money::usd(5, 3).scaled(1, 0), None);
    assert_eq!(Money::usd(1, 3).round_up_to(2), Some(1));
    assert_eq!(Money::usd(250, 2).round_up_to(2), Some(250));
    assert_eq!(Money::usd(1, 0).round_up_to(19), None);
}

#[test]
fn money_is_written_as_digits_and_a_code() {
    let json = serde_json::to_string(&Money::usd(95, 5)).unwrap();
    assert_eq!(json, r#"{"amount":"0.00095","currency":"USD"}"#);
    for text in [
        r#"{"amount":"0.00095","currency":"USD"}"#,
        r#"{"amount":0.00095,"currency":"USD"}"#,
        r#"{"amount":"0.000950","currency":"USD"}"#,
    ] {
        assert_eq!(
            serde_json::from_str::<Money>(text).unwrap(),
            Money::usd(95, 5)
        );
    }
    serde_json::from_str::<Money>(r#"{"amount":"1","currency":"usd"}"#).unwrap_err();
}

#[test]
fn a_usage_without_a_charge_writes_no_charge_field() {
    let json = serde_json::to_string(&TurnUsage::default()).unwrap();
    assert!(!json.contains("charge"), "{json}");
    let priced = TurnUsage {
        charge: Some(Money::usd(95, 5)),
        ..TurnUsage::default()
    };
    let back: TurnUsage = serde_json::from_str(&serde_json::to_string(&priced).unwrap()).unwrap();
    assert_eq!(back, priced);
}
