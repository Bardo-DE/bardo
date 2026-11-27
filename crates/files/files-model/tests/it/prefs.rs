//! Per-folder views and sorts, and the effects that remember them.

use files_core::{FolderSettings, Sort, SortKey, SortOrder, ViewMode};
use files_model::{Effect, Input};

use crate::common::*;

fn descending_icons() -> FolderSettings {
    FolderSettings {
        view: ViewMode::Icons,
        sort: Sort {
            key: SortKey::Name,
            order: SortOrder::Descending,
        },
    }
}

#[test]
fn loaded_prefs_for_this_folder_change_the_view_and_the_order() {
    let browser = loaded(vec![file("a"), file("b")]);
    let (browser, effects) = browser.step(
        Input::FolderSettingsLoaded(home(), descending_icons()),
        at(0),
    );
    assert_eq!(effects, [] as [files_model::Effect; 0]);
    assert_eq!(browser.prefs(), descending_icons());
    assert_eq!(shown(&browser), vec!["b", "a"]);
}

#[test]
fn prefs_for_a_place_already_left_are_ignored() {
    let browser = loaded(vec![file("a")]);
    let (after, _) = browser.clone().step(
        Input::FolderSettingsLoaded(dir("/elsewhere"), descending_icons()),
        at(0),
    );
    assert_eq!(after, browser);
}

#[test]
fn changing_the_view_or_sort_saves_it_for_the_folder() {
    let browser = loaded(vec![file("a"), file("b")]);
    let descending = Sort {
        key: SortKey::Name,
        order: SortOrder::Descending,
    };
    let (browser, effects) = browser.step(Input::SetView(ViewMode::Icons), at(0));
    let mut want = FolderSettings {
        view: ViewMode::Icons,
        ..FolderSettings::default()
    };
    assert_eq!(effects, vec![Effect::SaveFolderSettings(home(), want)]);
    let (browser, effects) = browser.step(Input::SetSort(descending), at(0));
    want.sort = descending;
    assert_eq!(effects, vec![Effect::SaveFolderSettings(home(), want)]);
    assert_eq!(shown(&browser), vec!["b", "a"]);
}

#[test]
fn a_new_folder_starts_with_defaults_until_its_prefs_arrive() {
    let (browser, _) = run(
        loaded(vec![]),
        vec![Input::FolderSettingsLoaded(home(), descending_icons())],
    );
    let (browser, effects) = browser.step(Input::Go(dir("/x")), at(0));
    assert_eq!(browser.prefs(), FolderSettings::default());
    assert!(effects.contains(&Effect::LoadFolderSettings(dir("/x"))));
}

#[test]
fn resorting_keeps_the_selection() {
    let browser = loaded(vec![file("a"), file("b")]);
    let (browser, _) = run(
        browser,
        vec![
            Input::Click(name("a"), files_model::Modifier::None),
            Input::SetSort(Sort {
                key: SortKey::Name,
                order: SortOrder::Descending,
            }),
        ],
    );
    assert_eq!(selected(&browser), vec!["a"]);
}
