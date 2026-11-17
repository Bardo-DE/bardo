//! The system clipboard, for the app: `write_text` behind a "Copy address" item, `read_text` behind
//! a paste.
//!
//! Copy and paste inside a text field need nothing from the app: Blitz handles Ctrl+C/X/V there
//! through the same shell provider.
//!
//! The clipboard is a [`Clipboard`], reached through a context every document the renderer
//! hosts provides: `ui_blitz::clipboard::System`, the window's winit shell, or
//! `ui_blitz::clipboard::Memory`, the harness's, so a test copies and pastes without touching the
//! desktop's clipboard. The renderer's implementations are `ui_blitz`'s; the trait and the calls
//! a component makes are here.
//!
//! Files ride the same clipboard: `write_files` / `read_files` take a typed [`ClipboardFiles`]
//! (an operation and paths). The system clipboard carries them as `text/uri-list`, so another
//! app sees a copy; a cut is the app's own state. The harness's memory clipboard holds both
//! `text/uri-list` and `x-special/gnome-copied-files`.
//!
//! `text/html` (an edit surface's paste) is not in blitz's `ShellProvider`, so the window reads it
//! from `arboard` itself and the harness from its memory clipboard's HTML slot.
//!
//! The system clipboard is another program's, asked at the moment of the paste, so a read does
//! not run on the UI thread: it runs on a worker with a short wait and a size cap (`read`), and
//! an owner that is slow, silent or enormous is read as an empty clipboard. Bardo asks for the
//! selection with the window's own connection and does not use the privileged data-control
//! protocol (FINDINGS "Clipboard"), which would let a client read the clipboard unfocused.

mod files;

pub use self::files::{ClipboardFiles, FileOp};

use dioxus::prelude::*;
use std::rc::Rc;
use ui_components::host::pasted::Pasted;

/// Why the clipboard was not read or written.
#[derive(Debug, thiserror::Error)]
pub enum ClipboardError {
    /// Not called inside a document the renderer hosts (`launch`, the harness): there is no clipboard to
    /// reach. Call it from a handler.
    #[error("no document to reach a clipboard through")]
    NoHost,
    /// The system clipboard refused, or holds no text.
    #[error("the clipboard is unavailable or holds no text")]
    Unavailable,
    /// The system clipboard could not be opened or written.
    #[error("the system clipboard could not be written")]
    Refused(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// The document has no window shell to reach the system clipboard through: a root
    /// [`provide_host`](crate::provide_host) wired by itself, not one `launch` built. Nothing was
    /// written.
    #[error("this document has no window shell to reach the system clipboard through")]
    NoShell,
}

/// Which selection a read or write reaches.
///
/// The regular clipboard is what Copy and Paste use. The primary selection is the text last
/// selected, pasted by a middle click, on X11 and (where the compositor has it) Wayland; a
/// terminal's OSC 52 `p` writes it. A platform with no primary selection answers
/// [`ClipboardError::Unavailable`] to a write and nothing to a read: it never falls back to the
/// regular clipboard, which would replace what the person copied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ClipboardTarget {
    /// The regular clipboard.
    #[default]
    Clipboard,
    /// The primary selection.
    Primary,
}

/// A clipboard a document's app and its edit surfaces read and write.
pub trait Clipboard {
    /// The text on `target`.
    fn read_from(&self, target: ClipboardTarget) -> Option<String>;
    /// Put `text` on `target`, or say why it was not put.
    ///
    /// # Errors
    ///
    /// Returns `ClipboardError` when this is not called inside a document, the document has no
    /// window shell, or the system clipboard refuses.
    fn write_to(&self, target: ClipboardTarget, text: &str) -> Result<(), ClipboardError>;
    /// The text on the regular clipboard.
    fn read_text(&self) -> Option<String> {
        self.read_from(ClipboardTarget::Clipboard)
    }
    /// What a paste inserts: the HTML with its plain text when the clipboard has HTML, else the
    /// text.
    fn read_html(&self) -> Option<Pasted>;
    /// Put `text` on the regular clipboard, or say why it was not put.
    ///
    /// # Errors
    ///
    /// Returns `ClipboardError` when this is not called inside a document, the document has no
    /// window shell, or the system clipboard refuses.
    fn write_text(&self, text: &str) -> Result<(), ClipboardError> {
        self.write_to(ClipboardTarget::Clipboard, text)
    }
    /// The files on the regular clipboard: a file list, else text that is only paths or URIs.
    fn read_files(&self) -> Option<ClipboardFiles> {
        self.read_text()
            .and_then(|text| ClipboardFiles::from_text(&text))
    }
    /// Put `files` on the regular clipboard, or say why it was not put. The default writes their
    /// paths as text.
    ///
    /// # Errors
    ///
    /// Returns `ClipboardError` when this is not called inside a document, the document has no
    /// window shell, or the system clipboard refuses.
    fn write_files(&self, files: &ClipboardFiles) -> Result<(), ClipboardError> {
        self.write_text(&files.to_text())
    }
}

/// Put `text` on the clipboard. Call it from a handler inside a document the renderer hosts.
///
/// # Errors
/// [`ClipboardError::NoShell`] in a root [`provide_host`](crate::provide_host) wired on its own
/// (no window shell to write through), [`ClipboardError::Unavailable`] when the system
/// clipboard refuses.
pub fn write_text(text: &str) -> Result<(), ClipboardError> {
    host()?.write_text(text)
}

/// Put `text` on `target`. Call it from a handler inside a document the renderer hosts.
///
/// # Errors
/// As [`write_text`]; and [`ClipboardError::Unavailable`] for [`ClipboardTarget::Primary`] where
/// the platform has no primary selection or its compositor does not offer one.
pub fn write_to(target: ClipboardTarget, text: &str) -> Result<(), ClipboardError> {
    host()?.write_to(target, text)
}

/// The text on `target`. Call it from a handler inside a document the renderer hosts.
///
/// # Errors
///
/// Returns `ClipboardError` when this is not called inside a document, or the system clipboard
/// refuses or holds no text.
pub fn read_from(target: ClipboardTarget) -> Result<String, ClipboardError> {
    host()?.read_from(target).ok_or(ClipboardError::Unavailable)
}

/// The text on the clipboard. Call it from a handler inside a document the renderer hosts.
///
/// # Errors
///
/// Returns `ClipboardError` when this is not called inside a document, or the system clipboard
/// refuses or holds no text.
pub fn read_text() -> Result<String, ClipboardError> {
    host()?.read_text().ok_or(ClipboardError::Unavailable)
}

/// Put `files` on the clipboard. Call it from a handler inside a document the renderer hosts. Another app
/// sees a copy of these files; a cut is the app's own state (see `CONSUMING.md`).
///
/// # Errors
/// As [`write_text`].
pub fn write_files(files: &ClipboardFiles) -> Result<(), ClipboardError> {
    host()?.write_files(files)
}

/// The files on the clipboard. Call it from a handler inside a document the renderer hosts.
///
/// # Errors
///
/// Returns `ClipboardError` when this is not called inside a document, or the system clipboard
/// refuses or holds no files.
pub fn read_files() -> Result<ClipboardFiles, ClipboardError> {
    host()?.read_files().ok_or(ClipboardError::Unavailable)
}

/// The clipboard's `text/html`. Call it from a handler inside a document the renderer hosts.
///
/// # Errors
///
/// Returns `ClipboardError` when this is not called inside a document, or the system clipboard
/// refuses or holds no HTML.
pub fn read_html() -> Result<String, ClipboardError> {
    match host()?.read_html() {
        Some(Pasted::Html { html, .. }) => Ok(html),
        Some(Pasted::Text(_)) | None => Err(ClipboardError::Unavailable),
    }
}

/// What a paste inserts: the HTML with its plain text when there is HTML, else the text.
#[must_use]
pub fn pasted(html: Option<String>, text: Option<String>) -> Option<Pasted> {
    match (html.filter(|html| !html.is_empty()), text) {
        (Some(html), text) => Some(Pasted::Html {
            html,
            text: text.unwrap_or_default(),
        }),
        (None, Some(text)) => Some(Pasted::Text(text)),
        (None, None) => None,
    }
}

/// The calling document's clipboard; outside any Dioxus runtime there is none (asking would
/// panic).
fn host() -> Result<Rc<dyn Clipboard>, ClipboardError> {
    dioxus::core::Runtime::try_current()
        .and_then(|_| try_consume_context::<Rc<dyn Clipboard>>())
        .ok_or(ClipboardError::NoHost)
}

impl ClipboardError {
    /// The system clipboard refused with `error`.
    pub fn refused(error: impl std::error::Error + Send + Sync + 'static) -> ClipboardError {
        ClipboardError::Refused(Box::new(error))
    }
}
