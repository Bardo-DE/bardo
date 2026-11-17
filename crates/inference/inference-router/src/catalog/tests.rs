use super::*;
use crate::testkit::entries;
use base_ids::{Dims, Tokens};

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
fn a_computer_use_entry_claims_for_the_dialects_an_engine_here_can_speak() {
    let claims = |dialect: &str| {
        let text = entries::computer_use().replace(r#"{ kind = "tool", v = "holo31" }"#, dialect);
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
    let text = entries::computer_use().replace(
        r#"{ kind = "tool", v = "holo31" }"#,
        r#"{ kind = "wire", v = "open_ai_computer" }"#,
    );
    let caps = capabilities_of(&parse_entry(&text).expect("entry"));
    assert!(caps.iter().any(|c| matches!(c, Capability::Llm(_))));
}
