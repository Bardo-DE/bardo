//! Records the specs talk about, as values. Labels are built from fields: `provenance::Label`'s
//! constructors are stubs in the accounts service until its fill.

use crate::clock::NOW;
use memory_core::{
    Actor, AgentRole, AppName, AreaPayload, AreaTag, Cause, Confidentiality, ContentDigest,
    DataClass, Effect, EventBody, FileChange, FileView, FileWhy, Integrity, JsonText, KindTag,
    Label, Record, RunId, SessionId, Source, SpaceId, SpacePath, SystemPart, ThingKey, ThingKind,
    ThingRef, ThingRole, ThingView, UserText, Verb,
};
use std::collections::BTreeSet;

fn space() -> SpaceId {
    SpaceId::parse("work").unwrap_or_else(|_| SpaceId::desktop())
}

fn app(name: &str) -> AppName {
    AppName::parse(name).unwrap_or_else(|_| unreachable_name())
}

fn unreachable_name() -> AppName {
    // The fixtures only name valid apps; this keeps the helpers total without a panic path.
    AppName::parse("org.bardo.Shell").unwrap_or_else(|_| unreachable_name())
}

/// The Mail app.
#[must_use]
pub fn mail() -> AppName {
    app("org.bardo.Mail")
}

/// A thing in Mail.
#[must_use]
pub fn thing(kind: &str, key: &str) -> Option<ThingRef> {
    Some(ThingRef {
        app: mail(),
        kind: ThingKind::parse(kind).ok()?,
        key: ThingKey::parse(key).ok()?,
    })
}

fn view(kind: &str, key: &str, title: &str) -> Option<ThingView> {
    Some(ThingView {
        thing: thing(kind, key)?,
        title: title.into(),
        subtitle: UserText::default(),
    })
}

/// The person's own words: trusted, public.
#[must_use]
pub fn trusted_label() -> Label {
    Label {
        integrity: Integrity::Trusted,
        confidentiality: Confidentiality::Public,
        classes: BTreeSet::new(),
        sources: BTreeSet::from([Source::User]),
    }
}

/// Mail from someone else: untrusted, private to the Space.
#[must_use]
pub fn mail_label() -> Label {
    Label {
        integrity: Integrity::Untrusted,
        confidentiality: Confidentiality::Private(BTreeSet::from([space()])),
        classes: BTreeSet::from([DataClass::Mail]),
        sources: BTreeSet::from([Source::Mail]),
    }
}

fn record(actor: Actor, effect: Effect, label: Label, body: EventBody, cause: Cause) -> Record {
    Record {
        space: space(),
        occurred: NOW,
        actor,
        effect,
        label,
        body,
        cause,
    }
}

/// The person archived the thread "Q4 budget" in Mail.
#[must_use]
pub fn mail_thread_archived() -> Option<Record> {
    Some(record(
        Actor::User { via: mail() },
        Effect::UndoableWrite,
        trusted_label(),
        EventBody::Thing {
            verb: Verb::Archived,
            thing: view("mail.thread", "7f3a", "Q4 budget")?,
            sources: vec![],
        },
        Cause::None,
    ))
}

/// A receipt attachment was saved to Downloads and Mail said why.
#[must_use]
pub fn file_saved_from_attachment() -> Option<Record> {
    let cause = thing("mail.message", "m1")?;
    Some(record(
        Actor::User { via: mail() },
        Effect::UndoableWrite,
        mail_label(),
        EventBody::File {
            change: FileChange::Created,
            file: FileView {
                path: SpacePath::parse("/home/u/Downloads/receipt.pdf").ok()?,
                inode: 4242,
                content: ContentDigest([7; 32]),
            },
            why: FileWhy::Explained {
                cause,
                verb: Verb::Downloaded,
                by: Actor::User { via: mail() },
            },
        },
        Cause::None,
    ))
}

/// The companion forwarded a thread (the router records it on the companion's behalf).
#[must_use]
pub fn companion_forwarded() -> Option<Record> {
    Some(record(
        Actor::Companion {
            session: SessionId::parse("s-1").ok()?,
            role: AgentRole::Planner,
        },
        Effect::Outbound,
        mail_label(),
        EventBody::Thing {
            verb: Verb::Forwarded,
            thing: view("mail.thread", "7f3a", "Q4 budget")?,
            sources: vec![],
        },
        Cause::Plan(SessionId::parse("s-1").ok()?),
    ))
}

fn area(area: AreaTag, kind: &str, json: &str) -> Option<EventBody> {
    Some(EventBody::Area(AreaPayload {
        area,
        kind: KindTag::parse(kind).ok()?,
        json: JsonText::parse(json).ok()?,
        things: vec![(
            view("mail.thread", "7f3a", "Q4 budget")?,
            ThingRole::Subject,
        )],
    }))
}

/// One computer-use step, as the computer-use daemon records it.
#[must_use]
pub fn computer_use_run_step() -> Option<Record> {
    let run = RunId::parse("r-1").ok()?;
    Some(record(
        Actor::Companion {
            session: SessionId::parse("s-1").ok()?,
            role: AgentRole::ComputerUse { run },
        },
        Effect::Read,
        mail_label(),
        area(
            AreaTag::ComputerUse,
            "cua.step",
            r#"{"step":1,"action":"observe"}"#,
        )?,
        Cause::None,
    ))
}

/// The router asked the person before an outbound action.
#[must_use]
pub fn policy_ask() -> Option<Record> {
    Some(record(
        Actor::System {
            part: SystemPart::Router,
        },
        Effect::Outbound,
        trusted_label(),
        area(
            AreaTag::Actions,
            "policy.ruled",
            r#"{"ruling":"ask","rule":"untrusted-sink"}"#,
        )?,
        Cause::None,
    ))
}

/// The thing a companion session's entries name as their `Subject`: the stream key.
#[must_use]
pub fn session_thing(session: &str) -> Option<ThingRef> {
    Some(ThingRef {
        app: app("org.bardo.Companion"),
        kind: ThingKind::parse("companion.session").ok()?,
        key: ThingKey::parse(session).ok()?,
    })
}

/// One entry of a companion session's durable log, as the actions service writes it: an `Area(Companion)`
/// payload of kind `companion.session.<slug>` that names its session as the `Subject`.
#[must_use]
pub fn session_entry(session: &str, slug: &str, text: &str) -> Option<Record> {
    let json = serde_json::json!({ "session": session, "text": text }).to_string();
    Some(record(
        Actor::System {
            part: SystemPart::Router,
        },
        Effect::Read,
        trusted_label(),
        EventBody::Area(AreaPayload {
            area: AreaTag::Companion,
            kind: KindTag::parse(&format!("companion.session.{slug}")).ok()?,
            json: JsonText::parse(&json).ok()?,
            things: vec![(
                ThingView {
                    thing: session_thing(session)?,
                    title: UserText::default(),
                    subtitle: UserText::default(),
                },
                ThingRole::Subject,
            )],
        }),
        Cause::None,
    ))
}
