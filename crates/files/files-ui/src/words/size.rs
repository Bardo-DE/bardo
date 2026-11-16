//! Sizes as Finder says them: decimal units, whole kilobytes, one decimal from megabytes up.

use files_core::{ByteSize, Entry, EntryKind, LinkTo};

use super::NONE;

/// Units above bytes, each a thousand of the one before.
const UNITS: [&str; 5] = ["KB", "MB", "GB", "TB", "PB"];

/// `size` in words: `Zero bytes`, `1 byte`, `512 bytes`, `4 KB`, `1.5 MB`, `12 GB`.
///
/// Kilobytes are whole; megabytes and up keep one decimal, dropped when it is zero. A size that
/// rounds up to a thousand of its unit is shown in the next one (`999999` is `1 MB`).
#[must_use]
pub fn size_words(size: ByteSize) -> String {
    match size.0 {
        0 => "Zero bytes".to_owned(),
        1 => "1 byte".to_owned(),
        bytes @ 2..=999 => format!("{bytes} bytes"),
        bytes => scaled(bytes),
    }
}

/// What the Size column says of `entry`: a file's size, or a dash for anything with none (a folder,
/// a link that leads nowhere, a pipe).
#[must_use]
pub fn entry_size_words(entry: &Entry) -> String {
    match &entry.kind {
        EntryKind::File(size) | EntryKind::Link(LinkTo::File(size)) => size_words(*size),
        EntryKind::Folder | EntryKind::Link(_) | EntryKind::Special(_) => NONE.to_owned(),
    }
}

/// `bytes` (a thousand or more) in kilobytes or a bigger unit.
#[expect(
    clippy::cast_precision_loss,
    reason = "a byte count shown rounded to three digits; precision past 2^53 does not change the text"
)]
fn scaled(bytes: u64) -> String {
    let mut value = bytes as f64 / 1000.0;
    let mut unit = 0;
    loop {
        let shown = rounded(unit, value);
        if shown < 1000.0 || unit + 1 == UNITS.len() {
            return written(unit, shown);
        }
        value /= 1000.0;
        unit += 1;
    }
}

/// `value` of `UNITS[unit]` rounded as it is shown: whole kilobytes, tenths above.
fn rounded(unit: usize, value: f64) -> f64 {
    if unit == 0 {
        value.round()
    } else {
        (value * 10.0).round() / 10.0
    }
}

/// `shown` of `UNITS[unit]` as text.
fn written(unit: usize, shown: f64) -> String {
    let name = UNITS.get(unit).copied().unwrap_or("PB");
    if shown.fract() == 0.0 {
        format!("{shown:.0} {name}")
    } else {
        format!("{shown:.1} {name}")
    }
}

#[cfg(test)]
mod tests {
    use files_core::{FileName, Special};

    use super::*;

    fn entry(name: &str, kind: EntryKind) -> Entry {
        Entry::new(FileName::new(name).unwrap(), kind)
    }

    #[test]
    fn sizes_are_said_as_finder_says_them() {
        const CASES: &[(&str, u64, &str)] = &[
            ("empty", 0, "Zero bytes"),
            ("one byte", 1, "1 byte"),
            ("a few bytes", 512, "512 bytes"),
            ("the most bytes", 999, "999 bytes"),
            ("a thousand is a kilobyte", 1_000, "1 KB"),
            ("kilobytes are whole", 4_096, "4 KB"),
            ("kilobytes round", 1_499, "1 KB"),
            ("kilobytes round up", 1_500, "2 KB"),
            ("the most kilobytes", 999_499, "999 KB"),
            ("rounds into megabytes", 999_999, "1 MB"),
            ("a whole megabyte drops its decimal", 1_000_000, "1 MB"),
            ("megabytes keep one decimal", 1_500_000, "1.5 MB"),
            ("megabytes round to a tenth", 123_456_789, "123.5 MB"),
            ("gigabytes", 12_340_000_000, "12.3 GB"),
            ("terabytes", 2_000_000_000_000, "2 TB"),
            ("the biggest", u64::MAX, "18446.7 PB"),
        ];
        for &(name, bytes, want) in CASES {
            assert_eq!(size_words(ByteSize(bytes)), want, "{name}");
        }
    }

    #[test]
    fn only_files_have_a_size_to_show() {
        let cases = [
            (
                "a file",
                entry("a.txt", EntryKind::File(ByteSize(2_000))),
                "2 KB",
            ),
            ("a folder", entry("d", EntryKind::Folder), NONE),
            (
                "a link to a file",
                entry("l", EntryKind::Link(LinkTo::File(ByteSize(5)))),
                "5 bytes",
            ),
            (
                "a link to a folder",
                entry("l", EntryKind::Link(LinkTo::Folder)),
                NONE,
            ),
            (
                "a broken link",
                entry("l", EntryKind::Link(LinkTo::Broken)),
                NONE,
            ),
            (
                "a pipe",
                entry("p", EntryKind::Special(Special::Fifo)),
                NONE,
            ),
        ];
        for (name, entry, want) in cases {
            assert_eq!(entry_size_words(&entry), want, "{name}");
        }
    }
}
