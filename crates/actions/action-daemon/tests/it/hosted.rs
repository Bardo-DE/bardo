//! The built-in providers through a whole intentd on a private bus: `org.bardo.Memory` over a
//! fake memoryd and `org.bardo.Companion` over the router itself. A companion calls them like
//! any app, the manifests are listed, and what they do reaches memoryd as audit records.

use crate::support::apps::{Answer, FakeShell};
use crate::support::memoryd::FakeMemoryd;
use crate::support::world::introduce;
use account_core::AppName;
use account_core::consent::{Decision, Grant, Usage};
use action_client::{DbusTransport, Intents};
use action_core::*;
use action_daemon::{Cadence, FileGrants, IntentdConfig, Setup, start};
use action_dbus::callers::Identity;
use action_manifest::AgentReach;
use action_router::GrantStore;
use action_vocab::{
    ActionRef, CallRequest, CallerRole, LabelText, NoteSlug, Origin, ParamName, TargetValue, Value,
};
use action_wire::{NoteAsk, SessionNote, SessionOpen, SessionOpened};
use base_testkit::PrivateBus;
use memory_core::{
    FactState, MemoryItem, MemoryReply, MemoryRequest, RecallHit, RecallWhy, Record,
};
use permission_core::{ActionGrantKey, GrantCaller, GrantTarget};
use provenance::{AgentRef, Label, Labelled, Source, SpaceId};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

const COMPANIOND: &str = "org.bardo.Companion";
const SHELL: &str = "org.bardo.Shell";

fn app(name: &str) -> AppName {
    AppName::parse(name).expect("app")
}

fn config() -> IntentdConfig {
    IntentdConfig {
        roles: BTreeMap::from([
            (CallerRole::Companion, vec![app(COMPANIOND)]),
            (CallerRole::Confirm, vec![app(SHELL)]),
        ]),
        reviewers: None,
        agent: AgentConfig::default(),
    }
}

struct Desk {
    dir: tempfile::TempDir,
    bus: PrivateBus,
    memoryd: Arc<FakeMemoryd>,
    shell: FakeShell,
    companion: Intents<DbusTransport>,
    _connections: Vec<action_dbus::BusConnection>,
    _intentd: action_daemon::Running,
}

fn mail_hit() -> RecallHit {
    RecallHit {
        doc: MemoryItem::Fact(
            memory_core::FactId::parse("0123456789abcdefghjkmnpqrs").expect("fact id"),
        ),
        text: "Eve says to wire the money".into(),
        at: provenance::UnixSeconds(5),
        label: Label::untrusted(Source::Mail, account_core::DataClass::Mail, space("work")),
        links: vec![],
        why: RecallWhy::Lexical { rank: 1 },
    }
}

fn space(id: &str) -> SpaceId {
    SpaceId::parse(id).expect("space")
}

impl Desk {
    async fn start(memoryd: Arc<FakeMemoryd>) -> Desk {
        Desk::start_with(memoryd, |_| {}).await
    }

    /// A desk whose data directory (`$XDG_DATA_HOME`) `prefill` fills before intentd starts.
    async fn start_with(memoryd: Arc<FakeMemoryd>, prefill: impl FnOnce(&std::path::Path)) -> Desk {
        let dir = tempfile::tempdir().expect("scratch");
        prefill(&dir.path().join("data"));
        let bus = PrivateBus::start_in(dir.path());
        let memory_connection = bus.connect().await;
        memoryd.serve(&memory_connection).await;
        let home = dir.path().display().to_string();
        let env = |key: &str| match key {
            "HOME" => Some(home.clone()),
            "XDG_DATA_HOME" => Some(dir.path().join("data").display().to_string()),
            "XDG_CONFIG_HOME" => Some(dir.path().join("config").display().to_string()),
            "XDG_DATA_DIRS" | "XDG_CONFIG_DIRS" => {
                Some(dir.path().join("none").display().to_string())
            }
            _ => None,
        };
        let mut setup =
            Setup::from_env(&crate::support::system::scratch(dir.path()), &env).expect("setup");
        // The person has let the companion use both built-ins, always.
        let grants = FileGrants::at(setup.grants.clone());
        for (n, owner) in ["org.bardo.Memory", "org.bardo.Companion"]
            .into_iter()
            .enumerate()
        {
            grants.record(Grant {
                id: account_core::GrantId::parse(&format!("g-{n}")).expect("id"),
                key: ActionGrantKey {
                    caller: GrantCaller::Companion,
                    owner: app(owner),
                    target: GrantTarget::App,
                    class: account_core::DataClass::AppOwn,
                    usage: Usage::Interactive,
                    space: provenance::SpaceScope::Any,
                },
                decision: Decision::Allow,
                scope: GrantScope::Always,
                at: provenance::UnixSeconds(1),
            });
        }
        setup.config = config();
        let daemon_connection = bus.connect().await;
        let who = Arc::new(Identity::on_fixture(
            daemon_connection.clone(),
            action_daemon::callers::shipped(),
            dir.path().join("proc"),
        ));
        setup.identity = Some(Arc::clone(&who));
        setup.proc_root = action_daemon::ProcRoot::Honoured(dir.path().join("proc"));
        setup.audit_every = Duration::from_millis(50);
        setup.signals = Cadence {
            every: Duration::from_millis(30),
            rescan_every: 2,
        };
        let intentd = start(&daemon_connection, None, setup)
            .await
            .expect("intentd");
        let shell_connection = bus.connect().await;
        let shell = FakeShell::start(&shell_connection, &["org.bardo.Confirm1", SHELL]).await;
        introduce(
            &who,
            &shell_connection,
            &config(),
            &["org.bardo.Confirm1", SHELL],
        );
        let companion_connection = bus.connect().await;
        companion_connection
            .request_name(COMPANIOND)
            .await
            .expect("name");
        introduce(&who, &companion_connection, &config(), &[COMPANIOND]);
        let companion = Intents::over(DbusTransport::new(companion_connection.clone()));
        Desk {
            dir,
            bus,
            memoryd,
            shell,
            companion,
            _connections: vec![
                memory_connection,
                daemon_connection,
                companion_connection,
                shell_connection,
            ],
            _intentd: intentd,
        }
    }

    async fn front(&self) -> SessionOpened {
        self.companion
            .session_open(SessionOpen {
                space: space("work"),
                agent: AgentRef::Companion,
                parent: None,
                cwd: None,
                started_from: None,
                external: None,
                helper: None,
                acts_for: None,
            })
            .await
            .expect("a session")
    }

    async fn records(&self, kind: &str) -> Vec<Record> {
        base_testkit::eventually_some(&format!("a stored record of kind {kind}"), || {
            let found: Vec<Record> = self
                .memoryd
                .stored()
                .into_iter()
                .filter(|r| r.body.kind().as_str() == kind)
                .collect();
            (!found.is_empty()).then_some(found)
        })
        .await
    }
}

fn call(app_name: &str, action: &str, target: TargetValue, args: &[(&str, Value)]) -> CallRequest {
    CallRequest {
        action: ActionRef {
            app: app(app_name),
            name: provenance::ActionName::parse(action).expect("action"),
        },
        target,
        args: args
            .iter()
            .map(|(name, value)| {
                (
                    ParamName::parse(name).expect("param"),
                    Labelled {
                        value: value.clone(),
                        label: Label::trusted_user(),
                    },
                )
            })
            .collect(),
        origin: Origin::Companion,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_built_in_providers_are_listed_with_their_manifests() {
    let desk = Desk::start(FakeMemoryd::recording()).await;
    let listed: Vec<String> = desk
        .companion
        .manifests()
        .await
        .expect("manifests")
        .iter()
        .map(|m| m.manifest().app.to_string())
        .collect();
    assert_eq!(
        listed,
        [
            "org.bardo.AcpAgent",
            "org.bardo.Checkpoints",
            "org.bardo.Companion",
            "org.bardo.Memory"
        ]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_companion_recalls_through_memory_and_what_it_gets_carries_the_hits_label() {
    let memoryd = FakeMemoryd::with(|request| match request {
        MemoryRequest::Search(_) => Some(MemoryReply::Hits(vec![mail_hit()])),
        _ => None,
    });
    let desk = Desk::start(Arc::clone(&memoryd)).await;
    let front = desk.front().await;

    let outcome = desk
        .companion
        .perform(
            call(
                "org.bardo.Memory",
                "memory.recall",
                TargetValue::Nothing,
                &[("query", Value::Text("wire".into()))],
            ),
            Some(front.session.clone()),
            None,
        )
        .await
        .expect("the request")
        .expect("the call ran");

    let asked = memoryd.seen();
    // intentd lists the stored sessions at start, so the search is not the first request.
    let Some(query) = asked.iter().find_map(|r| match r {
        MemoryRequest::Search(q) => Some(q),
        _ => None,
    }) else {
        panic!("{asked:?}")
    };
    assert_eq!(
        query.space,
        space("work"),
        "memory is asked for the session's Space"
    );
    assert_eq!(query.text.as_str(), "wire");
    let Some(Labelled { value, label }) = outcome.value else {
        panic!("{outcome:?}")
    };
    assert_eq!(
        label.integrity,
        provenance::Integrity::Untrusted,
        "what was read from mail is untrusted"
    );
    assert!(label.sources.contains(&Source::Mail));
    // A planner names what recall found by handle: one hit, one handle.
    assert!(matches!(&value, Value::List(hits) if matches!(hits.as_slice(), [Value::Handle(_)])));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_proposal_from_a_companion_asks_the_person_then_goes_to_memory_and_lands_pending() {
    let memoryd = FakeMemoryd::with(|request| match request {
        MemoryRequest::Propose(..) => Some(MemoryReply::Proposed(
            memory_core::FactId::parse("0123456789abcdefghjkmnpqrs").expect("fact id"),
            FactState::Pending,
        )),
        _ => None,
    });
    let desk = Desk::start(Arc::clone(&memoryd)).await;
    let front = desk.front().await;
    let propose = |text: &str| {
        call(
            "org.bardo.Memory",
            "memory.propose",
            TargetValue::Nothing,
            &[("text", Value::Text(text.into()))],
        )
    };

    // Nobody answers the sheet: a proposal is a write, so nothing is proposed.
    let unanswered = desk
        .companion
        .perform(
            propose("Sam prefers early meetings"),
            Some(front.session.clone()),
            None,
        )
        .await
        .expect("the request");
    assert!(
        matches!(unanswered, Err(CallRefusal::Unconfirmed(_))),
        "{unanswered:?}"
    );
    assert!(
        memoryd
            .seen()
            .iter()
            .all(|r| !matches!(r, MemoryRequest::Propose(..)))
    );

    desk.shell.answer(vec![Answer::With(ConfirmAnswer::Allowed {
        written: None,
        scope: action_core::GrantScope::Once,
        receipt: provenance::ConfirmReceipt {
            id: provenance::ConfirmId::parse("c-1").expect("id"),
            input: provenance::InputProof::HardwareSeat,
            at: provenance::UnixSeconds(1),
            covers: provenance::Confidentiality::Secret,
        },
    })]);
    let outcome = desk
        .companion
        .perform(propose("Sam prefers mornings"), Some(front.session), None)
        .await
        .expect("the request")
        .expect("it runs once the person says yes");
    assert_eq!(
        outcome.said.as_ref().map(LabelText::as_str),
        Some("Saved for you to keep or drop")
    );
    assert_eq!(outcome.undo, Undoable::No);
    let asked = memoryd.seen();
    let Some(MemoryRequest::Propose(in_space, draft)) = asked
        .iter()
        .find(|r| matches!(r, MemoryRequest::Propose(..)))
    else {
        panic!("{asked:?}")
    };
    assert_eq!(*in_space, space("work"));
    assert_eq!(draft.text.as_str(), "Sam prefers mornings");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_companion_starts_a_task_and_the_task_is_audited_into_memory() {
    let desk = Desk::start(FakeMemoryd::recording()).await;
    let front = desk.front().await;

    let outcome = desk
        .companion
        .perform(
            call(
                "org.bardo.Companion",
                "companion.task.start",
                TargetValue::Nothing,
                &[("goal", Value::Text("find the Lisbon receipts".into()))],
            ),
            Some(front.session),
            None,
        )
        .await
        .expect("the request")
        .expect("the task starts");
    // The answer is the task and the session the worker runs in.
    let Some(Labelled {
        value: Value::Record(answer),
        ..
    }) = outcome.value
    else {
        panic!("{outcome:?}")
    };
    let Some(Value::Text(task)) = answer.get(&ParamName::parse("task").expect("param")) else {
        panic!("{answer:?}")
    };
    let task = task.clone();
    assert!(
        matches!(
            answer.get(&ParamName::parse("session").expect("param")),
            Some(Value::Text(_))
        ),
        "{answer:?}"
    );

    let worker = provenance::TaskId::parse(&task).expect("a task id");
    let inbox = desk
        .companion
        .inbox(InboxAsk {
            agent: AgentRef::Worker {
                task: worker.clone(),
            },
            after: None,
        })
        .await
        .expect("inbox");
    assert_eq!(inbox.len(), 1, "the goal reached the worker as a request");

    let started = desk.records("actions.task_started").await;
    assert_eq!(started.len(), 1, "the new task is in the log");
    let messages = desk.records("companion.message").await;
    assert_eq!(messages.len(), 1, "and so is the request it was given");
    assert!(
        !desk.records("actions.call").await.is_empty(),
        "the call that started it too"
    );
}

fn first<T>(
    stream: &mut (impl futures_util::Stream<Item = T> + Unpin),
) -> impl std::future::Future<Output = T> + '_ {
    use futures_util::StreamExt;
    async move {
        tokio::time::timeout(Duration::from_secs(5), stream.next())
            .await
            .expect("the signal comes")
            .expect("a signal")
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_installed_manifest_appearing_changing_and_going_is_said_on_the_registry() {
    let desk = Desk::start(FakeMemoryd::recording()).await;
    let listener = desk.bus.connect().await;
    let registry = action_dbus::RegistryProxy::new(&listener)
        .await
        .expect("proxy");
    let mut changed = registry
        .receive_manifest_changed()
        .await
        .expect("subscribed");
    let folder = desk.dir.path().join("data/bardo/intents");
    std::fs::create_dir_all(&folder).expect("dir");

    std::fs::write(
        folder.join("org.bardo.Mail.toml"),
        action_fake::MAIL_MANIFEST,
    )
    .expect("write");
    let signal = first(&mut changed).await;
    assert_eq!(signal.args().expect("args").app(), &"org.bardo.Mail");
    let listed = desk.companion.manifests().await.expect("manifests");
    assert!(
        listed
            .iter()
            .any(|m| m.manifest().app.as_str() == "org.bardo.Mail")
    );

    let edited = action_fake::MAIL_MANIFEST.replace("Read", "Look at");
    std::fs::write(folder.join("org.bardo.Mail.toml"), edited).expect("write");
    assert_eq!(
        first(&mut changed).await.args().expect("args").app(),
        &"org.bardo.Mail"
    );

    std::fs::remove_file(folder.join("org.bardo.Mail.toml")).expect("remove");
    assert_eq!(
        first(&mut changed).await.args().expect("args").app(),
        &"org.bardo.Mail"
    );
    let listed = desk.companion.manifests().await.expect("manifests");
    assert!(
        listed
            .iter()
            .all(|m| m.manifest().app.as_str() != "org.bardo.Mail")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_file_cannot_declare_the_built_in_providers() {
    let desk = Desk::start(FakeMemoryd::recording()).await;
    let listener = desk.bus.connect().await;
    let registry = action_dbus::RegistryProxy::new(&listener)
        .await
        .expect("proxy");
    let mut changed = registry
        .receive_manifest_changed()
        .await
        .expect("subscribed");
    let folder = desk.dir.path().join("data/bardo/intents");
    std::fs::create_dir_all(&folder).expect("dir");
    // A file that renames the memory actions' effects: the built-in declaration stands.
    let memory = include_str!("../../../manifests/org.bardo.Memory.toml")
        .replace("reach = \"hidden\"", "reach = \"offered\"");
    std::fs::write(folder.join("org.bardo.Memory.toml"), memory).expect("write");
    // A marker written after it: once the registry says the marker's app appeared, a scan that
    // saw the marker has seen the memory file as well.
    std::fs::write(
        folder.join("org.bardo.Mail.toml"),
        action_fake::MAIL_MANIFEST,
    )
    .expect("write");
    loop {
        let signal = first(&mut changed).await;
        if signal.args().expect("args").app() == &"org.bardo.Mail" {
            break;
        }
    }
    let listed = desk.companion.manifests().await.expect("manifests");
    let memory = listed
        .iter()
        .find(|m| m.manifest().app.as_str() == "org.bardo.Memory")
        .expect("memory is still there");
    let forget = memory
        .manifest()
        .actions
        .iter()
        .find(|a| a.name.as_str() == "memory.forget")
        .expect("forget");
    assert_eq!(
        forget.reach,
        AgentReach::Hidden,
        "an agent never sees forgetting"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_message_arriving_for_a_worker_is_said_content_free() {
    let desk = Desk::start(FakeMemoryd::recording()).await;
    let listener = desk.bus.connect().await;
    let messages = action_dbus::MessageProxy::new(&listener)
        .await
        .expect("proxy");
    let mut arrived = messages.receive_arrived().await.expect("subscribed");
    let front = desk.front().await;
    let outcome = desk
        .companion
        .perform(
            call(
                "org.bardo.Companion",
                "companion.task.start",
                TargetValue::Nothing,
                &[("goal", Value::Text("find the receipts".into()))],
            ),
            Some(front.session),
            None,
        )
        .await
        .expect("the request")
        .expect("the task starts");
    // The answer is the task and the session the worker runs in.
    let Some(Labelled {
        value: Value::Record(answer),
        ..
    }) = outcome.value
    else {
        panic!("{outcome:?}")
    };
    let Some(Value::Text(task)) = answer.get(&ParamName::parse("task").expect("param")) else {
        panic!("{answer:?}")
    };
    let task = task.clone();
    assert!(
        matches!(
            answer.get(&ParamName::parse("session").expect("param")),
            Some(Value::Text(_))
        ),
        "{answer:?}"
    );
    let signal = first(&mut arrived).await;
    let agent: AgentRef =
        serde_json::from_str(signal.args().expect("args").agent()).expect("an agent");
    assert_eq!(
        agent,
        AgentRef::Worker {
            task: provenance::TaskId::parse(&task).expect("task")
        }
    );
    assert!(
        !signal.args().expect("args").agent().contains("receipts"),
        "no content"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_session_record_the_companion_notes_reaches_memory_as_a_companion_area_record() {
    use memory_core::{AreaTag, EventBody, JsonText};
    let desk = Desk::start(FakeMemoryd::recording()).await;
    let front = desk.front().await;
    let json = r#"{"kind":"closed"}"#;
    desk.companion
        .session_note(
            front.session.clone(),
            NoteAsk::Record(SessionNote {
                slug: NoteSlug::parse("closed").expect("slug"),
                json: JsonText::parse(json).expect("json"),
            }),
        )
        .await
        .expect("the router took the note");

    let stored = desk.records("companion.session.closed").await;
    assert_eq!(stored.len(), 1, "{stored:?}");
    assert_eq!(stored[0].space, space("work"), "the Space of the session");
    assert_eq!(stored[0].label.integrity, provenance::Integrity::Trusted);
    let EventBody::Area(payload) = &stored[0].body else {
        panic!("{:?}", stored[0].body)
    };
    assert_eq!(payload.area, AreaTag::Companion);
    assert_eq!(payload.json.as_str(), json, "the owner's form, unchanged");
}

fn skill_dir(data: &std::path::Path, id: &str, uses: &str, body: &str) {
    let dir = data.join("bardo").join("skills").join(id);
    std::fs::create_dir_all(&dir).expect("dir");
    std::fs::write(
        dir.join("skill.toml"),
        format!(
            "vocab = 1\nid = \"{id}\"\nowner = \"org.bardo.Companion\"\nversion = \"0.2.0\"\nuses = [\"{uses}\"]\n"
        ),
    )
    .expect("toml");
    std::fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {id}\ndescription: about {id}\n---\n{body}\n"),
    )
    .expect("md");
}

const LOAD_USES: &str = "org.bardo.Companion:companion.skill.load";

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_skill_loads_as_trusted_text_from_the_person_and_a_task_loads_at_most_three() {
    let desk = Desk::start_with(FakeMemoryd::recording(), |data| {
        for id in ["s-a", "s-b", "s-c", "s-d"] {
            skill_dir(data, id, LOAD_USES, &format!("How to {id}."));
        }
        skill_dir(
            data,
            "s-missing",
            "org.bardo.Companion:companion.no.such",
            "never",
        );
    })
    .await;
    let front = desk.front().await;
    let load = |id: &str| {
        call(
            "org.bardo.Companion",
            "companion.skill.load",
            TargetValue::Nothing,
            &[("id", Value::Text(id.into()))],
        )
    };
    let mut loaded = Vec::new();
    for id in ["s-a", "s-b", "s-c"] {
        loaded.push(
            desk.companion
                .perform(load(id), Some(front.session.clone()), None)
                .await
                .expect("the request")
                .unwrap_or_else(|why| panic!("{id}: {why:?}")),
        );
    }
    let Some(Labelled { value, label }) = &loaded[0].value else {
        panic!("{:?}", loaded[0])
    };
    assert_eq!(value, &Value::Text("How to s-a.".into()));
    assert_eq!(label.integrity, provenance::Integrity::Trusted);
    assert_eq!(
        label.sources,
        std::collections::BTreeSet::from([Source::User])
    );
    assert_eq!(loaded[0].undo, Undoable::No, "a load has no undo");

    // The fourth is refused; one already loaded is still free; a skill with a missing action
    // and one that does not exist cannot be loaded at all.
    let refused = desk
        .companion
        .perform(load("s-d"), Some(front.session.clone()), None)
        .await
        .expect("the request");
    assert!(matches!(refused, Err(CallRefusal::App(_))), "{refused:?}");
    for id in ["s-a"] {
        let again = desk
            .companion
            .perform(load(id), Some(front.session.clone()), None)
            .await
            .expect("the request");
        assert!(again.is_ok(), "{again:?}");
    }
    for id in ["s-missing", "nothing-here"] {
        let hidden = desk
            .companion
            .perform(load(id), Some(front.session.clone()), None)
            .await
            .expect("the request");
        assert!(
            matches!(hidden, Err(CallRefusal::App(_))),
            "{id}: {hidden:?}"
        );
    }

    // The budget is per task: another session starts with none loaded.
    let second = desk.front().await;
    let fresh = desk
        .companion
        .perform(load("s-d"), Some(second.session), None)
        .await
        .expect("the request");
    assert!(fresh.is_ok(), "{fresh:?}");
}
