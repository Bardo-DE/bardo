//! What Get Info says about one item.

use std::time::SystemTime;

use crate::{ByteSize, EntryKind, FileName, FolderPath, ItemRef, LinkTo};

/// The facts about one item, as the filesystem gave them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Info {
    /// Its name.
    pub name: FileName,
    /// The folder it is in.
    pub folder: FolderPath,
    /// What it is, with what only that kind has (a file's size, a link's target).
    pub kind: EntryKind,
    /// When it was made; `None` when the filesystem does not say.
    pub created: Option<SystemTime>,
    /// When its contents last changed; `None` when the filesystem does not say.
    pub modified: Option<SystemTime>,
    /// For a folder: how many items it holds (hidden ones left out, as the list does); `None` for
    /// anything else, or when it can't be read.
    pub items: Option<u32>,
}

impl Info {
    /// The facts of `item`, which is of `kind`, with nothing else known yet.
    #[must_use]
    pub fn new(item: ItemRef, kind: EntryKind) -> Info {
        Info {
            name: item.name,
            folder: item.folder,
            kind,
            created: None,
            modified: None,
            items: None,
        }
    }

    /// The same facts, made at `at`.
    #[must_use]
    pub fn created_at(self, at: SystemTime) -> Info {
        Info {
            created: Some(at),
            ..self
        }
    }

    /// The same facts, last changed at `at`.
    #[must_use]
    pub fn modified_at(self, at: SystemTime) -> Info {
        Info {
            modified: Some(at),
            ..self
        }
    }

    /// The same facts for a folder holding `count` items.
    #[must_use]
    pub fn holding(self, count: u32) -> Info {
        Info {
            items: Some(count),
            ..self
        }
    }

    /// The item these are the facts of.
    #[must_use]
    pub fn item(&self) -> ItemRef {
        ItemRef::new(self.folder.clone(), self.name.clone())
    }

    /// Its size in bytes, for a file or a link to one; `None` for anything else.
    #[must_use]
    pub fn size(&self) -> Option<ByteSize> {
        match &self.kind {
            EntryKind::File(size) | EntryKind::Link(LinkTo::File(size)) => Some(*size),
            EntryKind::Folder | EntryKind::Link(_) | EntryKind::Special(_) => None,
        }
    }
}
