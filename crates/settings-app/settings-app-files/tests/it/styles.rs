//! The style files the person wrote, over a scratch settings folder.

use settings_app_files::{ConfigHome, DataHome, StyleSheet, published_classes, read_style_sheets};
use settings_app_model::StyleOwner;
use settings_app_testkit::Scratch;
use std::fs;

fn write(scratch: &Scratch, relative: &str, text: &str) {
    let path = scratch.join(relative);
    fs::create_dir_all(path.parent().expect("a file has a parent")).expect("mkdir");
    fs::write(path, text).expect("write");
}

fn sheet(owner: StyleOwner, css: &str) -> StyleSheet {
    StyleSheet {
        owner,
        css: css.to_owned(),
    }
}

#[test]
fn the_desktop_sheet_comes_first_then_each_app_with_one_in_name_order() {
    let scratch = Scratch::under(env!("CARGO_TARGET_TMPDIR"), "style-sheets");
    write(&scratch, "bardo/shell/style.css", "b");
    write(&scratch, "bardo/style.css", "a");
    write(&scratch, "bardo/files/style.css", "c");
    write(&scratch, "mailo/style.css", "d");
    write(&scratch, "mailo2/settings.toml", "not a sheet");
    write(&scratch, "loose.css", "not in a folder");
    let found = read_style_sheets(&ConfigHome(scratch.path().to_owned()));
    assert_eq!(
        found,
        [
            sheet(StyleOwner::Desktop, "a"),
            sheet(StyleOwner::App("files".to_owned()), "c"),
            sheet(StyleOwner::App("shell".to_owned()), "b"),
            sheet(StyleOwner::App("mailo".to_owned()), "d"),
        ]
    );
}

#[test]
fn a_missing_folder_or_a_sheet_that_cannot_be_read_leaves_the_file_out() {
    let scratch = Scratch::under(env!("CARGO_TARGET_TMPDIR"), "style-sheets-unreadable");
    assert!(read_style_sheets(&ConfigHome(scratch.join("nowhere"))).is_empty());
    // A folder where the sheet should be, and bytes that are not text.
    fs::create_dir_all(scratch.join("bardo/shell/style.css")).expect("mkdir");
    write(&scratch, "bardo/files/style.css", "ok");
    fs::create_dir_all(scratch.join("bardo")).expect("mkdir");
    fs::write(scratch.join("bardo/style.css"), b"\xff\xfe").expect("write");
    let found = read_style_sheets(&ConfigHome(scratch.path().to_owned()));
    assert_eq!(found, [sheet(StyleOwner::App("files".to_owned()), "ok")]);
}

#[test]
fn the_classes_apps_publish_are_read_from_the_system_folders_then_the_persons() {
    let scratch = Scratch::under(env!("CARGO_TARGET_TMPDIR"), "published-classes");
    let parts = |app: &str, class: &str| format!("app = \"{app}\"\nclasses = [\"{class}\"]\n");
    write(
        &scratch,
        "system/bardo/selectors/shell.toml",
        &parts("shell", "shell-bar"),
    );
    write(
        &scratch,
        "data/bardo/selectors/files.toml",
        &parts("files", "files-row"),
    );
    let found = published_classes(&DataHome(scratch.join("data")), &[scratch.join("system")]);
    assert_eq!(
        found,
        [vec!["shell-bar".to_owned()], vec!["files-row".to_owned()]]
    );
    assert!(published_classes(&DataHome(scratch.join("nowhere")), &[]).is_empty());
}
