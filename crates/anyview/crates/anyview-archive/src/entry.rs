//! What a listing holds: entries with their kind and size, and how many there are.

use anyview_core::{ArchiveFormat, ByteLen};
use ui_core::word::Word;
use std::ops::ControlFlow;

/// What an entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word)]
pub enum EntryKind {
    /// A file with bytes.
    File,
    /// A folder.
    Directory,
    /// A symbolic or hard link.
    Link,
    /// A device, a pipe or anything else with no bytes to show.
    Other,
}

/// One entry of an archive.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Entry {
    /// Its path inside the archive, `/`-separated, as the archive spells it.
    pub path: String,
    /// What it is.
    pub kind: EntryKind,
    /// Its size once unpacked; `None` when the stream it sits in was not read that far.
    pub size: Option<ByteLen>,
}

/// How many entries an archive has, as far as a listing could see.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntryCount {
    /// The whole index was read.
    Exact(u32),
    /// Only the start was read: there are at least this many.
    AtLeast(u32),
}

impl EntryCount {
    /// The count, whether or not it is exact.
    pub fn get(self) -> u32 {
        match self {
            EntryCount::Exact(count) | EntryCount::AtLeast(count) => count,
        }
    }

    /// `12`, or `12+` for a lower bound.
    pub fn text(self) -> String {
        match self {
            EntryCount::Exact(count) => count.to_string(),
            EntryCount::AtLeast(count) => format!("{count}+"),
        }
    }
}

/// The start of an archive's index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing {
    /// What the file was sniffed as.
    pub format: ArchiveFormat,
    /// Whether it is one file compressed on its own (a `.gz` that is not a tarball), not an
    /// archive of entries.
    pub one_file: bool,
    /// The first entries, in the archive's own order, at most as many as were asked for.
    pub entries: Vec<Entry>,
    /// How many entries there are.
    pub count: EntryCount,
    /// The sizes of the counted entries added up.
    pub unpacked: ByteLen,
}

/// How many entries a listing keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntryLimit(pub u32);

/// The most entries a listing counts before it reports a lower bound.
pub(crate) const COUNT_LIMIT: u32 = 100_000;

/// Collects entries as a format's reader yields them: keeps the first `limit`, counts all, adds
/// their sizes.
#[derive(Debug)]
pub(crate) struct Tally {
    limit: usize,
    entries: Vec<Entry>,
    count: u32,
    unpacked: u64,
}

impl Tally {
    pub(crate) fn new(limit: EntryLimit) -> Self {
        Tally {
            limit: usize::try_from(limit.0).unwrap_or(usize::MAX),
            entries: Vec::new(),
            count: 0,
            unpacked: 0,
        }
    }

    /// Takes one more entry; `Break` once the count limit is reached and reading should stop.
    pub(crate) fn push(&mut self, entry: Entry) -> ControlFlow<()> {
        if self.count >= COUNT_LIMIT {
            return ControlFlow::Break(());
        }
        self.count += 1;
        self.unpacked = self
            .unpacked
            .saturating_add(entry.size.map_or(0, |size| size.0));
        if self.entries.len() < self.limit {
            self.entries.push(entry);
        }
        ControlFlow::Continue(())
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub(crate) fn finish(self, format: ArchiveFormat, complete: bool) -> Listing {
        let count = if complete && self.count < COUNT_LIMIT {
            EntryCount::Exact(self.count)
        } else {
            EntryCount::AtLeast(self.count)
        };
        Listing {
            format,
            one_file: false,
            entries: self.entries,
            count,
            unpacked: ByteLen(self.unpacked),
        }
    }
}
