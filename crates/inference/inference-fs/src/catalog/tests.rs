use super::*;
use base_testkit::Scratch;
use inference_router::testkit::entries;

#[test]
fn a_user_file_replaces_the_system_file_and_a_bad_file_is_skipped_and_named() {
    let (system, user) = (Scratch::new("system"), Scratch::new("user"));
    std::fs::write(system.path().join("tiny-chat.toml"), entries::chat()).expect("write");
    std::fs::write(
        user.path().join("tiny-chat.toml"),
        entries::chat().replace("context = 8192", "context = 4096"),
    )
    .expect("write");
    std::fs::write(user.path().join("broken.toml"), "id = ").expect("write");
    std::fs::write(user.path().join("notes.txt"), "not a catalog file").expect("write");
    let catalog = read_catalog(&CatalogDirs {
        system: system.path().to_path_buf(),
        user: user.path().to_path_buf(),
    });
    assert_eq!(catalog.entries.len(), 1);
    assert_eq!(
        catalog.entries[0]
            .capabilities
            .chat_caps()
            .map(|caps| caps.context.0),
        Some(4096)
    );
    assert_eq!(catalog.skipped.len(), 1);
    assert!(catalog.skipped[0].what.ends_with("broken.toml"));
}

#[test]
fn a_missing_directory_is_an_empty_catalog() {
    let catalog = read_catalog(&CatalogDirs {
        system: PathBuf::from("/nonexistent/a"),
        user: PathBuf::from("/nonexistent/b"),
    });
    assert_eq!(catalog, Catalog::default());
}
