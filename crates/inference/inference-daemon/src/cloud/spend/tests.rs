use super::*;
use account_core::{AccountId, AppId};
use account_core::{AppName, Isolation};
use base_ids::{Tokens, TurnCount};
use base_testkit::Scratch;
use base_time::UnixSeconds;
use inference_core::{ChargeTotal, Period, SpendScope, SpendVerdict, TokenUsage};
use inference_settings::{Ledger, Spent};
use inference_settings::{ScopeLimits, SpendLimits, SpendLine, cents_to_limit};
use model_provider::Money;
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

fn used(input: u32, output: u32) -> TokenUsage {
    TokenUsage {
        input: Tokens(input),
        output: Tokens(output),
        cached: Tokens(0),
        charge: None,
    }
}

/// A reply of `input` and `output` tokens that its provider said cost `milli` thousandths of a
/// US dollar.
fn charged(input: u32, output: u32, milli: i64) -> TokenUsage {
    TokenUsage {
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

fn line(app_daily: u32) -> SpendLine {
    SpendLine {
        limits: SpendLimits {
            account: ScopeLimits::default(),
            app: ScopeLimits {
                daily: cents_to_limit(app_daily),
                monthly: None,
            },
        },
        ..SpendLine::default()
    }
}

#[test]
fn a_ledger_opened_on_a_file_keeps_what_it_was_told_across_a_restart() {
    use std::os::unix::fs::PermissionsExt;
    let dir = Scratch::new("inferd-spend");
    let file = dir.join("state").join("spend.json");
    let who = app("org.bardo.Companion");
    {
        let ledger = open_at(file.clone(), LEAP_NOON);
        ledger.record(
            &who,
            &account("openrouter"),
            charged(7, 3, 1_234),
            LEAP_NOON,
        );
        ledger.record(&who, &account("openrouter"), used(1, 1), LEAP_NOON);
    };
    let again = open_at(file.clone(), LEAP_NOON);
    let scope = SpendScope::App(who);
    assert_eq!(
        again.spent(&scope, Period::Daily, LEAP_NOON),
        Spent {
            charges: ChargeTotal {
                unpriced: TurnCount(1),
                ..milli(1_234)
            },
            tokens_in: 8,
            tokens_out: 4
        }
    );
    let text = std::fs::read_to_string(&file).expect("the file");
    assert!(!text.contains("sk-"), "money and counts only: {text}");
    let mode = std::fs::metadata(&file).expect("meta").permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
}

fn scratch(name: &str) -> Scratch {
    Scratch::new(&format!("inferd-spend-{name}"))
}

/// rel-7: before, a file that did not read started the ledger empty, so every cap saw nothing
/// spent and the next request went through, however much had been spent. Now the file is moved
/// aside, and a cap of the day and the month stops until they end.
#[test]
fn a_file_that_does_not_read_is_set_aside_and_the_caps_stop_until_the_period_ends() {
    let dir = scratch("bad");
    let file = dir.join("spend.json");
    std::fs::write(&file, "not json").expect("write");
    let who = app("org.bardo.Companion");
    let through = account("openrouter");
    let ledger = open_at(file.clone(), LEAP_NOON);
    // The unreadable file is kept, under a name that says when.
    let aside = dir.join(format!("spend.json.bad-{}", LEAP_NOON.0));
    assert_eq!(std::fs::read_to_string(&aside).expect("kept"), "not json");
    // Nothing is known to be spent, and a cap stops all the same.
    let scope = SpendScope::App(who.clone());
    assert_eq!(
        ledger.spent(&scope, Period::Daily, LEAP_NOON),
        Spent::default()
    );
    let stopped = |ledger: &Ledger, at: UnixSeconds| ledger.verdict(line(10), &who, &through, at);
    assert_eq!(stopped(&ledger, LEAP_NOON), SpendVerdict::Stop);
    // With no cap set there is nothing to hold to.
    assert_eq!(
        ledger.verdict(SpendLine::default(), &who, &through, LEAP_NOON),
        SpendVerdict::Within
    );
    // The next day the daily cap counts again from nothing (the lost day is over).
    let tomorrow = UnixSeconds(LEAP_NOON.0 + 86_400);
    assert_eq!(stopped(&ledger, tomorrow), SpendVerdict::Within);
    // Records go on, and the stop is in the file: a restart in the same day does not forget it.
    ledger.record(&who, &through, charged(1, 1, 5), LEAP_NOON);
    let again = open_at(file.clone(), LEAP_NOON);
    assert_eq!(stopped(&again, LEAP_NOON), SpendVerdict::Stop);
    assert_eq!(
        again.spent(&scope, Period::Daily, LEAP_NOON).charges,
        milli(5),
        "what was recorded since is kept"
    );
    // A restart after the day is over has nothing to remember for the daily cap.
    let later = open_at(file, tomorrow);
    assert_eq!(stopped(&later, tomorrow), SpendVerdict::Within);
}

#[test]
fn a_monthly_cap_stays_stopped_to_the_end_of_the_month_and_a_missing_file_is_just_new() {
    let dir = scratch("monthly");
    let file = dir.join("spend.json");
    let who = app("org.bardo.Companion");
    let through = account("openrouter");
    // No file yet: a new ledger, nothing set aside, caps work as usual.
    let fresh = open_at(file.clone(), LEAP_NOON);
    assert_eq!(
        fresh.verdict(line(10), &who, &through, LEAP_NOON),
        SpendVerdict::Within
    );
    assert!(!dir.join(format!("spend.json.bad-{}", LEAP_NOON.0)).exists());
    // A directory where the file should be cannot be read either: lost, not new.
    let blocked_root = scratch("monthly-dir");
    let blocked = blocked_root.join("spend.json");
    std::fs::create_dir_all(&blocked).expect("dir in the way");
    let monthly = SpendLine {
        limits: SpendLimits {
            account: ScopeLimits::default(),
            app: ScopeLimits {
                daily: None,
                monthly: crate::settings::cents_to_limit(10),
            },
        },
        ..SpendLine::default()
    };
    // Lost on the 19th of February 2024; the 20th is the same month; 1 March is the next.
    let mid_february = UnixSeconds(LEAP_NOON.0 - 10 * 86_400);
    let lost = open_at(blocked, mid_february);
    for (name, at, want) in [
        ("the day", mid_february, SpendVerdict::Stop),
        (
            "the next day, same month",
            UnixSeconds(mid_february.0 + 86_400),
            SpendVerdict::Stop,
        ),
        (
            "the month after",
            UnixSeconds(LEAP_NOON.0 + 86_400),
            SpendVerdict::Within,
        ),
    ] {
        assert_eq!(lost.verdict(monthly, &who, &through, at), want, "{name}");
    }
}
