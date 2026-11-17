//! "Save a Copy": an online document's export, written next to it as a file made on this
//! computer, "Waiting to upload" until it is sent.

use super::{Drive, NodeId, held, now_secs, unlisted};
use crate::SkipError;
use crate::index::{Dirty, Edit};
use crate::naming;
use crate::source::CopyError;
use base_fs::atomic::AtomicWrite;
use sync_core::ItemKind;

/// The name a file made here is shown by when the index lost it (a crash between its write and
/// the index's), followed by its number.
pub(super) const RECOVERED: &str = "Recovered File";

impl Drive {
    /// Saves the online document `id` as a file in its export format next to it, named as the
    /// document is with the format's extension ("Plan.docx", or "Plan 2.docx" when that is
    /// taken); the copy's node.
    ///
    /// # Errors
    /// [`Reason::NotADocument`] for an item that is no online document, [`Reason::NoFileFormat`]
    /// for one with no file format (a Form, a Site, a Map), [`Reason::TooLarge`] past what the
    /// drive exports, [`Reason::CopyFailed`] when the drive could not be reached.
    pub async fn save_copy(&self, id: &NodeId) -> Result<NodeId, SkipError> {
        let NodeId::Remote(remote) = id else {
            return Err(SkipError::NotADocument);
        };
        let inner = &self.inner;
        let entry = inner
            .entry(remote)
            .map_err(unlisted(SkipError::NotADocument))?
            .ok_or(SkipError::NotADocument)?;
        let ItemKind::Online { export, .. } = &entry.kind else {
            return Err(SkipError::NotADocument);
        };
        let (mime, extension) = export
            .as_deref()
            .and_then(|mime| Some((mime, naming::copy_extension(mime)?)))
            .ok_or(SkipError::NoFileFormat)?;
        let bytes = inner
            .source
            .export(remote, mime)
            .await
            .map_err(|why| match why {
                CopyError::NoFormat => SkipError::NoFileFormat,
                CopyError::TooLarge => SkipError::TooLarge,
                CopyError::Failed => SkipError::CopyFailed,
            })?;
        let folder = inner
            .parent_of(id)
            .map_err(|why| {
                eprintln!("drive-core: the folder of a document to copy is unknown: {why}");
                SkipError::CopyFailed
            })?
            .ok_or(SkipError::NotADocument)?;
        let taken: Vec<String> = inner
            .children(&folder)
            .map_err(|why| {
                eprintln!("drive-core: the folder of a copy could not be listed: {why}");
                SkipError::CopyFailed
            })?
            .into_iter()
            .map(|node| node.name)
            .collect();
        let wanted = format!("{}.{extension}", naming::shown_base(&entry.name));
        let name = naming::free(&wanted, |name| taken.iter().any(|t| t == name));
        let n = held(&inner.index).take_local();
        AtomicWrite::PRIVATE
            .write(&inner.local_file(n), &bytes)
            .map_err(|why| {
                eprintln!("drive-core: a copy of a document could not be written: {why}");
                SkipError::CopyFailed
            })?;
        let made = now_secs();
        let dirty = Dirty {
            file: n,
            size: bytes.len() as u64,
            modified: made,
            generation: 0,
        };
        let mut edit = Edit::made_here(n, folder.clone(), name, Some(dirty), made);
        {
            let mut index = held(&inner.index);
            if let Some(dirty) = edit.dirty.as_mut() {
                dirty.generation = index.take_generation();
            }
            edit.seq = index.take_seq();
            index.edits.insert(edit.id.clone(), edit)
        };
        inner.touched();
        // On the disk before it is answered: a copy said made is never lost.
        if inner.flush_now().is_err() {
            held(&inner.index).edits.remove(&NodeId::Local(n));
            let _ = std::fs::remove_file(inner.local_file(n));
            inner.touched();
            return Err(SkipError::CopyFailed);
        }
        let copy = NodeId::Local(n);
        inner.ino(&copy);
        inner.wake.notify_one();
        inner.emit(vec![copy.clone(), folder]);
        Ok(copy)
    }
}
