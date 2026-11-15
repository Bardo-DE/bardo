//! Tabs in the window: ⌘T opens one, the tab bar shows from two on, each tab keeps its folder.

use std::sync::Arc;
use std::time::Duration;

use ds::prelude::ShortcutKey;
use ds_harness::harness::settle_until;
use ds_harness::{Harness, Input};
use tempfile::TempDir;

use super::support::{RecordingOpener, harness, launch, scratch_folder};

/// The tab bar's label, as it is in the markup.
const BAR: &str = "[aria-label=\"Tabs\"]";

/// A window on the scratch folder, with what it needs kept alive.
struct Rig {
    harness: Harness,
    _dir: TempDir,
    _config: TempDir,
}

fn open() -> Rig {
    let dir = scratch_folder();
    let config = tempfile::tempdir().unwrap();
    let opener = Arc::new(RecordingOpener::default());
    let mut harness = harness(launch(dir.path(), config.path(), &opener));
    settle_until(&mut harness, |h| h.html().contains("zebra.png"));
    Rig {
        harness,
        _dir: dir,
        _config: config,
    }
}

impl Rig {
    fn command(&mut self, key: ShortcutKey) {
        self.harness.send(Input::chord(&[ShortcutKey::Super], key));
        self.harness.advance(Duration::from_millis(50));
    }

    fn shows(&mut self, text: &'static str) {
        settle_until(&mut self.harness, |h| h.html().contains(text));
    }

    fn bar(&self) -> bool {
        self.harness.centre(BAR).is_some()
    }
}

#[test]
fn command_t_opens_a_tab_on_the_same_folder_and_the_bar_shows_from_two_tabs() {
    let mut rig = open();
    assert!(!rig.bar(), "one tab shows no bar");
    rig.command(ShortcutKey::Char('t'));
    settle_until(&mut rig.harness, |h| h.centre(BAR).is_some());
    rig.shows("zebra.png");
    // Both tabs are titled with the folder's name.
    let name = rig
        ._dir
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let html = rig.harness.html();
    assert!(
        html.matches(&format!(">{name}<")).count() >= 2,
        "two tabs are not titled {name}:\n{html}"
    );
}
