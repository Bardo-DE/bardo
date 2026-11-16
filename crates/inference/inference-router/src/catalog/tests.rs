use super::*;
use crate::testkit::entries;
use std::path::PathBuf;
use base_testkit::Scratch;

#[test]
fn an_embedding_entry_makes_a_claim_from_its_embed_table() {
    let entry = parse_entry(&entries::embed()).expect("entry");
    let claims = claims_of(&entry);
    let [
        Claim {
            offer: Offer::Present(Capability::Embeddings(cap)),
            subject: Subject::Model(model),
            provenance: Provenance::Curated,
        },
    ] = claims.as_slice()
    else {
        panic!("one embeddings claim: {claims:?}");
    };
    assert_eq!(model.as_str(), "tiny-embed");
    assert_eq!(cap.dims, Dims(4));
    assert_eq!(cap.max_input, Tokens(512));
    assert_eq!(cap.max_batch, Count(2));
    assert_eq!(cap.prompts.query, "search_query: ");
    assert_eq!(cap.modalities, BTreeSet::from([Modality::Text]));
}

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
fn a_computer_use_entry_claims_for_the_dialects_an_engine_here_can_speak() {
    let claims = |dialect: &str| {
        let text = entries::cua().replace(r#"{ kind = "tool", v = "holo31" }"#, dialect);
        capabilities_of(&parse_entry(&text).expect("entry"))
            .iter()
            .any(|c| matches!(c, Capability::ComputerUse(_)))
    };
    assert!(claims(r#"{ kind = "tool", v = "holo31" }"#));
    assert!(claims(r#"{ kind = "text", v = "ui_tars15" }"#));
    assert!(
        !claims(r#"{ kind = "wire", v = "open_ai_computer" }"#),
        "a vendor's wire needs the vendor's backend"
    );
    let text = entries::cua().replace(
        r#"{ kind = "tool", v = "holo31" }"#,
        r#"{ kind = "wire", v = "open_ai_computer" }"#,
    );
    let caps = capabilities_of(&parse_entry(&text).expect("entry"));
    assert!(caps.iter().any(|c| matches!(c, Capability::Llm(_))));
}
