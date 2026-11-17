use super::*;
use crate::{ScopeLimits, SpendLimits};
use account_core::{AppName, Isolation};
use base_ids::{Tokens, TurnCount};
use rust_decimal::Decimal;

fn app(name: &str) -> AppId {
    AppId {
        name: AppName::parse(name).expect("name"),
        isolation: Isolation::Unsandboxed,
    }
}

fn account(id: &str) -> AccountId {
    AccountId::parse(id).expect("id")
}

fn used(input: u32, output: u32) -> TurnUsage {
    TurnUsage {
        input: Tokens(input),
        output: Tokens(output),
        cached: Tokens(0),
        charge: None,
        ..TurnUsage::default()
    }
}

/// A reply of `input` and `output` tokens that its provider said cost `milli` thousandths of a
/// US dollar.
fn charged(input: u32, output: u32, milli: i64) -> TurnUsage {
    TurnUsage {
        charge: Some(Money::usd(Decimal::new(milli, 3))),
        ..used(input, output)
    }
}

fn milli(milli: i64) -> ChargeTotal {
    ChargeTotal {
        reported: Money::usd(Decimal::new(milli, 3)),
        unpriced: TurnCount(0),
    }
}

/// 2024-02-29 12:00:00 UTC.
const LEAP_NOON: UnixSeconds = UnixSeconds(1_709_208_000);

#[test]
fn days_and_months_are_calendar_ones_in_utc() {
    let cases = [
        ("the epoch", 0, 0, 1970 * 12),
        ("a second before it", -1, -1, 1969 * 12 + 11),
        ("a leap day", 1_709_208_000, 19_782, 2024 * 12 + 1),
        (
            "the first of March 2000",
            951_868_800,
            11_017,
            2000 * 12 + 2,
        ),
        (
            "the last second of 2024",
            1_735_689_599,
            20_088,
            2024 * 12 + 11,
        ),
        ("the first second of 2025", 1_735_689_600, 20_089, 2025 * 12),
    ];
    for (name, at, day, month) in cases {
        assert_eq!(day_of(UnixSeconds(at)), day, "{name}: day");
        assert_eq!(month_of(UnixSeconds(at)), month, "{name}: month");
    }
}

fn line(app_daily: u32) -> SpendLine {
    SpendLine {
        limits: SpendLimits {
            account: ScopeLimits::default(),
            app: ScopeLimits {
                daily: crate::cents_to_limit(app_daily),
                monthly: None,
            },
        },
        ..SpendLine::default()
    }
}

#[test]
fn a_reply_counts_against_the_app_and_the_account_for_the_day_and_the_month() {
    let ledger = Ledger::in_memory();
    let (companion, reader) = (app("org.bardo.Companion"), app("org.bardo.Reader"));
    ledger.record(
        &companion,
        &account("openrouter"),
        charged(1_000, 200, 700),
        LEAP_NOON,
    );
    ledger.record(
        &reader,
        &account("openrouter"),
        charged(10, 5, 30),
        LEAP_NOON,
    );
    // A reply whose provider reported no charge is a turn, and no money.
    ledger.record(&reader, &account("openrouter"), used(1, 1), LEAP_NOON);
    let spent = |scope: SpendScope, period| ledger.spent(&scope, period, LEAP_NOON);
    assert_eq!(
        spent(SpendScope::App(companion), Period::Daily),
        Spent {
            charges: milli(700),
            tokens_in: 1_000,
            tokens_out: 200
        }
    );
    assert_eq!(
        spent(SpendScope::App(reader), Period::Monthly).charges,
        ChargeTotal {
            unpriced: TurnCount(1),
            ..milli(30)
        }
    );
    let through = SpendScope::Account(account("openrouter"));
    assert_eq!(
        spent(through.clone(), Period::Daily).charges,
        ChargeTotal {
            unpriced: TurnCount(1),
            ..milli(730)
        }
    );
    assert_eq!(spent(through, Period::Monthly).tokens_in, 1_011);
    assert_eq!(
        spent(SpendScope::Account(account("openai")), Period::Daily),
        Spent::default()
    );
}

#[test]
fn a_new_day_starts_the_day_over_and_the_month_goes_on() {
    let ledger = Ledger::in_memory();
    let who = app("org.bardo.Companion");
    ledger.record(&who, &account("openrouter"), charged(1, 1, 500), LEAP_NOON);
    let next_day = UnixSeconds(LEAP_NOON.0 + 86_400);
    let scope = SpendScope::App(who.clone());
    let reported = |period, at| ledger.spent(&scope, period, at).charges;
    assert_eq!(reported(Period::Daily, next_day), ChargeTotal::default());
    assert_eq!(
        reported(Period::Monthly, next_day),
        ChargeTotal::default(),
        "that was March"
    );
    let same_month = UnixSeconds(LEAP_NOON.0 - 86_400);
    assert_eq!(reported(Period::Monthly, same_month), milli(500));
    ledger.record(&who, &account("openrouter"), charged(1, 1, 5), next_day);
    assert_eq!(reported(Period::Daily, next_day), milli(5));
}

#[test]
fn the_caps_say_within_warn_or_stop_and_no_cap_says_within() {
    let who = app("org.bardo.Companion");
    let through = account("openrouter");
    // 10 cents a day is 100 thousandths of a dollar; the warning line is at 80 per cent.
    let cases = [
        ("nothing spent", 0, SpendVerdict::Within),
        ("under the line", 79, SpendVerdict::Within),
        ("on the line", 80, SpendVerdict::Warn),
        ("under the cap", 99, SpendVerdict::Warn),
        ("at the cap", 100, SpendVerdict::Stop),
    ];
    for (name, spent, want) in cases {
        let ledger = Ledger::in_memory();
        if spent > 0 {
            ledger.record(&who, &through, charged(0, 0, spent), LEAP_NOON);
        }
        let got = ledger.verdict(line(10), &who, &through, LEAP_NOON);
        assert_eq!(got, want, "{name}");
    }
    let ledger = Ledger::in_memory();
    ledger.record(&who, &through, charged(0, 0, 1_000_000), LEAP_NOON);
    assert_eq!(
        ledger.verdict(SpendLine::default(), &who, &through, LEAP_NOON),
        SpendVerdict::Within,
        "no cap, no limit"
    );
}

/// A cap compares what providers reported: turns that reported nothing are counted and never meet
/// it, and the turn that passes it is the last one, not the one it stops.
#[test]
fn only_reported_charges_meet_a_cap_and_the_turn_that_passes_it_is_the_last() {
    let ledger = Ledger::in_memory();
    let who = app("org.bardo.Companion");
    let through = account("openai");
    for _ in 0..1_000 {
        ledger.record(&who, &through, used(500, 500), LEAP_NOON);
    }
    assert_eq!(
        ledger.verdict(line(1), &who, &through, LEAP_NOON),
        SpendVerdict::Within,
        "a thousand turns with no charge are no money"
    );
    let scope = SpendScope::App(who.clone());
    assert_eq!(
        ledger
            .spent(&scope, Period::Daily, LEAP_NOON)
            .charges
            .unpriced,
        TurnCount(1_000)
    );
    // The reply that carries the charge is counted whole, and the next turn is stopped.
    ledger.record(&who, &through, charged(10, 10, 10), LEAP_NOON);
    assert_eq!(
        ledger.verdict(line(1), &who, &through, LEAP_NOON),
        SpendVerdict::Stop
    );
}

#[test]
fn the_strictest_cap_of_the_four_decides() {
    let who = app("org.bardo.Companion");
    let through = account("openrouter");
    let ledger = Ledger::in_memory();
    ledger.record(&who, &through, charged(0, 0, 150), LEAP_NOON);
    let line = SpendLine {
        limits: SpendLimits {
            account: ScopeLimits {
                daily: None,
                monthly: crate::cents_to_limit(1_000),
            },
            app: ScopeLimits {
                daily: crate::cents_to_limit(10),
                monthly: None,
            },
        },
        ..SpendLine::default()
    };
    assert_eq!(
        ledger.verdict(line, &who, &through, LEAP_NOON),
        SpendVerdict::Stop,
        "the app's daily cap is passed though the account's monthly one is not"
    );
    let other = app("org.bardo.Reader");
    assert_eq!(
        ledger.verdict(line, &other, &through, LEAP_NOON),
        SpendVerdict::Within,
        "another app has spent nothing and the account's cap is far off"
    );
}

#[test]
fn usage_names_what_the_app_used_and_each_cap_that_is_set() {
    let ledger = Ledger::in_memory();
    let who = app("org.bardo.Companion");
    ledger.record(&who, &account("openrouter"), charged(40, 9, 321), LEAP_NOON);
    ledger.record(&who, &account("openai"), used(1, 1), LEAP_NOON);
    let rows = ledger.usage_of(line(10), &who, LEAP_NOON);
    let money = |milli: i64| UsageValue::Money(Money::usd(Decimal::new(milli, 3)));
    let want = [
        ("tokens_in_day", UsageValue::Count(41)),
        ("tokens_out_day", UsageValue::Count(10)),
        ("reported_day", money(321)),
        ("reported_month", money(321)),
        ("unpriced_turns_day", UsageValue::Count(1)),
        ("unpriced_turns_month", UsageValue::Count(1)),
        ("cap_day", money(100)),
    ];
    let got: Vec<(&str, UsageValue)> = rows.iter().map(|(n, v)| (n.as_str(), *v)).collect();
    assert_eq!(got, want);
    assert_eq!(written(Money::usd(Decimal::new(9500, 6))), "0.0095 USD");
    assert_eq!(written(Money::usd(Decimal::ZERO)), "0 USD");
}
