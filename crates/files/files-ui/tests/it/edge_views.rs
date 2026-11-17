//! The edge's seams for what the person sees of an item: the remembered view of a folder, Get Info's
//! facts and Quick Look's preview.

use std::sync::Arc;

use files_core::{ByteSize, FolderPath, FolderSettings, ItemRef, Location, ViewMode};
use files_fs::FolderSettingsStore;
use files_model::{Effect, FolderListError, Input, TabId};
use files_ui::edge::{Edge, Heard, Seams};

use super::edge::{file, folder, name, next, rig};
use super::support::{RecordingOpener, scratch_folder};

#[test]
fn a_folders_view_is_remembered_and_loaded_in_the_order_asked() {
    let mut rig = rig();
    let here = folder("/p");
    let place = Location::Folder(here.clone());
    let prefs = FolderSettings {
        view: ViewMode::Icons,
        ..FolderSettings::default()
    };
    // Never changed: the defaults, and saving says nothing.
    rig.edge.run_in(
        TabId::FIRST,
        vec![Effect::LoadFolderSettings(place.clone())],
    );
    assert_eq!(
        next(&mut rig.inbox),
        Heard::Tab(
            TabId::FIRST,
            Input::FolderSettingsLoaded(place.clone(), FolderSettings::default())
        )
    );
    rig.edge.run_in(
        TabId::FIRST,
        vec![
            Effect::SaveFolderSettings(place.clone(), prefs),
            Effect::LoadFolderSettings(place.clone()),
        ],
    );
    assert_eq!(
        next(&mut rig.inbox),
        Heard::Tab(
            TabId::FIRST,
            Input::FolderSettingsLoaded(place.clone(), prefs)
        )
    );
    assert_eq!(rig.prefs.load(&here), prefs);
    // The Trash is shown the same way every time.
    rig.edge.run_in(
        TabId::FIRST,
        vec![
            Effect::SaveFolderSettings(Location::Trash, prefs),
            Effect::LoadFolderSettings(Location::Trash),
        ],
    );
    assert_eq!(
        next(&mut rig.inbox),
        Heard::Tab(
            TabId::FIRST,
            Input::FolderSettingsLoaded(Location::Trash, FolderSettings::default())
        )
    );
}

#[test]
fn get_info_reads_the_facts_or_says_why_not() {
    let mut rig = rig();
    let here = folder("/p");
    rig.lister.set(here.clone(), Ok(vec![file("a")]));
    let found = ItemRef::new(here.clone(), name("a"));
    let missing = ItemRef::new(here, name("zzz"));
    rig.edge
        .run_in(TabId::FIRST, vec![Effect::ReadInfo(found.clone())]);
    let Heard::Tab(TabId::FIRST, Input::InfoRead(got, Ok(info))) = next(&mut rig.inbox) else {
        panic!("no facts");
    };
    assert_eq!(got, found);
    assert_eq!(info.size(), Some(ByteSize(3)));
    rig.edge
        .run_in(TabId::FIRST, vec![Effect::ReadInfo(missing.clone())]);
    assert_eq!(
        next(&mut rig.inbox),
        Heard::Tab(
            TabId::FIRST,
            Input::InfoRead(missing, Err(FolderListError::NotFound))
        )
    );
}

#[test]
fn quick_look_gets_a_preview_of_the_file() {
    let dir = scratch_folder();
    let (edge, mut inbox) = Edge::new(Seams::disk(Arc::new(RecordingOpener::default())));
    let item = ItemRef::new(FolderPath::new(dir.path()).unwrap(), name("notes.txt"));
    edge.run_in(TabId::FIRST, vec![Effect::Look(item.clone())]);
    let heard = loop {
        match next(&mut inbox) {
            Heard::Looked(got, peeked) => break (got, peeked),
            // The folder's view is asked for as a window opens; only the preview matters here.
            Heard::Tab(..)
            | Heard::All(_)
            | Heard::CloseWindow
            | Heard::Told(_)
            | Heard::Announced(_)
            | Heard::Thumbed(..) => {}
        }
    };
    assert_eq!(heard.0, item);
    assert_eq!(heard.1.name, "notes.txt");
}
