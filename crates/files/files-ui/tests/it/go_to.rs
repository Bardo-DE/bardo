//! Go to Folder in the window: Shift+Command+G, a path typed, Return to go and Escape to stay.

use std::sync::Arc;
use std::time::Duration;

use files_core::FolderPath;
use tempfile::TempDir;
use ui_components::prelude::ShortcutKey;
use ui_harness::harness::settle_until;
use ui_harness::{Harness, Input};

use super::support::{RecordingOpener, harness, launch};

/// A window on a scratch folder holding `notes.txt` and `docs/inner.txt`, whose home is that folder.
struct Rig {
    harness: Harness,
    dir: TempDir,
    _config: TempDir,
}

fn open() -> Rig {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("notes.txt"), "hello").unwrap();
    std::fs::create_dir(dir.path().join("docs")).unwrap();
    std::fs::write(dir.path().join("docs").join("inner.txt"), "x").unwrap();
    let config = tempfile::tempdir().unwrap();
    let opener = Arc::new(RecordingOpener::default());
    let home = FolderPath::new(dir.path()).unwrap();
    let mut harness = harness(launch(dir.path(), config.path(), &opener).with_home(home));
    settle_until(&mut harness, |h| h.html().contains("notes.txt"));
    Rig {
        harness,
        dir,
        _config: config,
    }
}

impl Rig {
    fn key(&mut self, key: ShortcutKey) {
        self.harness.send(Input::key(key));
        self.harness.advance(Duration::from_millis(30));
    }

    fn type_text(&mut self, text: &str) {
        for c in text.chars() {
            self.key(ShortcutKey::Char(c));
        }
    }

    /// Shift+Command+G, and the sheet is up with the folder shown as its text.
    fn ask(&mut self) {
        self.harness.send(Input::chord(
            &[ShortcutKey::Super, ShortcutKey::Shift],
            ShortcutKey::Char('g'),
        ));
        settle_until(&mut self.harness, |h| h.count(".ds-text-field") > 0);
    }

    /// Empties the field, whatever the caret was at.
    fn clear(&mut self) {
        self.key(ShortcutKey::End);
        for _ in 0..self.dir.path().to_string_lossy().chars().count() {
            self.key(ShortcutKey::Backspace);
        }
    }

    fn sheet_is_up(&self) -> bool {
        self.harness.count(".ds-text-field") > 0
    }
}

#[test]
fn tilde_is_the_home_folder() {
    let mut rig = open();
    rig.ask();
    rig.clear();
    rig.type_text("~/docs");
    rig.key(ShortcutKey::Enter);
    settle_until(&mut rig.harness, |h| h.html().contains("inner.txt"));
    assert!(!rig.sheet_is_up());
}
