//! The renderer's clipboards: [`System`], the desktop's, and [`Memory`], the harness's. The
//! [`Clipboard`] trait, the typed file list and the calls a component makes are `ui_host_hooks::clipboard`'s.
//!
//! The system clipboard is another program's, asked at the moment of the paste, so a read does
//! not run on the UI thread: it runs on a worker with a short wait and a size cap (`read`), and
//! an owner that is slow, silent or enormous is read as an empty clipboard. quire asks for the
//! selection with the window's own connection and does not use the privileged data-control
//! protocol (FINDINGS "Clipboard"), which would let a client read the clipboard unfocused.

mod files;
mod primary;
mod read;

use self::read::Reader;
use crate::memory_shell::MemoryShell;
use blitz_traits::shell::ShellProvider;
use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use ui_components::host::pasted::Pasted;
use ui_host_hooks::clipboard::{
    Clipboard, ClipboardError, ClipboardFiles, ClipboardTarget, pasted,
};

/// The desktop's clipboard, through the window's shell: the shell is reached once the window's
/// document exists (`crate::install`).
#[derive(Default)]
pub struct System {
    shell: RefCell<Option<Arc<dyn ShellProvider>>>,
    /// Runs each read off the UI thread, under a time and a size limit.
    reader: Reader,
}

impl std::fmt::Debug for System {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("System")
    }
}

impl System {
    /// Reach the clipboard through `shell` from now on.
    pub(crate) fn reach(&self, shell: Arc<dyn ShellProvider>) {
        self.shell.replace(Some(shell));
    }

    fn shell(&self) -> Option<Arc<dyn ShellProvider>> {
        self.shell.borrow().clone()
    }
}

impl Clipboard for System {
    fn read_from(&self, target: ClipboardTarget) -> Option<String> {
        match target {
            ClipboardTarget::Clipboard => {
                let shell = self.shell()?;
                self.reader.text(move || shell.get_clipboard_text().ok())
            }
            ClipboardTarget::Primary => self.reader.text(primary::read),
        }
    }

    fn write_to(&self, target: ClipboardTarget, text: &str) -> Result<(), ClipboardError> {
        let text = text.to_owned();
        match target {
            ClipboardTarget::Clipboard => {
                let shell = self.shell().ok_or(ClipboardError::NoShell)?;
                guarded(move || match shell.set_clipboard_text(text) {
                    Ok(()) => Ok(()),
                    Err(_refused) => Err(ClipboardError::Unavailable),
                })
            }
            ClipboardTarget::Primary => guarded(move || primary::write(&text)),
        }
    }

    fn read_files(&self) -> Option<ClipboardFiles> {
        let list = self.reader.text(files::read_system);
        list.as_deref()
            .and_then(ClipboardFiles::from_uri_list)
            .or_else(|| {
                self.read_text()
                    .and_then(|text| ClipboardFiles::from_text(&text))
            })
    }

    fn write_files(&self, files: &ClipboardFiles) -> Result<(), ClipboardError> {
        let paths = files.paths.clone();
        guarded(move || files::write_system(&paths))
    }

    fn read_html(&self) -> Option<Pasted> {
        let html = self.reader.text(|| {
            arboard::Clipboard::new()
                .and_then(|mut clipboard| clipboard.get().html())
                .ok()
        });
        pasted(html, self.read_text())
    }
}

/// The harness's clipboard: text and HTML in a shell's memory.
#[derive(Debug)]
pub struct Memory(pub Arc<MemoryShell>);

impl Clipboard for Memory {
    fn read_from(&self, target: ClipboardTarget) -> Option<String> {
        match target {
            ClipboardTarget::Clipboard => self.0.text(),
            ClipboardTarget::Primary => self.0.primary_text(),
        }
    }

    fn write_to(&self, target: ClipboardTarget, text: &str) -> Result<(), ClipboardError> {
        match target {
            ClipboardTarget::Clipboard => self.0.put(text.to_owned()),
            ClipboardTarget::Primary => self.0.put_primary(text.to_owned()),
        }
        Ok(())
    }

    fn read_files(&self) -> Option<ClipboardFiles> {
        self.0.files().or_else(|| {
            self.0
                .text()
                .and_then(|text| ClipboardFiles::from_text(&text))
        })
    }

    fn write_files(&self, files: &ClipboardFiles) -> Result<(), ClipboardError> {
        self.0.put_files(files);
        Ok(())
    }

    fn read_html(&self) -> Option<Pasted> {
        pasted(self.0.html(), self.0.text())
    }
}

/// Run a clipboard write, reading a panic as no answer: blitz-shell unwraps
/// `arboard::Clipboard::new()`, which fails on a session with no clipboard at all. (Reads go
/// through [`Reader`], which runs them on a worker thread, where a panic ends only that thread.)
fn guarded(call: impl FnOnce() -> Result<(), ClipboardError>) -> Result<(), ClipboardError> {
    catch_unwind(AssertUnwindSafe(call)).unwrap_or(Err(ClipboardError::Unavailable))
}
