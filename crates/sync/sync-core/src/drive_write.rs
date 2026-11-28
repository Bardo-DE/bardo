//! Writes to a whole drive, beside the app folder's [`crate::Replica`]: what a drive's folder on
//! this computer sends when a person changes it. Items are named by id and placed by a parent id
//! and a name (never a path: a drive's names are shown through naming rules this crate does not
//! know). Every change of an existing item carries the version it was made against, and a stale
//! one is a [`PutError::Conflict`], never an overwrite. Uploads read their bytes a chunk at a
//! time; a large one runs in an upload session that can be resumed after a restart. A delete
//! sends the item to the drive's recycle bin, never a permanent delete.
//!
//! A write whose answer was lost can be told from one that never landed: [`DriveWrite::look`]
//! says what the drive has as an item (its size and content hash), and a drive that lets its
//! client choose ids ([`DriveWrite::reserve_ids`]) makes a new item under the id reserved for
//! it, so a retry finds it there rather than making a second.

use crate::item::{BaseVersion, ContentHash, RemoteId, RemoteVersion};
use crate::refusal::PutError;
use serde::{Deserialize, Serialize};

/// Where an item goes: a folder by id (`None`: the top of the drive) and its name there.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Place {
    /// The folder.
    pub parent: Option<RemoteId>,
    /// The name, as the drive will have it.
    pub name: String,
    /// The id reserved for a new item ([`DriveWrite::reserve_ids`]), which a drive that takes
    /// one makes it under.
    pub id: Option<RemoteId>,
}

impl Place {
    /// `name` in the folder `parent`, with no id reserved.
    pub fn new(parent: Option<RemoteId>, name: impl Into<String>) -> Self {
        Self {
            parent,
            name: name.into(),
            id: None,
        }
    }
}

/// What an upload makes or replaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadTo {
    /// A new file.
    New(Place),
    /// New content for a file that exists: the same id, a new version, so the drive keeps the
    /// history.
    Replace(RemoteId),
}

/// A drive's upload session in flight: its address.
///
/// The address takes bytes with no sign-in, so it is a credential of its own: stored owner-only,
/// and never shown or logged (its `Debug` says nothing of it).
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Session(String);

impl Session {
    /// The session at `address`.
    pub fn new(address: impl Into<String>) -> Self {
        Self(address.into())
    }

    /// Its address, for the replica that sends to it.
    #[must_use]
    pub fn address(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Session(..)")
    }
}

/// What an upload sends: `size` bytes, read a span at a time, never whole. Where they live (a
/// file on this computer) is the runner's: this crate only asks for spans.
pub trait Content: std::fmt::Debug + Send + Sync {
    /// How many bytes it is.
    fn size(&self) -> u64;

    /// `len` bytes from `offset` (fewer at its end).
    ///
    /// # Errors
    /// The bytes could not be read.
    fn read(&self, offset: u64, len: u64) -> std::io::Result<Vec<u8>>;
}

/// Bytes held in memory.
impl Content for Vec<u8> {
    fn size(&self) -> u64 {
        self.len() as u64
    }

    fn read(&self, offset: u64, len: u64) -> std::io::Result<Vec<u8>> {
        let start = usize::try_from(offset).map_or(self.len(), |at| at.min(self.len()));
        let end =
            usize::try_from(offset.saturating_add(len)).map_or(self.len(), |at| at.min(self.len()));
        Ok(self[start..end].to_vec())
    }
}

/// What an upload tells as it goes.
pub trait Progress: Send + Sync {
    /// The session it opened (`None`: the one it had is over), to be kept with the change so
    /// the upload resumes after a restart.
    fn session(&self, session: Option<&Session>);

    /// How many bytes the drive has so far.
    fn sent(&self, bytes: u64);

    /// Whether the upload is to stop between chunks (the drive was paused): it ends as
    /// [`PutError::Transient`], its session kept, to resume later.
    fn held(&self) -> bool {
        false
    }
}

/// A [`Progress`] that keeps nothing.
#[derive(Debug, Clone, Copy, Default)]
pub struct Unheard;

impl Progress for Unheard {
    fn session(&self, _: Option<&Session>) {}

    fn sent(&self, _: u64) {}
}

/// One upload.
pub struct Upload<'a> {
    /// What it makes or replaces.
    pub to: UploadTo,
    /// What a replaced file was when the change was made (`Absent` for a new file).
    pub base: BaseVersion,
    /// The bytes.
    pub content: &'a dyn Content,
    /// A session from before, resumed when the drive still has it.
    pub resume: Option<Session>,
    /// Told of the session and of the bytes sent.
    pub progress: &'a dyn Progress,
}

impl std::fmt::Debug for Upload<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Upload")
            .field("to", &self.to)
            .field("base", &self.base)
            .field("content", &self.content)
            .field("resume", &self.resume)
            .finish_non_exhaustive()
    }
}

/// An item as the drive has it after a write.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Written {
    /// Its id.
    pub id: RemoteId,
    /// Its version now.
    pub version: RemoteVersion,
}

/// An item as the drive has it now, to tell whether a write whose answer was lost landed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Found {
    /// Its version.
    pub version: RemoteVersion,
    /// Its size (a folder's is 0).
    pub size: u64,
    /// Its content hash, of the kind the drive reports, when it reports one.
    pub hash: Option<ContentHash>,
    /// Whether it is a folder.
    pub folder: bool,
}

/// The writes of a whole drive. Each is one step: a folder, a file's bytes, a new place, the
/// recycle bin.
pub trait DriveWrite: Send + Sync {
    /// Whether the drive takes `name` for a file or folder. A name it refuses is refused when it
    /// is made here, not when it is sent.
    fn accepts_name(&self, name: &str) -> bool;

    /// The longest path the drive takes for an item, in characters, from the top of the drive
    /// (its names joined by `/`); `None` when it sets none. A longer one is refused when it is
    /// made here, as a name is.
    fn path_max(&self) -> Option<usize> {
        None
    }

    /// `item` as the drive has it now; `None` when it has no such item (or has it only in its
    /// recycle bin).
    fn look(&self, item: &RemoteId)
    -> impl Future<Output = Result<Option<Found>, PutError>> + Send;

    /// Ids for `count` items not made yet, for a drive that lets its client choose them: an item
    /// made with one ([`Place::id`]) is made once however often it is sent. A drive that does
    /// not gives none.
    fn reserve_ids(
        &self,
        count: usize,
    ) -> impl Future<Output = Result<Vec<RemoteId>, PutError>> + Send {
        let _ = count;
        async { Ok(Vec::new()) }
    }

    /// Makes a folder at `place`. A name taken there is a `Conflict` (`Exists`).
    fn make_folder(&self, place: &Place) -> impl Future<Output = Result<Written, PutError>> + Send;

    /// Sends a file's bytes: a small one in one request, a larger one in an upload session, in
    /// chunks.
    fn upload(&self, upload: Upload<'_>) -> impl Future<Output = Result<Written, PutError>> + Send;

    /// Renames and/or moves `item` to `to`, if the drive still has `base`.
    fn relocate(
        &self,
        item: &RemoteId,
        base: &BaseVersion,
        to: &Place,
    ) -> impl Future<Output = Result<Written, PutError>> + Send;

    /// Sends `item` to the drive's recycle bin, if the drive still has `base`; a folder takes
    /// what it holds.
    fn trash(
        &self,
        item: &RemoteId,
        base: &BaseVersion,
    ) -> impl Future<Output = Result<(), PutError>> + Send;

    /// Gives up an upload session that will not be resumed (the bytes it was sending changed),
    /// so the drive drops what it holds of it. A drive that drops sessions only in time does
    /// nothing.
    fn abandon(&self, session: &Session) -> impl Future<Output = ()> + Send {
        let _ = session;
        async {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_session_never_shows_its_address() {
        let session = Session::new("https://up.example.test/s/secret");
        assert_eq!(format!("{session:?}"), "Session(..)");
        assert_eq!(session.address(), "https://up.example.test/s/secret");
        let json = serde_json::to_string(&session).expect("json");
        assert_eq!(
            serde_json::from_str::<Session>(&json).expect("session"),
            session
        );
    }

    #[test]
    fn bytes_in_memory_are_read_a_span_at_a_time_and_stop_at_their_end() {
        let content = b"0123456789".to_vec();
        assert_eq!(content.size(), 10);
        assert_eq!(content.read(2, 3).expect("read"), b"234");
        assert_eq!(content.read(6, 10).expect("read"), b"6789");
        assert_eq!(content.read(10, 1).expect("read"), b"");
        assert_eq!(content.read(99, 1).expect("read"), b"");
    }
}
