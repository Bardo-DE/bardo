//! What kind of thing an entry is, in the words of Finder's Kind column.
//!
//! A MIME type's own description ("PDF document") is quire's and the desktop's to give once a
//! lookup exists; until then a file's kind comes from its extension.

use files_core::{Entry, EntryKind, LinkTo, Special};

/// The Kind column's words for `entry`: `Folder`, `Alias`, `PDF file`, or `Document` for a file
/// with no extension.
pub fn kind_words(entry: &Entry) -> String {
    match &entry.kind {
        EntryKind::Folder => "Folder".to_owned(),
        EntryKind::File(_) => file_words(entry),
        EntryKind::Link(LinkTo::Broken) => "Broken alias".to_owned(),
        EntryKind::Link(_) => "Alias".to_owned(),
        EntryKind::Special(Special::Fifo) => "Pipe".to_owned(),
        EntryKind::Special(Special::Socket) => "Socket".to_owned(),
        EntryKind::Special(Special::Device) => "Device".to_owned(),
    }
}

/// `PDF file` for `report.pdf`; `Document` when the name has no extension.
fn file_words(entry: &Entry) -> String {
    match entry.name.extension() {
        Some(extension) => format!("{} file", extension.to_string_lossy().to_uppercase()),
        None => "Document".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use files_core::{ByteSize, Entry, EntryKind, FileName, LinkTo, Special};

    use super::*;

    fn entry(name: &str, kind: EntryKind) -> Entry {
        Entry::new(FileName::new(name).unwrap(), kind)
    }

    #[test]
    fn kinds_are_said_from_the_entry_and_its_extension() {
        let file = |name: &str| entry(name, EntryKind::File(ByteSize(1)));
        let cases = [
            ("a folder", entry("d", EntryKind::Folder), "Folder"),
            ("a pdf", file("Report.pdf"), "PDF file"),
            (
                "the last extension counts",
                file("archive.tar.gz"),
                "GZ file",
            ),
            ("case is folded", file("PHOTO.JpG"), "JPG file"),
            ("no extension", file("Makefile"), "Document"),
            ("a dotfile has none", file(".bashrc"), "Document"),
            ("a trailing dot has none", file("odd."), "Document"),
            (
                "a link to a file",
                entry("l", EntryKind::Link(LinkTo::File(ByteSize(1)))),
                "Alias",
            ),
            (
                "a link to a folder",
                entry("l", EntryKind::Link(LinkTo::Folder)),
                "Alias",
            ),
            (
                "a broken link",
                entry("l", EntryKind::Link(LinkTo::Broken)),
                "Broken alias",
            ),
            (
                "a pipe",
                entry("p", EntryKind::Special(Special::Fifo)),
                "Pipe",
            ),
            (
                "a socket",
                entry("s", EntryKind::Special(Special::Socket)),
                "Socket",
            ),
            (
                "a device",
                entry("v", EntryKind::Special(Special::Device)),
                "Device",
            ),
        ];
        for (name, entry, want) in cases {
            assert_eq!(kind_words(&entry), want, "{name}");
        }
    }
}
