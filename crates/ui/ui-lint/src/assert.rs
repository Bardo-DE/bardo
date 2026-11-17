//! The one call a consumer's test makes.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use super::rule::{Exception, LintConfig, Rule};
use super::severity;
use super::stylesheet::every_offence;
use crate::rule::Offence;

/// Panics, listing every offence with its line and column, unless `css` is clean.
///
/// Clean means that once `config.exceptions` are applied nothing is left and every exception
/// suppressed at least one offence. An exception that suppressed nothing is stale, and a stale
/// exception is a failure. It prints what each exception suppressed (to stderr on success, in the
/// panic otherwise).
///
/// Warnings ([`super::Severity::Warning`]) never fail it: they are printed to stderr, and an
/// exception that suppresses only a warning is not stale.
///
/// # Panics
///
/// Panics when an offence is not excused or an exception suppressed nothing.
pub fn assert_clean(css: &str, config: &LintConfig) {
    let (unexcused, suppressed) = config.partition(every_offence(css, config));
    let (offences, warnings) = severity::split(unexcused);
    let stale = stale(config, &suppressed);
    let counts = suppressed_counts(config, &suppressed);
    eprint!("{}", warning_text(&warnings));
    match (offences.is_empty(), stale.is_empty()) {
        (true, true) => eprint!("{counts}"),
        (true, false) => panic!("{}{counts}", stale_text(&stale)),
        (false, _) => panic!("{}{counts}{}", offence_text(&offences), stale_text(&stale)),
    }
}

/// The exceptions of `config` that suppressed none of `suppressed`.
fn stale<'a>(config: &'a LintConfig, suppressed: &[Offence]) -> Vec<&'a Exception> {
    config
        .exceptions
        .iter()
        .filter(|exception| !suppressed.iter().any(|offence| exception.covers(offence)))
        .collect()
}

/// The offences, grouped by rule, each with its line and column.
fn offence_text(offences: &[Offence]) -> String {
    let mut by_rule: BTreeMap<Rule, Vec<&Offence>> = BTreeMap::new();
    for offence in offences {
        by_rule.entry(offence.rule).or_default().push(offence);
    }
    let mut message = format!("bardo lint found {} offence(s):\n", offences.len());
    for (rule, group) in &by_rule {
        for offence in group {
            writeln!(
                message,
                "  {rule:?} {}:{}: {}",
                offence.line, offence.column, offence.text
            )
            .expect("writing to a String cannot fail");
        }
    }
    message
}

/// One line per warning, or nothing.
fn warning_text(warnings: &[Offence]) -> String {
    if warnings.is_empty() {
        return String::new();
    }
    let mut lines = String::new();
    for offence in warnings {
        writeln!(
            lines,
            "  {:?} {}:{}: {}",
            offence.rule, offence.line, offence.column, offence.text
        )
        .expect("writing to a String cannot fail");
    }
    format!(
        "bardo lint warns {} time(s) (not yet failing; design/27 section 7):\n{lines}",
        warnings.len()
    )
}

/// One line per stale exception, or nothing.
fn stale_text(stale: &[&Exception]) -> String {
    if stale.is_empty() {
        return String::new();
    }
    let mut text = format!(
        "bardo lint found {} stale exception(s), which suppressed nothing:\n",
        stale.len()
    );
    for exception in stale {
        writeln!(
            text,
            "  {:?} on {} ({})",
            exception.rule, exception.selector, exception.reason
        )
        .expect("writing to a String cannot fail");
    }
    text
}

/// One line per exception: how many offences it suppressed.
fn suppressed_counts(config: &LintConfig, suppressed: &[Offence]) -> String {
    if config.exceptions.is_empty() {
        return String::new();
    }
    let mut text = format!(
        "{} offence(s) suppressed by exceptions:\n",
        suppressed.len()
    );
    for exception in config.exceptions {
        let count = suppressed
            .iter()
            .filter(|offence| exception.covers(offence))
            .count();
        writeln!(
            text,
            "  {count} x {:?} on {} ({})",
            exception.rule, exception.selector, exception.reason
        )
        .expect("writing to a String cannot fail");
    }
    text
}
