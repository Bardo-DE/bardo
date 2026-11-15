//! What Get Info says of an item, as rows of label and words.

use std::time::SystemTime;

use files_core::{ByteSize, Entry, Info, ItemCount};
use files_model::ListFailure;

use super::{kind_words, size_words};

/// One row of Get Info: what it is and what it says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoRow {
    /// The row's name: `Kind`, `Size`, `Where`.
    pub label: &'static str,
    /// What it says.
    pub value: String,
}

impl InfoRow {
    pub(super) fn new(label: &'static str, value: String) -> InfoRow {
        InfoRow { label, value }
    }
}

/// `size` as Get Info says it: the words the list uses, then the exact bytes once they differ
/// from the words: `12 KB (12,345 bytes)`, `5 bytes`.
pub fn exact_size_words(size: ByteSize) -> String {
    let words = size_words(size);
    if size.0 < 1000 {
        words
    } else {
        format!("{words} ({} bytes)", grouped(size.0))
    }
}

/// `count` as `Zero items`, `1 item`, `12 items`.
pub fn items_words(count: ItemCount) -> String {
    match count.0 {
        0 => "Zero items".to_owned(),
        1 => "1 item".to_owned(),
        n => format!("{n} items"),
    }
}

/// The digits of `n` in groups of three, comma between.
fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let first = digits.len() % 3;
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && at % 3 == first {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// The rows Get Info shows for `info`, in order: Kind, Size (a file) or Contents (a folder),
/// Where, Created (when the filesystem says), Modified. `when` writes a date.
pub fn info_rows(info: &Info, when: impl Fn(Option<SystemTime>) -> String) -> Vec<InfoRow> {
    let kind = kind_words(&Entry::new(info.name.clone(), info.kind.clone()));
    let mut rows = vec![InfoRow::new("Kind", kind)];
    if let Some(size) = info.size() {
        rows.push(InfoRow::new("Size", exact_size_words(size)));
    }
    if let Some(count) = info.items {
        rows.push(InfoRow::new("Contents", items_words(count)));
    }
    rows.push(InfoRow::new(
        "Where",
        info.folder.as_path().display().to_string(),
    ));
    if info.created.is_some() {
        rows.push(InfoRow::new("Created", when(info.created)));
    }
    rows.push(InfoRow::new("Modified", when(info.modified)));
    rows
}

/// The rows for the name: `Name`, and `Extension` when it has one. Read-only for now.
pub fn name_rows(info: &Info) -> Vec<InfoRow> {
    let mut rows = vec![InfoRow::new("Name", info.name.display().into_owned())];
    if let Some(extension) = info.name.extension() {
        rows.push(InfoRow::new(
            "Extension",
            extension.to_string_lossy().into_owned(),
        ));
    }
    rows
}

/// What Get Info says when the facts of an item could not be read.
pub fn info_failure_words(failure: ListFailure) -> &'static str {
    match failure {
        ListFailure::NotFound => {
            "This item can\u{2019}t be found. It may have been moved or deleted."
        }
        ListFailure::NoPermission => "You don\u{2019}t have permission to see this item.",
        ListFailure::NotAFolder | ListFailure::Other => {
            "The details of this item can\u{2019}t be read."
        }
    }
}

#[cfg(test)]
mod tests {
    use files_core::{EntryKind, FileName, FolderPath, ItemRef};

    use super::*;
    use crate::words::NONE;

    #[test]
    fn get_info_says_the_words_and_then_the_exact_bytes() {
        const CASES: &[(&str, u64, &str)] = &[
            ("empty", 0, "Zero bytes"),
            ("a few", 999, "999 bytes"),
            ("a thousand", 1_000, "1 KB (1,000 bytes)"),
            ("kilobytes", 12_345, "12 KB (12,345 bytes)"),
            ("megabytes", 1_500_000, "1.5 MB (1,500,000 bytes)"),
            ("a million exactly", 1_000_000, "1 MB (1,000,000 bytes)"),
            (
                "groups of three",
                123_456_789,
                "123.5 MB (123,456,789 bytes)",
            ),
        ];
        for &(name, bytes, want) in CASES {
            assert_eq!(exact_size_words(ByteSize(bytes)), want, "{name}");
        }
        let counts = [(0, "Zero items"), (1, "1 item"), (2, "2 items")];
        for (count, want) in counts {
            assert_eq!(items_words(ItemCount(count)), want);
        }
    }

    #[test]
    fn get_info_rows_follow_the_kind_of_item() {
        let item = |name: &str| {
            ItemRef::new(
                FolderPath::new("/home/p").unwrap(),
                FileName::new(name).unwrap(),
            )
        };
        let when = |at: Option<SystemTime>| at.map_or(NONE.to_owned(), |_| "then".to_owned());
        let labels = |info: &Info| -> Vec<&'static str> {
            info_rows(info, when).iter().map(|row| row.label).collect()
        };
        let file = Info::new(item("Report.pdf"), EntryKind::File(ByteSize(12_345)));
        assert_eq!(labels(&file), ["Kind", "Size", "Where", "Modified"]);
        let rows = info_rows(&file, when);
        assert_eq!(rows[0].value, "PDF file");
        assert_eq!(rows[1].value, "12 KB (12,345 bytes)");
        assert_eq!(rows[2].value, "/home/p");
        assert_eq!(rows[3].value, NONE);

        let made = SystemTime::UNIX_EPOCH;
        let folder = Info::new(item("Projects"), EntryKind::Folder)
            .created_at(made)
            .modified_at(made)
            .holding(ItemCount(3));
        assert_eq!(
            labels(&folder),
            ["Kind", "Contents", "Where", "Created", "Modified"]
        );
        assert_eq!(info_rows(&folder, when)[1].value, "3 items");

        let names = |info: &Info| -> Vec<(&'static str, String)> {
            name_rows(info)
                .into_iter()
                .map(|row| (row.label, row.value))
                .collect()
        };
        assert_eq!(
            names(&file),
            [
                ("Name", "Report.pdf".to_owned()),
                ("Extension", "pdf".to_owned())
            ]
        );
        assert_eq!(names(&folder), [("Name", "Projects".to_owned())]);
    }
}
