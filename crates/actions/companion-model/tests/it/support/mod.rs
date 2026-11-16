//! Builders the tests share; labels come from prov's constructors.

use account_core::{AppName, DataClass};
use action_manifest::{ActionDecl, Manifest, Visibility};
use action_vocab::{IntentsVocab, LabelText};
use provenance::{EntityId, EntityKey, EntityKind, Label, Source, SpaceId, UnixSeconds};

pub fn app(name: &str) -> AppName {
    AppName::parse(name).expect("app")
}
pub fn space(name: &str) -> SpaceId {
    SpaceId::parse(name).expect("space")
}
pub fn kind(name: &str) -> EntityKind {
    EntityKind::parse(name).expect("kind")
}
pub fn words(text: &str) -> LabelText {
    LabelText::parse(text).expect("words")
}
pub fn entity(kind_name: &str, key: &str) -> EntityId {
    EntityId {
        app: app("org.quire.Mail"),
        kind: kind(kind_name),
        key: EntityKey::parse(key).expect("key"),
    }
}
pub fn at(t: i64) -> UnixSeconds {
    UnixSeconds(t)
}
pub fn untrusted_mail(in_space: &str) -> Label {
    Label::untrusted(Source::Mail, DataClass::Mail, space(in_space))
}
pub fn manifest(app_name: &str, actions: Vec<ActionDecl>) -> Manifest {
    Manifest {
        vocab: IntentsVocab::CURRENT,
        app: app(app_name),
        entities: vec![],
        actions,
        visibility: Visibility::Everyone,
    }
}
