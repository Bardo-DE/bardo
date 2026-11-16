//! One start of a program with styles, as the guard file keeps it, and the file's small text
//! form. The file is a TOML subset the guard writes and reads itself:
//!
//! ```toml
//! [[start]]
//! hash = "0123456789abcdef"
//! good = false
//! ```
//!
//! Reading is lenient: a line it does not know is skipped and a half-written record is dropped,
//! so a damaged file is the same as no history.

use ui_components::style::kit::Fingerprint;

/// One start: the hash of the three sheets it loaded, and whether it got far enough.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Record {
    /// The sheets' fingerprint.
    pub hash: Fingerprint,
    /// Whether the first frame was drawn and the program lived five seconds more. `false`: the
    /// styles were about to load and nothing more is known.
    pub good: bool,
}

/// The records in `text`, oldest first.
pub fn parse(text: &str) -> Vec<Record> {
    let mut records = Vec::new();
    let mut hash = None;
    let mut good = None;
    let mut close = |hash: &mut Option<Fingerprint>, good: &mut Option<bool>| {
        if let (Some(hash), Some(good)) = (hash.take(), good.take()) {
            records.push(Record { hash, good });
        }
    };
    for line in text.lines().map(str::trim) {
        if line == "[[start]]" {
            close(&mut hash, &mut good);
        } else if let Some(value) = value_of(line, "hash") {
            hash = u64::from_str_radix(value, 16).ok().map(Fingerprint);
        } else if let Some(value) = bool_of(line, "good") {
            good = Some(value);
        }
    }
    close(&mut hash, &mut good);
    records
}

/// `value` of a `key = "value"` line.
fn value_of<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.strip_prefix(key)?
        .trim_start()
        .strip_prefix('=')?
        .trim()
        .strip_prefix('"')?
        .strip_suffix('"')
}

/// `value` of a `key = true` or `key = false` line.
fn bool_of(line: &str, key: &str) -> Option<bool> {
    line.strip_prefix(key)?
        .trim_start()
        .strip_prefix('=')?
        .trim()
        .parse()
        .ok()
}

/// The text form of `records`.
pub fn render(records: &[Record]) -> String {
    records
        .iter()
        .map(|record| {
            format!(
                "[[start]]\nhash = \"{}\"\ngood = {}\n",
                record.hash.hex(),
                record.good
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
