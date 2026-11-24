//! intentd's inferd link is inference-client's D-Bus transport on the daemon's own connection. The
//! bus here is a private one with no inferd on it, so the first `open` says inferd is away (the
//! reviewer then asks the person) instead of the daemon failing to start. The link is
//! `action_dbus::inferd_transport`, the one constructor intentd, companiond and readerd share.

use base_testkit::PrivateBus;
use account_capability::LlmFeature;
use account_capability::Tier;
use account_core::DataClass;
use account_match::LlmNeed;
use account_match::Need;
use action_daemon::{InferdModel, InferdWriter, inferd_transport};
use base_ids::Tokens;
use inference_client::{Transport, TransportError};
use inference_core::{Model, ModelCard};
use std::collections::BTreeSet;

#[tokio::test]
async fn an_absent_inferd_is_unreachable_on_the_link() {
    let scratch = tempfile::tempdir().expect("scratch");
    let bus = PrivateBus::start_in(scratch.path());
    let connection = bus.connect().await;

    let link = inferd_transport(&connection, action_dbus::InferWatch::off());
    let need = Need::Llm(LlmNeed::new(
        BTreeSet::from([LlmFeature::Chat]),
        Tokens(4096),
    ));
    let opened = link.open(&need, DataClass::Prompt, Tier::Fast).await;
    assert!(
        matches!(opened, Err(TransportError::Unreachable)),
        "{:?}",
        opened.map(|_| ())
    );
}

#[tokio::test]
async fn the_models_and_the_writer_are_built_on_the_bus() {
    let scratch = tempfile::tempdir().expect("scratch");
    let bus = PrivateBus::start_in(scratch.path());
    let connection = bus.connect().await;
    let card = ModelCard {
        account: account_core::AccountId::parse("local").expect("account"),
        model: account_capability::ModelId::parse("q").expect("model"),
        locality: account_capability::Locality::OnDevice,
        billing: account_capability::Billing::Free,
        capabilities: vec![],
    };
    let model = InferdModel::on_bus(&connection, card.clone(), action_dbus::InferWatch::off());
    assert_eq!(model.card(), &card);
    assert!(format!("{model:?}").contains("Dbus"), "{model:?}");
    let writer = InferdWriter::on_bus(&connection, action_dbus::InferWatch::off());
    assert!(format!("{writer:?}").contains("Dbus"), "{writer:?}");
}
