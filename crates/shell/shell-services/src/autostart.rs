//! Open at Login: the freedesktop autostart entry `$XDG_CONFIG_HOME/autostart/<app id>.desktop`.
//!
//! Turning it on copies the app's own desktop entry there, with one line added under
//! `[Desktop Entry]` that says bardo-shell wrote it ([`MARK_LINE`]). Turning it off deletes the file only
//! when that line is in it: a file the person or another program put there is shown as on and
//! left alone. The state is always read from the folder, never remembered.

use std::path::{Path, PathBuf};

use shell_launcher::AppId;

use crate::env::Dirs;
use crate::error::ServiceError;

/// The key that marks a file bardo-shell wrote.
pub const MARK_KEY: &str = "X-Bardo-Autostart";

/// The line added to a copied entry.
const MARK_LINE: &str = "X-Bardo-Autostart=true";

/// The group header the mark goes under.
const HEADER: &str = "[Desktop Entry]";

/// Whether an app starts at login, as the autostart folder says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Login {
    /// No autostart entry for the app.
    Off,
    /// An entry bardo-shell wrote: bardo-shell may remove it.
    OurEntry,
    /// An entry someone else wrote: shown as on, never removed here.
    OthersEntry,
}

impl Login {
    /// Whether the app starts at login.
    #[must_use]
    pub fn is_on(self) -> bool {
        self != Login::Off
    }
}

/// `app`'s autostart file under `dirs`' config directory.
#[must_use]
pub fn file_of(dirs: &Dirs, app: &AppId) -> PathBuf {
    dirs.config
        .join("autostart")
        .join(format!("{}.desktop", app.0))
}

/// Whether `text` carries bardo-shell's mark.
#[must_use]
pub fn is_marked(text: &str) -> bool {
    text.lines().any(|line| line.trim() == MARK_LINE)
}

/// `entry` with the mark under its `[Desktop Entry]` header; none when it has no such header.
#[must_use]
pub fn with_mark(entry: &str) -> Option<String> {
    let mut out = String::with_capacity(entry.len() + MARK_LINE.len() + 2);
    let mut placed = false;
    for line in entry.split_inclusive('\n') {
        out.push_str(line);
        if !placed && line.trim_end() == HEADER {
            if !line.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(MARK_LINE);
            out.push('\n');
            placed = true;
        }
    }
    placed.then_some(out)
}

/// What the folder says about `app`.
#[must_use]
pub fn login_of(dirs: &Dirs, app: &AppId) -> Login {
    match std::fs::read_to_string(file_of(dirs, app)) {
        Ok(text) if is_marked(&text) => Login::OurEntry,
        Ok(_) => Login::OthersEntry,
        // An unreadable file is somebody's: it is there, and not ours to remove.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Login::Off,
        Err(_) => Login::OthersEntry,
    }
}

/// Start `app` at login: copy its desktop entry at `entry` into the autostart folder. An entry
/// already there, ours or not, stays as it is.
pub fn turn_on(dirs: &Dirs, app: &AppId, entry: &Path) -> Result<(), ServiceError> {
    if login_of(dirs, app).is_on() {
        return Ok(());
    }
    let target = file_of(dirs, app);
    let text = std::fs::read_to_string(entry).map_err(|e| io(entry, e))?;
    let marked = with_mark(&text).ok_or_else(|| {
        io(
            entry,
            std::io::Error::new(std::io::ErrorKind::InvalidData, "no [Desktop Entry] group"),
        )
    })?;
    if let Some(folder) = target.parent() {
        std::fs::create_dir_all(folder).map_err(|e| io(folder, e))?;
    }
    std::fs::write(&target, marked).map_err(|e| io(&target, e))
}

/// Stop starting `app` at login: remove the entry when bardo-shell wrote it, and only then.
pub fn turn_off(dirs: &Dirs, app: &AppId) -> Result<(), ServiceError> {
    match login_of(dirs, app) {
        Login::OurEntry => {
            let target = file_of(dirs, app);
            std::fs::remove_file(&target).map_err(|e| io(&target, e))
        }
        Login::Off | Login::OthersEntry => Ok(()),
    }
}

/// [`turn_on`] on a thread of its own; a failure is logged, there being nobody to tell.
pub fn turn_on_in_background(dirs: Dirs, app: AppId, entry: PathBuf) {
    std::thread::spawn(move || {
        if let Err(error) = turn_on(&dirs, &app, &entry) {
            eprintln!("bardo-shell: Open at Login: {error}");
        }
    });
}

/// [`turn_off`] on a thread of its own; a failure is logged.
pub fn turn_off_in_background(dirs: Dirs, app: AppId) {
    std::thread::spawn(move || {
        if let Err(error) = turn_off(&dirs, &app) {
            eprintln!("bardo-shell: Open at Login: {error}");
        }
    });
}

fn io(path: &Path, source: std::io::Error) -> ServiceError {
    ServiceError::Io {
        path: path.to_path_buf(),
        source,
    }
}
