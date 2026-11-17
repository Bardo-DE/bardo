use super::*;
use crate::attached::config::{Reach, Site};
use crate::catalog::parse_entry_text;
use crate::testkit::entries;
use account_capability::Locality;
use account_capability::ModelId;
use base_testkit::Scratch;

fn attached(id: &str, place: Site) -> Attached {
    Attached {
        id: ModelId::parse(id).expect("id"),
        reach: Reach::Socket("/run/user/1000/lab.sock".into()),
        key_file: None,
        place,
        computer: None,
    }
}

fn catalogue() -> Vec<ModelEntry> {
    [entries::attached(), entries::chat(), entries::embed()]
        .iter()
        .map(|text| parse_entry_text(text).expect("entry"))
        .collect()
}

#[test]
fn an_attached_model_is_the_catalogues_entry_with_the_place_the_person_said() {
    let scratch = Scratch::new("am");
    let entries = catalogue();
    // The catalogue says on-device; the person's `where` is what routing sees.
    assert_eq!(entries[0].locality, model_catalog::Locality::OnDevice);
    let make = |place| {
        local_model(
            &attached(entries::ATTACHED, place),
            &entries,
            scratch.path(),
        )
        .expect("model")
    };
    let (here, there) = (make(Site::ThisDevice), make(Site::MyNetwork));
    assert_eq!(here.card.locality, Locality::OnDevice);
    assert_eq!(there.card.locality, Locality::LocalNetwork);
    for model in [&here, &there] {
        assert_eq!(model.card.billing, account_capability::Billing::Free);
        // A request names, and `/v1/models` must list, the name the catalogue says it is served as.
        assert_eq!(model.name.0, entries::SERVED);
        assert_eq!(model.entry.id.0, entries::ATTACHED);
        assert_eq!(model.card.model.as_str(), entries::ATTACHED);
        assert_eq!(model.spec.id.0, format!("attached:{}", entries::ATTACHED));
        // Never started: no GPU is claimed for it, and no weights are looked for.
        assert_eq!(model.spec.need, MiB(0));
        assert_eq!(model.weights(), crate::local::Weights::Present);
        assert_eq!(model.socket.0, PathBuf::from("/run/user/1000/lab.sock"));
        assert!(model.attached.is_some() && model.loopback.is_none());
        // The wire is the catalogue's `engine`.
        assert_eq!(model.flavor, Some(Flavor::Vllm));
        assert!(!model.card.capabilities.is_empty());
    }
}

#[test]
fn an_entry_inferd_launches_or_an_id_the_catalogue_lacks_is_a_typed_error() {
    let scratch = Scratch::new("am");
    let one = |id: &str| {
        local_model(
            &attached(id, Site::ThisDevice),
            &catalogue(),
            scratch.path(),
        )
    };
    assert_eq!(
        one("tiny-chat").err(),
        Some(AttachedError::NotAttachable {
            id: "tiny-chat".into()
        })
    );
    assert_eq!(
        one("not-there").err(),
        Some(AttachedError::NotInCatalogue {
            id: "not-there".into()
        })
    );
    let all = crate::attached::models(
        &[
            attached(entries::ATTACHED, Site::ThisDevice),
            attached("tiny-embed", Site::ThisDevice),
        ],
        &catalogue(),
        scratch.path(),
    );
    assert!(matches!(all, Err(AttachedError::NotAttachable { .. })));
}
