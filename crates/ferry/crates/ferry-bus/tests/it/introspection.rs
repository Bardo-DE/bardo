//! The checked-in introspection files are the interfaces as they are declared once. A change to a
//! signature changes the file in the same commit.

use std::path::PathBuf;

#[test]
fn checked_in_introspection_matches_the_interfaces() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../dbus");
    for document in [ferry_bus::SYNC, ferry_bus::STORAGE] {
        let path = dir.join(document.file_name);
        let expected =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let actual = match document.file_name {
            "org.quire.Sync1.xml" => without_picker(&document.introspection()),
            _ => document.introspection(),
        };
        assert!(
            actual == expected,
            "{} differs from the interfaces; it should read:\n{actual}",
            path.display()
        );
    }
}

/// The Sync1 document without the Photos Picker, which is outside the default surface (the
/// `photos-picker` feature adds it).
fn without_picker(xml: &str) -> String {
    match xml.split_once("<interface name=\"org.quire.Photos1.Picker\">") {
        Some((sync1, _)) => format!("{}</node>\n", sync1.trim_end_matches(' ')),
        None => xml.to_owned(),
    }
}

/// The words that cross the bus as text and are named in no signature: the error names, the
/// resolve words and the status key. The method and signal lists are the checked-in files above.
#[test]
fn the_text_words_on_the_bus_are_the_documented_ones() {
    assert_eq!(
        ferry_bus::SYNC_ERROR_PAUSED,
        format!("{}Paused", ferry_bus::SYNC_ERROR_PREFIX)
    );
    assert_eq!(
        ferry_bus::SYNC_ERROR_NOTHING_HELD,
        format!("{}NothingHeld", ferry_bus::SYNC_ERROR_PREFIX)
    );
    assert_eq!(
        ferry_bus::SYNC_ERROR_NO_SUCH_CONFLICT,
        format!("{}NoSuchConflict", ferry_bus::SYNC_ERROR_PREFIX)
    );
    assert_eq!(ferry_bus::RESOLVE_KEEP_LOCAL, "keep_local");
    assert_eq!(ferry_bus::RESOLVE_KEEP_REMOTE, "keep_remote");
    assert_eq!(ferry_bus::STATUS_KEY_QUOTA, "quota");
    #[cfg(feature = "photos-picker")]
    assert_eq!(
        ferry_bus::PICKER_ERROR_NOT_YET,
        format!("{}NotYet", ferry_bus::PICKER_ERROR_PREFIX)
    );
}
