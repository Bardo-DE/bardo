//! `$XDG_STATE_HOME/bardo/shell/notifications.json`: the history, kept across restarts (effects: the
//! filesystem). Written whole and durably after every change that
//! touches the history (`NotificationsEffect::SaveHistory`); read once at start. A file that cannot
//! be read is kept as `.damaged` rather than written over.
//!
//! An image the sender sent as pixels (`image-data`) is not written: the file would grow by
//! megabytes for a contact photo, and an image path or icon name is kept as it was.

use std::path::{Path, PathBuf};

use shell_model::notifications::{STORED_VERSION, StoredHistory};

use crate::durable;
use crate::error::ServiceError;
use base_fs::NewFileMode;

/// The file's name inside `$XDG_STATE_HOME/bardo/shell`.
pub const FILE_NAME: &str = "notifications.json";

/// `notifications.json` in bardo-shell's folder of `dirs`' state directory.
pub fn path_in(dirs: &shell_model::dirs::Dirs) -> PathBuf {
    dirs.sill_state().join(FILE_NAME)
}

/// The file at `path`; empty when there is none, it cannot be read, or it is another version
/// (a lost history costs the list, never a failed start).
pub fn load(path: &Path) -> StoredHistory {
    let Ok(text) = std::fs::read_to_string(path) else {
        return StoredHistory::default();
    };
    match serde_json::from_str::<StoredHistory>(&text) {
        Ok(stored) if stored.version == STORED_VERSION => stored,
        // Not this version's, or not readable: the next save would replace it, so it is kept.
        Ok(_) | Err(_) => {
            crate::durable::set_aside(path);
            StoredHistory::default()
        }
    }
}

/// Write `stored` to `path` (its directory created) durably: private to the person, whole or not
/// at all ([`crate::durable`]).
pub fn save(path: &Path, stored: &StoredHistory) -> Result<(), ServiceError> {
    let text = serde_json::to_string(stored).map_err(|e| ServiceError::Io {
        path: path.to_path_buf(),
        source: std::io::Error::other(e),
    })?;
    durable::write_file(path, text.as_bytes(), NewFileMode::OwnerOnly)
}

#[cfg(test)]
mod tests {
    use super::{load, save};
    use shell_model::notifications::Body;
    use shell_model::notifications::NotesState;
    use shell_model::notifications::{AppGroup, History};
    use shell_model::notifications::{AppKey, NotificationId, UnixMs};
    use shell_model::notifications::{
        AppRef, Expiry, Hints, IconRef, NoteImage, Notification, Pixels,
    };
    use shell_model::notifications::{STORED_VERSION, StoredHistory, for_disk};

    /// The file `name` was set aside as (`<name>.bad-<seconds>`), if there is one.
    fn set_aside_copy(dir: &std::path::Path, name: &str) -> Option<std::path::PathBuf> {
        let prefix = format!("{name}.bad-");
        std::fs::read_dir(dir)
            .ok()?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .find(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with(&prefix))
            })
    }

    fn note(id: u32, image: NoteImage) -> Notification {
        Notification {
            id: NotificationId(id),
            arrived: UnixMs(1_790_000_000_000),
            app: AppRef {
                name: "Mail".to_owned(),
                icon: IconRef::Name("mail-unread".to_owned()),
                desktop_entry: Some("org.example.Mail".to_owned()),
            },
            summary: "Ada".to_owned(),
            body: Body::plain("the engine"),
            actions: Vec::new(),
            hints: Hints {
                image,
                ..Hints::default()
            },
            expiry: Expiry::Default,
        }
    }

    fn state() -> NotesState {
        let pixels = NoteImage::Pixels(Pixels {
            width: 1,
            height: 1,
            rgba: vec![1, 2, 3, 255].into(),
        });
        NotesState {
            history: History {
                groups: vec![AppGroup {
                    key: AppKey("org.example.Mail".to_owned()),
                    notes: vec![note(7, pixels), note(6, NoteImage::Icon(IconRef::None))],
                }],
            },
            last_id: NotificationId(7),
            ..NotesState::default()
        }
    }

    #[test]
    fn pixels_are_not_written_and_the_rest_round_trips() {
        let dir = std::env::temp_dir().join(format!("shell-notes-{}", std::process::id()));
        let path = dir.join("state/bardo/shell/notifications.json");
        let stored = for_disk(&state());
        assert_eq!(stored.version, STORED_VERSION);
        assert_eq!(stored.last_id, NotificationId(7));
        assert_eq!(
            stored.history.groups[0].notes[0].hints.image,
            NoteImage::None
        );
        assert_eq!(
            stored.history.groups[0].notes[1].hints.image,
            NoteImage::Icon(IconRef::None)
        );
        save(&path, &stored).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(load(&path), stored);
        assert_eq!(
            std::fs::read_dir(path.parent().unwrap_or(&dir))
                .map(std::iter::Iterator::count)
                .unwrap_or_default(),
            1,
            "no temporary file is left beside the history"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_broken_or_foreign_file_is_empty() {
        let dir = std::env::temp_dir().join(format!("shell-notes-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(load(&dir.join("absent.json")), StoredHistory::default());
        let broken = dir.join("broken.json");
        std::fs::write(&broken, "{ not json").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(load(&broken), StoredHistory::default());
        assert!(
            set_aside_copy(&dir, "broken.json").is_some(),
            "the unreadable history is kept, not left for the next save to replace"
        );
        let foreign = dir.join("foreign.json");
        let other = StoredHistory {
            version: STORED_VERSION + 1,
            ..for_disk(&state())
        };
        std::fs::write(&foreign, serde_json::to_string(&other).unwrap_or_default())
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(load(&foreign), StoredHistory::default());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
