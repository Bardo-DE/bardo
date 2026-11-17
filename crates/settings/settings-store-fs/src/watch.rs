//! Live reload: a `notify` watch on the config directory, not the file, because editors and the
//! atomic writer both replace the inode by rename (design/22-SETTINGS.md section 2). A burst of
//! events settles for [`DEBOUNCE`], then the whole file is re-read.

use crate::error::SettingsError;
use crate::locator::Locator;
use crate::store::Store;
use crate::text::{FileText, SETTINGS_CAP, file_text};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use settings_store::latest::{self, Receiver, Sender};
use settings_store::{Loaded, Read, SettingsDoc, read_within};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;
use ui_core::spawner::Spawner;

/// How long the watch waits for a burst of events to settle before re-reading.
pub const DEBOUNCE: Duration = Duration::from_millis(30);

/// How long the watch waits before reading again a file the system would not give it (too many
/// open files, permissions mid-change). No file event follows such a failure, so the watch asks
/// again itself; the wait doubles each time.
const RETRY_FIRST: Duration = Duration::from_millis(250);

/// How many times a refused read is asked again before the watch waits for the next file event.
const RETRIES: u32 = 6;

/// Whether a [`Watch`] is looking at the disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchState {
    /// Every settled change arrives through [`Watch::changed`].
    Live,
    /// The directory could not be watched: the value stays as first read and `changed` ends.
    Blind {
        /// Why the watch could not start.
        reason: String,
    },
}

/// A running watch on one settings file. Dropping it stops the watch.
#[derive(Debug)]
pub struct Watch<D> {
    changes: Receiver<Loaded<D>>,
    state: WatchState,
    _watcher: Option<FileWatcher>,
}

/// The directory watcher behind a [`Settled`]: dropping it stops the watch.
#[derive(Debug)]
pub struct FileWatcher {
    _watcher: RecommendedWatcher,
}

/// The settled changes of one watched file: every burst of events on it arrives as one
/// [`settled`](Settled::settled), after [`DEBOUNCE`] with no further event. This is the one
/// directory watch and the one debounce every settings file's watcher is made of.
#[derive(Debug)]
pub struct Settled {
    signals: Receiver<()>,
}

impl Settled {
    /// Wait for the next burst of events on the file to settle. `None` once the watcher is gone.
    pub async fn settled(&mut self) -> Option<()> {
        self.signals.changed().await?;
        // Coalesce the rest of the burst: a rename can fire more than one raw event, and the
        // atomic writer plus a watching editor can both touch the file in quick succession. Every
        // further event restarts the window.
        loop {
            futures_timer::Delay::new(DEBOUNCE).await;
            if !self.signals.has_changed() {
                return Some(());
            }
            self.signals.catch_up();
        }
    }

    /// Whether an event arrived that [`settled`](Settled::settled) has not taken yet.
    #[must_use]
    pub fn has_changed(&self) -> bool {
        self.signals.has_changed()
    }
}

/// Watch `dir` (made when it is missing) for writes to the file named `file`: not the directory's
/// other files, not the temp file the atomic writer (or an editor) writes and renames away, and
/// not a read of it. The watcher is the caller's to keep alive; the [`Settled`] ends when it is
/// dropped.
///
/// # Errors
///
/// Returns a [`SettingsError`] when the directory cannot be made or watched.
pub fn watch_file(dir: &Path, file: &OsStr) -> Result<(FileWatcher, Settled), SettingsError> {
    std::fs::create_dir_all(dir).map_err(|source| SettingsError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    let (signal, signals) = latest::channel(());
    let file = file.to_owned();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res
            && touches_the_file(&event, &file)
        {
            // The receiver is gone once the watch task ended; there is nobody left to signal.
            let _ = signal.send(());
        }
    })?;
    watcher.watch(dir, RecursiveMode::NonRecursive)?;
    Ok((FileWatcher { _watcher: watcher }, Settled { signals }))
}

impl<D: SettingsDoc> Watch<D> {
    /// Wait for the next settled change: the whole re-read file, never a partial one, with what
    /// was refused or dropped in it. A file that is not valid text keeps the last good value and
    /// reports itself invalid. `None` once the watch has stopped.
    pub async fn changed(&mut self) -> Option<Loaded<D>> {
        self.changes.changed().await
    }

    /// The file as last read.
    #[must_use]
    pub fn current(&self) -> D {
        self.changes.latest().value
    }

    /// Whether the directory is being watched.
    #[must_use]
    pub fn state(&self) -> &WatchState {
        &self.state
    }
}

impl<D: SettingsDoc + Send> Watch<D> {
    pub(crate) fn start(store: &Store, spawn: &dyn Spawner) -> Watch<D> {
        // The watcher starts before the first read, so a write that lands in between is seen as
        // an event and re-read, not missed until the next edit.
        let armed = store
            .require_dir()
            .and_then(|dir| watch_file(&dir, OsStr::new(D::FILE.0)));
        let initial = store.load::<D>();
        let (out, changes) = latest::channel(initial.clone());
        match armed {
            Ok((watcher, signals)) => {
                let path = store.path::<D>().unwrap_or_default();
                spawn.spawn(Box::pin(settle(path, initial.value, signals, out)));
                Watch {
                    changes,
                    state: WatchState::Live,
                    _watcher: Some(watcher),
                }
            }
            Err(error) => Watch {
                changes,
                state: WatchState::Blind {
                    reason: error.to_string(),
                },
                _watcher: None,
            },
        }
    }
}

/// Whether `event` is a write to the file `name` itself: not the temp file the atomic writer
/// (or an editor) writes and renames away, not an unrelated file in the same directory (another
/// watched settings file included), and not an `Access` event: opening the file to read it
/// (which every re-read this watch does, and every editor's own load, triggers) is not a change
/// and must not re-arm the debounce, or a watcher re-reading the file becomes a change that
/// makes it re-read the file forever.
fn touches_the_file(event: &notify::Event, name: &OsStr) -> bool {
    !event.kind.is_access()
        && event
            .paths
            .iter()
            .any(|path| path.file_name() == Some(name))
}

/// Re-read `path` after every settled burst of events and publish it, until the watcher or the
/// receiver is dropped. `good` is the last value that read cleanly.
// The debounce is a wall-clock wait on the spawner's own thread: `ui_core::time::clock::sleep` follows the clock
// installed on its caller's thread and is not `Send`, and a task on a runtime has neither.
async fn settle<D: SettingsDoc + Send>(
    path: PathBuf,
    mut good: D,
    mut signals: Settled,
    out: Sender<Loaded<D>>,
) {
    while signals.settled().await.is_some() {
        let (loaded, mut refused) = reread(&path, &mut good);
        if out.send(loaded).is_err() {
            return;
        }
        // A refused read raises no file event, so ask again with a growing wait, unless the
        // file changes meanwhile (then the outer loop reads it). Only the answer that ends the
        // refusal is published; the same refusal again tells nobody anything new.
        let mut wait = RETRY_FIRST;
        for _ in 0..RETRIES {
            if !refused {
                break;
            }
            futures_timer::Delay::new(wait).await;
            if signals.has_changed() {
                break;
            }
            wait = wait.saturating_mul(2);
            let (again, next) = reread(&path, &mut good);
            refused = next;
            if !refused && out.send(again).is_err() {
                return;
            }
        }
    }
}

/// The file at `path` as the watch publishes it, updating `good` when it read cleanly or is gone,
/// and whether the system refused it (nothing will announce when it stops, so read it again later;
/// a file that was read, is gone or is wrongly written needs no retry, as an edit announces its fix).
fn reread<D: SettingsDoc + Send>(path: &Path, good: &mut D) -> (Loaded<D>, bool) {
    match file_text(path, SETTINGS_CAP) {
        FileText::Text(text) => match read_within::<D>(&text, D::FORMAT, &D::limits()) {
            Read::Garbled { reason } => (Loaded::garbled(good.clone(), reason), false),
            Read::Loaded(loaded) => {
                *good = loaded.value.clone();
                (loaded, false)
            }
        },
        FileText::Unusable(reason) => (Loaded::garbled(good.clone(), reason), true),
        FileText::Missing => {
            *good = D::default();
            (Loaded::default(), false)
        }
    }
}

/// A live watch on the file a [`Locator`] finds, for a daemon whose settings are a typed value
/// `T` read again after every settled change. Dropping it stops the watch.
#[derive(Debug)]
pub struct LocatedWatch<T> {
    changes: Receiver<Loaded<T>>,
    state: WatchState,
    _watcher: Option<FileWatcher>,
}

impl<T: Clone + Send + 'static> LocatedWatch<T> {
    /// Reads the file now with `read`, and watches the directory that holds it.
    ///
    /// The second half is the task that reads again after every settled change and publishes it;
    /// the caller runs it on its runtime (`tokio::spawn`). It is `None` when the directory could
    /// not be watched ([`WatchState::Blind`]): the value stays as first read.
    #[must_use]
    pub fn start(
        locator: &Locator,
        read: impl Fn() -> Loaded<T> + Send + 'static,
    ) -> (Self, Option<impl Future<Output = ()> + Send + 'static>) {
        let initial = read();
        let (out, changes) = latest::channel(initial);
        let armed = locator
            .watch_dir()
            .zip(locator.file_name())
            .ok_or_else(|| SettingsError::NoConfigDir {
                app: "this program",
            })
            .and_then(|(dir, file)| watch_file(&dir, file));
        match armed {
            Ok((watcher, signals)) => (
                Self {
                    changes,
                    state: WatchState::Live,
                    _watcher: Some(watcher),
                },
                Some(settle_located(signals, read, out)),
            ),
            Err(error) => (
                Self {
                    changes,
                    state: WatchState::Blind {
                        reason: error.to_string(),
                    },
                    _watcher: None,
                },
                None,
            ),
        }
    }

    /// The file as last read.
    #[must_use]
    pub fn current(&self) -> Loaded<T> {
        self.changes.latest()
    }

    /// Whether the directory is being watched.
    #[must_use]
    pub fn state(&self) -> &WatchState {
        &self.state
    }

    /// Waits for the next settled change: the whole file read again. `None` once the watch has
    /// stopped (or was never live).
    pub async fn changed(&mut self) -> Option<Loaded<T>> {
        self.changes.changed().await
    }
}

/// Reads again after every settled burst of events and publishes, until the watch is dropped.
async fn settle_located<T: Clone>(
    mut signals: Settled,
    read: impl Fn() -> Loaded<T>,
    out: Sender<Loaded<T>>,
) {
    while signals.settled().await.is_some() {
        if out.send(read()).is_err() {
            return;
        }
    }
}

/// Watches `dir` for writes to the file named `name` on a thread of its own, for a program that
/// runs no async runtime: `settled` is called after every burst of events has been quiet for
/// [`DEBOUNCE`], and ends the thread by returning `false`. Returns the watcher, which must be kept
/// alive; dropping it ends the thread. The directory must exist.
///
/// # Errors
///
/// Returns the `notify::Error` when `dir` cannot be watched or the thread cannot start.
pub fn watch_file_on_thread(
    dir: &Path,
    name: &str,
    thread_name: String,
    mut settled: impl FnMut() -> bool + Send + 'static,
) -> notify::Result<RecommendedWatcher> {
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    let file = OsString::from(name);
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res
            && touches_the_file(&event, &file)
        {
            // The thread is gone once the watch ended; nobody is left to tell.
            let _ = tx.send(());
        }
    })?;
    watcher.watch(dir, RecursiveMode::NonRecursive)?;
    std::thread::Builder::new()
        .name(thread_name)
        .spawn(move || {
            while rx.recv().is_ok() {
                // Every further event restarts the window: a rename can fire more than one.
                loop {
                    match rx.recv_timeout(DEBOUNCE) {
                        Ok(()) => {}
                        Err(RecvTimeoutError::Timeout) => break,
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                }
                if !settled() {
                    return;
                }
            }
        })
        .map_err(notify::Error::io)?;
    Ok(watcher)
}

#[cfg(test)]
mod tests {
    use super::touches_the_file;
    use notify::event::{AccessKind, CreateKind, EventKind};
    use std::ffi::OsStr;
    use std::path::PathBuf;

    fn event(kind: EventKind, path: &str) -> notify::Event {
        notify::Event::new(kind).add_path(PathBuf::from(path))
    }

    #[test]
    fn only_a_write_to_the_named_file_matters() {
        let name = OsStr::new("settings.toml");
        let write = EventKind::Create(CreateKind::File);
        for (path, want) in [
            ("/c/bardo/shell/settings.toml", true),
            ("/c/bardo/shell/other.toml", false),
            ("/c/bardo/shell/settings.toml.swp", false),
            ("/c/bardo/shell/.settings.toml.tmp", false),
        ] {
            assert_eq!(touches_the_file(&event(write, path), name), want, "{path}");
        }
        let read = EventKind::Access(AccessKind::Any);
        assert!(!touches_the_file(
            &event(read, "/c/bardo/shell/settings.toml"),
            name
        ));
    }
}
