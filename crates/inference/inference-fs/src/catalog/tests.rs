use super::*;
use account_capability::Subject;
use base_testkit::Scratch;
use inference_router::catalog::claims_of;
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

#[test]
fn the_catalogue_alone_makes_no_claim_for_an_attached_entry_whatever_its_locality_says() {
    let shipped = CatalogDirs {
        system: inference_router::testkit::pinned_stoker_catalog(),
        user: PathBuf::from("/nonexistent/user/catalog"),
    };
    let catalog = read_catalog(&shipped);
    let entry = catalog
        .entries
        .iter()
        .find(|entry| entry.id.0 == entries::ATTACHED)
        .expect("the shipped attached entry");
    assert!(
        entry.locality.is_on_device(),
        "the catalogue says on-device"
    );
    let claimed: Vec<_> = catalog
        .entries
        .iter()
        .filter(|entry| entry.locality.is_on_device() && entry.serving.is_launched())
        .flat_map(claims_of)
        .filter_map(|claim| match claim.subject {
            Subject::Model(id) => Some(id),
            _ => None,
        })
        .collect();
    assert!(
        claimed.iter().all(|id| id.as_str() != entries::ATTACHED),
        "{claimed:?}"
    );
    assert!(!claimed.is_empty(), "the launched models still claim");
}
