//! What hosted models have been charged, per scope and period.
//!
//! What hosted models have been charged: a ledger of the charges providers reported and the tokens
//! used, per scope (an account, an app) and period (a calendar day, a calendar month, in UTC), and
//! the verdict the caps (`ai.spend.*`) give one more request.
//!
//! The meter adds what each reply's usage reports: its tokens, and its charge when the provider
//! reported one (`TokenUsage::charge`). A reply with no charge is counted as an unpriced turn and
//! never as money: there is no price table to work one out from and no hold before a turn, so a
//! cap compares the reported total and can stop the next turn but never the current one. The
//! ledger keeps only the current day and month, in memory and, when it was opened on a file, in
//! that file (a small JSON document, written whole through a rename, mode 0600) so a restart does
//! not reset a cap. The file holds money and counts, never a key or a prompt.

use crate::settings::SpendLine;
use account_core::AccountId;
use account_core::AppId;
use base_fs::atomic::AtomicWrite;
use base_fs::{ReadCap, ReadError, move_aside, read_text_capped};
use base_time::UnixSeconds;
use inference_core::{ChargeTotal, Period, SpendScope, SpendVerdict, TokenUsage, spend_verdict};
use model_provider::Money;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

/// What one scope was charged in one period.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spent {
    /// The charges providers reported, and the turns that reported none.
    pub charges: ChargeTotal,
    /// Prompt tokens.
    pub tokens_in: u64,
    /// Reply tokens.
    pub tokens_out: u64,
}

impl Spent {
    /// Counts one reply.
    fn count(&mut self, usage: TokenUsage) {
        self.charges = self.charges.counting(usage.charge);
        self.tokens_in = self.tokens_in.saturating_add(u64::from(usage.input.0));
        self.tokens_out = self.tokens_out.saturating_add(u64::from(usage.output.0));
    }
}

/// The ledger file's format: version 2 holds the charges providers reported.
const FORMAT: u32 = 2;

/// The days since 1970-01-01 of `at` (UTC).
fn day_of(at: UnixSeconds) -> i64 {
    at.0.div_euclid(86_400)
}

/// The month `at` is in, as months since year 0 (UTC): `year * 12 + month - 1`. An instant chrono
/// cannot write (beyond year 262000) is month 0.
fn month_of(at: UnixSeconds) -> i64 {
    use chrono::Datelike;
    chrono::DateTime::from_timestamp(at.0, 0).map_or(0, |day| {
        i64::from(day.year()) * 12 + i64::from(day.month()) - 1
    })
}

/// The index of the period `at` falls in.
fn bucket_of(period: Period, at: UnixSeconds) -> i64 {
    match period {
        Period::Daily => day_of(at),
        Period::Monthly => month_of(at),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct Key {
    scope: SpendScope,
    period: Period,
    bucket: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Row {
    #[serde(flatten)]
    key: Key,
    spent: Spent,
}

#[derive(Debug, Serialize, Deserialize)]
struct Document {
    format: u32,
    rows: Vec<Row>,
    /// Set when the ledger's file could not be read: the day and month it was found in, whose
    /// spend is not known (kept in the file so a restart does not forget it).
    #[serde(default)]
    unknown: Option<Unknown>,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            format: FORMAT,
            rows: Vec::new(),
            unknown: None,
        }
    }
}

/// What a ledger file holds.
fn read_document(text: &str) -> Result<Document, ()> {
    let document: Document = serde_json::from_str(text).ok().ok_or(())?;
    // A format a later inferd wrote: not guessed at.
    if document.format == FORMAT {
        Ok(document)
    } else {
        Err(())
    }
}

/// The day and the month in which the ledger lost what had been spent. A cap of that period
/// stops everything until the period ends, since what was spent cannot be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct Unknown {
    day: i64,
    month: i64,
}

impl Unknown {
    fn at(now: UnixSeconds) -> Self {
        Self {
            day: day_of(now),
            month: month_of(now),
        }
    }

    /// Whether the period `now` is in is the one that was lost.
    fn covers(self, period: Period, now: UnixSeconds) -> bool {
        match period {
            Period::Daily => self.day == day_of(now),
            Period::Monthly => self.month == month_of(now),
        }
    }
}

/// Every scope's spend this day and month.
#[derive(Debug, Default)]
pub struct Ledger {
    rows: Mutex<HashMap<Key, Spent>>,
    file: Option<PathBuf>,
    unknown: Option<Unknown>,
}

impl Ledger {
    /// A ledger that remembers nothing across a restart.
    #[must_use]
    pub fn in_memory() -> Self {
        Self::default()
    }

    /// A ledger kept in `file`: what it holds is read now and every record is written back. A
    /// file that is missing starts it empty. A file that is there and cannot be read is not
    /// quietly forgotten (that would reset every cap to nothing spent): it is moved aside as
    /// `<name>.bad-<seconds>`, and the day and month `now` is in count as unknown, so a cap
    /// of either stops requests until that period ends.
    pub fn open_at(file: PathBuf, now: UnixSeconds) -> Self {
        let read = match read_text_capped(&file, ReadCap::mib(8)) {
            Ok(text) => read_document(&text),
            Err(ReadError::NotFound { .. }) => Ok(Document::default()),
            Err(_) => Err(()),
        };
        let (document, lost) = if let Ok(document) = read {
            (document, false)
        } else {
            set_aside(&file, now);
            (Document::default(), true)
        };
        // A loss from an earlier run counts while its period lasts.
        let unknown = if lost {
            Some(Unknown::at(now))
        } else {
            document.unknown.filter(|u| {
                [Period::Daily, Period::Monthly]
                    .into_iter()
                    .any(|period| u.covers(period, now))
            })
        };
        let rows = document
            .rows
            .into_iter()
            .map(|r| (r.key, r.spent))
            .collect();
        let ledger = Self {
            rows: Mutex::new(rows),
            file: Some(file),
            unknown,
        };
        if lost {
            // Keep the loss at once: a restart before the first record must not forget it.
            ledger.save(&ledger.rows.lock().unwrap_or_else(PoisonError::into_inner));
        }
        ledger
    }

    /// Whether what was spent in the period `now` is in is lost.
    fn is_unknown(&self, period: Period, now: UnixSeconds) -> bool {
        self.unknown.is_some_and(|u| u.covers(period, now))
    }

    /// What `scope` has spent in the period `now` is in.
    pub fn spent(&self, scope: &SpendScope, period: Period, now: UnixSeconds) -> Spent {
        let key = Key {
            scope: scope.clone(),
            period,
            bucket: bucket_of(period, now),
        };
        self.rows
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&key)
            .copied()
            .unwrap_or_default()
    }

    /// Counts one reply: its tokens and its charge (`usage.charge`, or an unpriced turn when it
    /// has none) against the app and the account, in the day and the month `now` is in; periods
    /// that have ended are forgotten.
    pub fn record(&self, app: &AppId, account: &AccountId, usage: TokenUsage, now: UnixSeconds) {
        let mut rows = self.rows.lock().unwrap_or_else(PoisonError::into_inner);
        rows.retain(|key, _| key.bucket >= bucket_of(key.period, now));
        let scopes = [
            SpendScope::App(app.clone()),
            SpendScope::Account(account.clone()),
        ];
        for scope in scopes {
            for period in [Period::Daily, Period::Monthly] {
                rows.entry(Key {
                    scope: scope.clone(),
                    period,
                    bucket: bucket_of(period, now),
                })
                .or_default()
                .count(usage);
            }
        }
        self.save(&rows);
    }

    fn save(&self, rows: &HashMap<Key, Spent>) {
        let Some(file) = &self.file else { return };
        let document = Document {
            format: FORMAT,
            rows: rows
                .iter()
                .map(|(key, spent)| Row {
                    key: key.clone(),
                    spent: *spent,
                })
                .collect(),
            unknown: self.unknown,
        };
        // On disk before it replaces the old file, or a power cut leaves an empty ledger.
        let write = || -> std::io::Result<()> {
            let text = serde_json::to_string(&document).map_err(std::io::Error::other)?;
            AtomicWrite::PRIVATE.write(file, text.as_bytes())
        };
        if let Err(why) = write() {
            eprintln!("inferd: spend: {}: {why}", file.display());
        }
    }

    /// What the caps say about one more request by `app` through `account`: the strictest of the
    /// caps that bear on it (`Within` when none is set). Each compares what providers have
    /// reported so far, so it can stop the next request and never the one running.
    pub fn verdict(
        &self,
        line: SpendLine,
        app: &AppId,
        account: &AccountId,
        now: UnixSeconds,
    ) -> SpendVerdict {
        let rows = self.rows.lock().unwrap_or_else(PoisonError::into_inner);
        line.caps_for(app, account)
            .iter()
            .map(|cap| {
                if self.is_unknown(cap.period, now) {
                    SpendVerdict::Stop
                } else {
                    let key = Key {
                        scope: cap.scope.clone(),
                        period: cap.period,
                        bucket: bucket_of(cap.period, now),
                    };
                    let spent = rows.get(&key).copied().unwrap_or_default().charges;
                    spend_verdict(cap, &spent)
                }
            })
            .fold(SpendVerdict::Within, strictest)
    }

    /// What `app` used today and this month, by name (the `Usage` dictionary of `Inference1`):
    /// tokens for the day, the charges providers reported and the turns that reported none for the
    /// day and the month, and each cap that is set.
    pub fn usage_of(
        &self,
        line: SpendLine,
        app: &AppId,
        now: UnixSeconds,
    ) -> Vec<(String, UsageValue)> {
        let scope = SpendScope::App(app.clone());
        let day = self.spent(&scope, Period::Daily, now);
        let month = self.spent(&scope, Period::Monthly, now);
        let mut rows = vec![
            ("tokens_in_day", UsageValue::Count(day.tokens_in)),
            ("tokens_out_day", UsageValue::Count(day.tokens_out)),
            ("reported_day", UsageValue::Money(day.charges.reported)),
            ("reported_month", UsageValue::Money(month.charges.reported)),
            (
                "unpriced_turns_day",
                UsageValue::Count(u64::from(day.charges.unpriced.0)),
            ),
            (
                "unpriced_turns_month",
                UsageValue::Count(u64::from(month.charges.unpriced.0)),
            ),
        ]
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value))
        .collect::<Vec<_>>();
        let caps = line.limits.app;
        rows.extend(
            [("cap_day", caps.daily), ("cap_month", caps.monthly)]
                .into_iter()
                .filter_map(|(name, limit)| Some((name.to_owned(), UsageValue::Money(limit?)))),
        );
        rows
    }
}

/// One value of the `Usage` dictionary: a count, or an amount of money in its currency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageValue {
    /// Tokens or turns.
    Count(u64),
    /// A reported total or a cap.
    Money(Money),
}

/// An amount as the `Usage` dictionary and the settings rows write it: its decimal digits, a
/// space and its ISO 4217 code (`0.0095 USD`).
#[must_use]
pub fn written(money: Money) -> String {
    format!("{} {}", money.amount.normalize(), money.currency.code())
}

/// Moves a ledger file that could not be read out of the way (`base_fs::move_aside`), so it can be
/// looked at and the next save does not write over it.
fn set_aside(file: &std::path::Path, now: UnixSeconds) {
    match move_aside(file, now) {
        Ok(aside) => eprintln!(
            "inferd: spend: {} could not be read; moved to {}. Spending limits stop requests \
             until the day and the month end.",
            file.display(),
            aside.display()
        ),
        Err(why) => eprintln!("inferd: spend: {why}"),
    }
}

fn strictest(a: SpendVerdict, b: SpendVerdict) -> SpendVerdict {
    let rank = |verdict: SpendVerdict| match verdict {
        SpendVerdict::Within => 0,
        SpendVerdict::Warn => 1,
        SpendVerdict::Stop => 2,
    };
    if rank(b) > rank(a) { b } else { a }
}

#[cfg(test)]
mod tests;
