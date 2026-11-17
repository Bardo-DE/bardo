//! Where the assistant may run, end to end on a private bus: `Inference1.Places` lists this
//! computer, the person's own computers and the cloud accounts, for the three callers that may
//! ask and nobody else.

use crate::hosting;

use account_callers::Caller;
use account_capability::ModelId;
use account_capability::Tier;
use account_core::{AppId, AppName, DataClass, Isolation};
use caller_role_types::CallerRole;
use hosting::accountd::{FakeAccount, Standing};
use hosting::engine::{Chat, Script};
use hosting::entries;
use hosting::lab::Lab;
use hosting::rig::{Hosted, Plan, Trust, World};
use inference_client::{ClientError, TransportError};
use inference_core::{ComputerName, PlaceKind, PlaceRow, PlaceState, Takes};
use inference_daemon::{Attached, Place, Reach};

fn app(name: &str) -> AppId {
    AppId {
        name: AppName::parse(name).expect("name"),
        isolation: Isolation::Unsandboxed,
    }
}

/// A world with one model on this computer, a lab machine called `lab` with one model, and two
/// cloud accounts: a working one and one that has to be signed in again.
async fn world(lab_dir: &str, role: CallerRole, who: &str) -> (World, Lab) {
    let dir = base_testkit::Scratch::new(&format!("pl-{lab_dir}"));
    let lab = Lab::start_in(dir, "lab", &[entries::SERVED], None).await;
    let plan = Plan {
        catalog: vec![
            ("a-attached.toml", entries::attached()),
            ("b-chat.toml", entries::chat()),
            ("c-claude.toml", entries::claude()),
            ("d-luna.toml", entries::luna()),
        ],
        attached: vec![Attached {
            id: ModelId::parse(entries::ATTACHED).expect("id"),
            reach: Reach::Socket(lab.socket()),
            key_file: None,
            place: Place::MyNetwork,
            computer: Some(ComputerName::parse("lab").expect("name")),
        }],
        role,
        app: Some(app(who)),
        hosted: Some(Hosted {
            accounts: vec![
                FakeAccount {
                    id: "openrouter",
                    standing: Standing::Ask,
                },
                FakeAccount {
                    id: "openai",
                    standing: Standing::Ask,
                },
            ],
            chat: Script {
                chat: vec![Chat::Say(vec!["hi"])],
                dims: 0,
            },
            trust: Trust::ScratchCa,
        }),
        ..Plan::default()
    };
    let world = World::start(plan).await;
    let accountd = world.accountd.as_ref().expect("accountd");
    accountd.describe("openrouter", "Spare router", Some("OpenRouter"), "ok");
    accountd.describe("openai", "Work OpenAI", Some("OpenAI"), "needs_reauth");
    (world, lab)
}

fn row<'a>(rows: &'a [PlaceRow], id: &str) -> &'a PlaceRow {
    rows.iter()
        .find(|one| one.id.as_str() == id)
        .unwrap_or_else(|| panic!("no place {id} in {:?}", ids(rows)))
}

fn ids(rows: &[PlaceRow]) -> Vec<&str> {
    rows.iter().map(|one| one.id.as_str()).collect()
}

fn models(row: &PlaceRow) -> Vec<&str> {
    row.models.iter().map(|one| one.id.as_str()).collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_lists_this_computer_a_named_computer_and_the_cloud_accounts() {
    let (world, _lab) = world("list", CallerRole::Settings, "org.bardo.Settings").await;
    let rows = world.accounts.places().await.expect("places");
    assert_eq!(
        ids(&rows),
        vec![
            "this-computer",
            "computer:lab",
            "account:openrouter",
            "account:openai"
        ]
    );
    let here = row(&rows, "this-computer");
    assert_eq!(here.kind, PlaceKind::ThisComputer);
    assert_eq!(here.provider, None);
    assert_eq!(here.state, PlaceState::Ready);
    assert_eq!(models(here), vec!["tiny-chat"]);
    assert_eq!(here.models[0].name, "Tiny chat");
    assert_eq!(here.models[0].takes, Takes::TextAndImages);
    let engine = here.models[0]
        .engine
        .as_ref()
        .expect("the engine inferd runs it with");
    assert_eq!(
        (engine.id.as_str(), engine.name.as_str()),
        ("llama_cpp", "llama.cpp")
    );

    let lab = row(&rows, "computer:lab");
    assert_eq!(lab.kind, PlaceKind::OwnComputer);
    assert_eq!(lab.name, "lab");
    assert_eq!(lab.state, PlaceState::Ready);
    assert_eq!(models(lab), vec![entries::ATTACHED]);
    assert_eq!(lab.models[0].engine, None);

    let router = row(&rows, "account:openrouter");
    assert_eq!(router.kind, PlaceKind::CloudAccount);
    assert_eq!(router.name, "Spare router");
    assert_eq!(router.provider.as_deref(), Some("OpenRouter"));
    assert_eq!(router.state, PlaceState::Ready);
    assert_eq!(models(router), vec!["claude-opus-5.5", "gpt-6-luna"]);
    assert_eq!(router.models[0].name, "Claude Opus 5.5");
    assert_eq!(router.models[0].takes, Takes::TextAndImages);

    // An account that must be signed in again is a place, but serves nothing now.
    let stale = row(&rows, "account:openai");
    assert_eq!(stale.name, "Work OpenAI");
    assert_eq!(stale.state, PlaceState::NotReady);
    assert_eq!(models(stale), Vec::<&str>::new());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_computer_that_does_not_answer_is_listed_but_not_ready() {
    let (world, lab) = world("down", CallerRole::Settings, "org.bardo.Settings").await;
    drop(lab);
    let rows = world.accounts.places().await.expect("places");
    let down = row(&rows, "computer:lab");
    assert_eq!(down.state, PlaceState::NotReady);
    assert_eq!(models(down), Vec::<&str>::new());
}

#[tokio::test(flavor = "multi_thread")]
async fn only_settings_the_shell_the_companion_and_cuad_may_list_the_places() {
    let (world, _lab) = world("callers", CallerRole::Settings, "org.bardo.Settings").await;
    let me = world.client.unique_name().expect("unique name").to_string();
    let cases = [
        (CallerRole::Settings, "org.bardo.Settings", true),
        (CallerRole::SheetHost, "org.bardo.Shell", true),
        (CallerRole::Agent, "org.bardo.Companion", true),
        (CallerRole::Agent, "org.bardo.Reader", false),
        (CallerRole::Agent, "org.bardo.Intents", false),
        (CallerRole::App, "org.bardo.Memory", false),
        (CallerRole::Cua, "org.bardo.ComputerUse", true),
        (CallerRole::AgentLauncher, "org.bardo.AgentLauncher", false),
    ];
    for (role, name, allowed) in cases {
        world.peers.introduce(
            &me,
            Caller {
                app: app(name),
                role,
            },
        );
        let got = world.accounts.places().await;
        match (allowed, got) {
            (true, Ok(rows)) => assert!(!rows.is_empty(), "{name}"),
            (false, Err(ClientError::Transport(TransportError::Denied(_)))) => {}
            (_, other) => panic!("{name} as {role:?}: {other:?}"),
        }
    }
    // Nobody the table names is refused too.
    let stranger = world.bus.connect().await;
    let accounts =
        inference_client::Inference::over(inference_client::DbusTransport::over(stranger));
    assert!(matches!(
        accounts.places().await,
        Err(ClientError::Transport(TransportError::Denied(_)))
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_shell_may_list_the_places_and_ask_for_no_model() {
    let (world, _lab) = world("shell", CallerRole::SheetHost, "org.bardo.Shell").await;
    world.accounts.places().await.unwrap();
    let need = account_match::Need::Llm(account_match::LlmNeed::new(
        [account_capability::LlmFeature::Chat].into(),
        base_ids::Tokens(1000),
    ));
    let opened = world
        .accounts
        .session(&need, DataClass::Public, Tier::Balanced)
        .await;
    assert!(
        matches!(
            opened,
            Err(ClientError::Transport(TransportError::Denied(_)))
        ),
        "{:?}",
        opened.map(|_| ())
    );
    let prepared = world
        .accounts
        .prepare(
            &need,
            DataClass::Public,
            Tier::Balanced,
            &inference_core::OpenOptions::default(),
        )
        .await;
    assert!(matches!(
        prepared,
        Err(ClientError::Transport(TransportError::Denied(_)))
    ));
}

/// An account that appears, goes or changes state is told to inferd by its own key store (the
/// news it wakes after a save); inferd says `EnginesChanged`, so a listener re-reads `Places`.
/// The fake key store wakes the same news.
#[tokio::test(flavor = "multi_thread")]
async fn an_account_that_appears_makes_inferd_say_engines_changed() {
    use account_dbus::BusStream;
    use inference_dbus::inference::InferenceProxy;
    let (world, _lab) = world("news", CallerRole::Settings, "org.bardo.Settings").await;
    let proxy = InferenceProxy::new(&world.client).await.expect("proxy");
    let mut changed = proxy.receive_engines_changed().await.expect("stream");
    let keys = world.accountd.as_ref().expect("the key store");
    let heard = within_generous(async {
        loop {
            keys.changed();
            let next = std::future::poll_fn(|cx| std::pin::Pin::new(&mut changed).poll_next(cx));
            if let Ok(Some(_)) =
                tokio::time::timeout(std::time::Duration::from_millis(150), next).await
            {
                return;
            }
        }
    })
    .await;
    assert!(heard, "EnginesChanged after an account appeared");
}

async fn within_generous(work: impl std::future::Future<Output = ()>) -> bool {
    tokio::time::timeout(base_testkit::GENEROUS, work)
        .await
        .is_ok()
}

// ---- routing inside the allowed set -------------------------------------------------------

mod routing {
    use super::*;
    use account_capability::LlmFeature;
    use account_match::LlmNeed;
    use account_match::Need;
    use base_ids::Tokens;
    use inference_core::{OpenOptions, PlaceId, Policy};
    use inference_dbus::inference::InferenceProxy;

    const COMPANION: &str = "org.bardo.Companion";

    fn llm() -> Need {
        Need::Llm(LlmNeed::new([LlmFeature::Chat].into(), Tokens(1000)))
    }

    fn place(text: &str) -> PlaceId {
        PlaceId::parse(text).expect("place")
    }

    /// A local chat model, a lab machine called `lab` with one model, and `OpenRouter` granted to
    /// the companion; the caller is the companion (a placer) unless the test says otherwise.
    pub async fn placed_world(dir: &str, role: CallerRole, who: &str) -> (World, Lab) {
        placed_world_with(dir, role, who, true).await
    }

    /// `placed_world`, with the local chat model only when `local` says so.
    pub async fn placed_world_with(
        dir: &str,
        role: CallerRole,
        who: &str,
        local: bool,
    ) -> (World, Lab) {
        let scratch = base_testkit::Scratch::new(&format!("pr-{dir}"));
        let lab = Lab::start_in(scratch, "lab", &[entries::SERVED], None).await;
        let mut catalog = vec![
            ("a-attached.toml", entries::attached()),
            ("c-claude.toml", entries::claude()),
        ];
        let mut scripts = Vec::new();
        if local {
            catalog.push(("b-chat.toml", entries::chat()));
            scripts.push((
                "tiny-chat",
                Script {
                    chat: vec![Chat::Say(vec!["local"])],
                    dims: 0,
                },
            ));
        }
        let plan = Plan {
            catalog,
            scripts,
            attached: vec![Attached {
                id: ModelId::parse(entries::ATTACHED).expect("id"),
                reach: Reach::Socket(lab.socket()),
                key_file: None,
                place: Place::MyNetwork,
                computer: Some(ComputerName::parse("lab").expect("name")),
            }],
            policy: Policy {
                local_only: false,
                floors: Vec::new(),
            },
            role,
            app: Some(app(who)),
            hosted: Some(Hosted {
                accounts: vec![FakeAccount {
                    id: "openrouter",
                    standing: Standing::Granted {
                        to: vec![COMPANION, "org.bardo.Memory"],
                        grant: "grant-openrouter",
                        key: Some("sk-or-test-key"),
                    },
                }],
                chat: Script {
                    chat: vec![Chat::Say(vec!["cloud"])],
                    dims: 0,
                },
                trust: Trust::ScratchCa,
            }),
            ..Plan::default()
        };
        (World::start(plan).await, lab)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn callers_that_may_not_choose_places_are_refused_not_ignored() {
        let (world, _lab) =
            placed_world("refused", CallerRole::Settings, "org.bardo.Settings").await;
        let me = world.client.unique_name().expect("unique name").to_string();
        let options = OpenOptions::default().with_places(vec![place("this-computer")]);
        for (role, name) in [
            (CallerRole::Settings, "org.bardo.Settings"),
            (CallerRole::App, "org.bardo.Memory"),
            (CallerRole::Cua, "org.bardo.ComputerUse"),
        ] {
            world.peers.introduce(
                &me,
                account_callers::Caller {
                    app: app(name),
                    role,
                },
            );
            let denied = |what: &str, got: Result<(), ClientError>| {
                assert!(
                    matches!(got, Err(ClientError::Transport(TransportError::Denied(_)))),
                    "{name} {what}: {got:?}"
                );
            };
            denied(
                "open",
                world
                    .accounts
                    .session_with(&llm(), DataClass::Public, Tier::Balanced, &options)
                    .await
                    .map(|_| ()),
            );
            denied(
                "prepare",
                world
                    .accounts
                    .prepare(&llm(), DataClass::Public, Tier::Balanced, &options)
                    .await
                    .map(|_| ()),
            );
            // The same call without the key is served.
            assert!(
                world
                    .accounts
                    .prepare(
                        &llm(),
                        DataClass::Public,
                        Tier::Balanced,
                        &OpenOptions::default()
                    )
                    .await
                    .is_ok(),
                "{name}"
            );
        }
        // A placer is let through.
        world.peers.introduce(
            &me,
            account_callers::Caller {
                app: app(COMPANION),
                role: CallerRole::Agent,
            },
        );
        world
            .accounts
            .prepare(&llm(), DataClass::Public, Tier::Balanced, &options)
            .await
            .unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_places_option_that_is_not_what_the_interface_says_is_invalid_args() {
        let (world, _lab) = placed_world("invalid", CallerRole::Agent, COMPANION).await;
        let proxy = InferenceProxy::new(&world.client).await.expect("proxy");
        let need = inference_dbus::inference::need_to_dbus(&llm());
        let text = |value: &str| {
            account_dbus::zvariant::OwnedValue::try_from(account_dbus::zvariant::Value::from(
                value.to_owned(),
            ))
            .expect("value")
        };
        let list = |values: &[&str]| {
            account_dbus::zvariant::OwnedValue::try_from(account_dbus::zvariant::Value::new(
                values.iter().map(|v| (*v).to_owned()).collect::<Vec<_>>(),
            ))
            .expect("value")
        };
        let pins = |place: &str, model: &str| {
            account_dbus::zvariant::OwnedValue::try_from(account_dbus::zvariant::Value::new(
                std::collections::HashMap::from([(place.to_owned(), model.to_owned())]),
            ))
            .expect("value")
        };
        let cases: Vec<(&str, account_dbus::Details)> = vec![
            (
                "a bad place id",
                [("places".to_owned(), list(&["somewhere"]))].into(),
            ),
            (
                "places as text",
                [("places".to_owned(), text("this-computer"))].into(),
            ),
            (
                "pins without places",
                [("place_models".to_owned(), pins("this-computer", "m"))].into(),
            ),
            (
                "a pin outside the set",
                [
                    ("places".to_owned(), list(&["this-computer"])),
                    ("place_models".to_owned(), pins("account:x", "m")),
                ]
                .into(),
            ),
            (
                "a pin to a bad model",
                [
                    ("places".to_owned(), list(&["this-computer"])),
                    (
                        "place_models".to_owned(),
                        pins("this-computer", "Not A Model"),
                    ),
                ]
                .into(),
            ),
        ];
        for (what, options) in cases {
            let got = proxy
                .availability(&need, "public", &options)
                .await
                .expect_err(what);
            assert!(account_dbus::is_invalid_args(&got), "{what}: {got:?}");
        }
    }
}

// ---- the typed refusal ---------------------------------------------------------------------

mod refusals {
    use super::routing::placed_world_with;
    use super::*;
    use account_capability::LlmFeature;
    use account_match::LlmNeed;
    use account_match::Need;
    use base_ids::Tokens;
    use inference_dbus::inference::{InferenceProxy, PLACE_ERROR_PREFIX, place_refusal_of};

    const COMPANION: &str = "org.bardo.Companion";

    fn llm() -> Need {
        Need::Llm(LlmNeed::new([LlmFeature::Chat].into(), Tokens(1000)))
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_reply_is_an_error_named_for_the_reason_with_plain_words_and_the_kind_in_its_body()
    {
        let (world, lab) = placed_world_with("r-wire", CallerRole::Agent, COMPANION, false).await;
        drop(lab);
        let proxy = InferenceProxy::new(&world.client).await.expect("proxy");
        let options = match account_dbus::zvariant::OwnedValue::try_from(
            account_dbus::zvariant::Value::new(vec!["computer:lab".to_owned()]),
        ) {
            Ok(value) => [("places".to_owned(), value)].into(),
            Err(e) => panic!("{e}"),
        };
        let need = inference_dbus::inference::need_to_dbus(&llm());
        let error = proxy
            .availability(&need, "public", &options)
            .await
            .expect_err("no allowed place");
        let zbus::Error::MethodError(name, text, _) = &error else {
            panic!("a method error, got {error:?}");
        };
        assert_eq!(
            name.as_str(),
            format!("{PLACE_ERROR_PREFIX}NotReady"),
            "reason-specific name"
        );
        assert_eq!(
            name.as_str(),
            "org.bardo.Inference1.Error.NoAllowedPlace.NotReady"
        );
        let words = text.as_deref().expect("words");
        assert_eq!(
            words,
            "None of the places you allowed is ready to do this right now."
        );
        for jargon in ["would_need", "cloud_account", "NoAllowedPlace", "="] {
            assert!(!words.contains(jargon), "{words}");
        }
        assert_eq!(
            place_refusal_of(&error),
            Some(("NotReady".to_owned(), Some("cloud_account".to_owned())))
        );
        // And the body is exactly two strings: the words, then the kind.
        let zbus::Error::MethodError(_, _, message) = &error else {
            unreachable!()
        };
        let body: (String, String) = message.body().deserialize().expect("two strings");
        assert_eq!(body.1, "cloud_account");
        // Another error of the same interface is not mistaken for one.
        let denied = proxy
            .availability(&need, "no-such-class", &account_dbus::Details::new())
            .await
            .expect_err("invalid");
        assert_eq!(place_refusal_of(&denied), None);
    }
}

// ---- computers added in Settings -----------------------------------------------------------

mod computers {
    use super::routing::placed_world_with;
    use super::*;
    use account_dbus::zvariant::{OwnedValue, Value};
    use account_dbus::{BusStream, Details};
    use hosting::bus::within;
    use inference_client::InferSession;
    use inference_core::{
        ChatMessage, ChatRequest, ClientFrame, InferEvent, InferRequest, MessagePart, OpenOptions,
        PlaceId, Policy, Role as ChatRole,
    };
    use inference_dbus::inference::COMPUTER_ERROR_PREFIX;
    use inference_dbus::inference::InferenceProxy;
    use std::path::Path;

    const KEY: &str = "sk-lab-S3CRET-0123456789";
    const COMPANION: &str = "org.bardo.Companion";

    fn text(value: &str) -> OwnedValue {
        OwnedValue::try_from(Value::from(value.to_owned())).expect("value")
    }

    /// One model of `AddComputer`: its id, and how it is reached and its key.
    fn model(
        id: &str,
        socket: Option<&Path>,
        port: Option<u16>,
        key: Option<&str>,
    ) -> (String, Details) {
        let mut details = Details::new();
        if let Some(socket) = socket {
            details.insert("socket".to_owned(), text(&socket.display().to_string()));
        }
        if let Some(port) = port {
            details.insert(
                "port".to_owned(),
                OwnedValue::try_from(Value::U16(port)).expect("value"),
            );
        }
        if let Some(key) = key {
            details.insert("key".to_owned(), text(key));
        }
        (id.to_owned(), details)
    }

    /// A world with no engine written by hand, and a lab machine that wants `KEY`.
    async fn world(name: &str) -> (World, Lab) {
        let dir = base_testkit::Scratch::new(&format!("cp-{name}"));
        let lab = Lab::start_in(dir, "lab", &[entries::SERVED], Some(KEY)).await;
        let plan = Plan {
            catalog: vec![("a-attached.toml", entries::attached())],
            policy: Policy {
                local_only: false,
                floors: Vec::new(),
            },
            role: CallerRole::Settings,
            app: Some(app("org.bardo.Settings")),
            ..Plan::default()
        };
        (World::start(plan).await, lab)
    }

    fn as_caller(world: &World, role: CallerRole, who: &str) {
        let me = world.client.unique_name().expect("unique name").to_string();
        world.peers.introduce(
            &me,
            Caller {
                app: app(who),
                role,
            },
        );
    }

    async fn proxy(world: &World) -> InferenceProxy<'static> {
        InferenceProxy::new(&world.client).await.expect("proxy")
    }

    fn name_and_words(error: &zbus::Error) -> (String, String) {
        match error {
            zbus::Error::MethodError(name, words, _) => {
                (name.to_string(), words.clone().unwrap_or_default())
            }
            other => panic!("a method error, got {other:?}"),
        }
    }

    fn chat_request() -> InferRequest {
        InferRequest::Chat(ChatRequest::new(
            vec![ChatMessage {
                role: ChatRole::User,
                parts: vec![MessagePart::Text("hello".into())],
            }],
            Tier::Balanced,
            DataClass::Public,
            account_core::consent::Usage::Interactive,
        ))
    }

    fn files_holding(dir: &Path, needle: &str) -> Vec<std::path::PathBuf> {
        let mut found = Vec::new();
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                found.extend(files_holding(&path, needle));
            } else if std::fs::read_to_string(&path).is_ok_and(|text| text.contains(needle)) {
                found.push(path);
            }
        }
        found
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn settings_adds_a_computer_that_places_lists_and_the_assistant_uses_with_its_key() {
        let (world, lab) = world("add").await;
        lab.say(&["from the studio"]);
        let proxy = proxy(&world).await;
        let mut changed = proxy.receive_engines_changed().await.expect("stream");
        let placed = within(
            "AddComputer",
            proxy.add_computer(
                "Studio PC",
                vec![model(
                    entries::ATTACHED,
                    Some(&lab.socket()),
                    None,
                    Some(KEY),
                )],
            ),
        )
        .await
        .expect("added");
        assert_eq!(placed, "computer:studio-pc");
        let told = std::future::poll_fn(|cx| std::pin::Pin::new(&mut changed).poll_next(cx));
        assert!(
            tokio::time::timeout(base_testkit::GENEROUS, told)
                .await
                .is_ok(),
            "EnginesChanged after an addition"
        );

        // Places shows it, under the name the person gave, ready, with its model.
        let rows = world.accounts.places().await.expect("places");
        let row = rows
            .iter()
            .find(|row| row.id.as_str() == "computer:studio-pc")
            .unwrap_or_else(|| panic!("the new computer in {rows:?}"));
        assert_eq!(row.name, "Studio PC");
        assert_eq!(row.state, inference_core::PlaceState::Ready);
        assert_eq!(
            row.models.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
            vec![entries::ATTACHED]
        );

        // A companion that names the place is answered by it, and the lab saw the key.
        as_caller(&world, CallerRole::Agent, COMPANION);
        let options = OpenOptions::default()
            .with_places(vec![PlaceId::parse("computer:studio-pc").expect("place")]);
        let need = account_match::Need::Llm(account_match::LlmNeed::new(
            [account_capability::LlmFeature::Chat].into(),
            base_ids::Tokens(1000),
        ));
        let mut session = within(
            "Open",
            world
                .accounts
                .session_with(&need, DataClass::Public, Tier::Balanced, &options),
        )
        .await
        .expect("open");
        let _ = session.send(ClientFrame::Request(chat_request())).await;
        loop {
            let event = within("an event", session.next()).await.expect("event");
            if matches!(event, InferEvent::Finished(_)) {
                break;
            }
        }
        assert_eq!(lab.chats().len(), 1);
        let bearer = format!("Bearer {KEY}");
        assert!(
            lab.seen()
                .iter()
                .any(|seen| seen.authorization.as_deref() == Some(bearer.as_str())),
            "the lab was sent the key"
        );

        // The key is in one file, readable by its owner alone, and nowhere else: not in the
        // list of computers, not in what inferd logged, not in any reply.
        let key_file = world
            .state
            .join("computer-keys")
            .join(format!("studio-pc--{}.key", entries::ATTACHED));
        assert_eq!(std::fs::read_to_string(&key_file).expect("key file"), KEY);
        let mode = std::os::unix::fs::PermissionsExt::mode(
            &std::fs::metadata(&key_file).expect("meta").permissions(),
        ) & 0o777;
        assert_eq!(mode, 0o600);
        assert_eq!(files_holding(&world.scratch, KEY), vec![key_file]);
        let lines = format!("{:?}", world.log.lock().expect("lock"));
        assert!(!lines.contains(KEY), "{lines}");
        assert!(!format!("{rows:?} {placed}").contains(KEY));
        let list = std::fs::read_to_string(world.state.join("computers.toml")).expect("list");
        assert!(list.contains("Studio PC") && !list.contains(KEY), "{list}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn removing_a_computer_takes_it_and_its_key_away_and_says_engines_changed() {
        let (world, lab) = world("remove").await;
        let proxy = proxy(&world).await;
        proxy
            .add_computer(
                "Studio PC",
                vec![model(
                    entries::ATTACHED,
                    Some(&lab.socket()),
                    None,
                    Some(KEY),
                )],
            )
            .await
            .expect("added");
        let mut changed = proxy.receive_engines_changed().await.expect("stream");
        proxy
            .remove_computer("computer:studio-pc")
            .await
            .expect("removed");
        let told = std::future::poll_fn(|cx| std::pin::Pin::new(&mut changed).poll_next(cx));
        assert!(
            tokio::time::timeout(base_testkit::GENEROUS, told)
                .await
                .is_ok(),
            "EnginesChanged after a removal"
        );
        let rows = world.accounts.places().await.expect("places");
        assert!(
            rows.iter()
                .all(|row| row.id.as_str() != "computer:studio-pc")
        );
        assert!(
            !world
                .state
                .join("computer-keys")
                .join(format!("studio-pc--{}.key", entries::ATTACHED))
                .exists()
        );
        // Gone is gone.
        let again = proxy
            .remove_computer("studio-pc")
            .await
            .expect_err("not there");
        assert_eq!(
            name_and_words(&again),
            (
                format!("{COMPUTER_ERROR_PREFIX}NotThere"),
                "There is no computer with that name.".to_owned()
            )
        );
        // And the same name can be used again.
        proxy
            .add_computer(
                "Studio PC",
                vec![model(entries::ATTACHED, Some(&lab.socket()), None, None)],
            )
            .await
            .unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    #[expect(
        clippy::too_many_lines,
        reason = "a table of refusals and the sentence each one reaches Settings as"
    )]
    async fn what_is_refused_reaches_settings_as_a_named_error_with_a_plain_sentence() {
        type Case<'a> = (&'a str, Vec<(String, Details)>, &'a str, &'a str);
        let (world, lab) = world("refuse").await;
        let proxy = proxy(&world).await;
        let socket = lab.socket();
        let studio = |models: Vec<(String, Details)>| {
            let proxy = proxy.clone();
            async move { proxy.add_computer("Studio PC", models).await }
        };
        let unreachable = format!(
            "\u{201c}{}\u{201d} cannot be reached that way. Give the full path of a connection on this computer, or a port on this computer.",
            entries::ATTACHED
        );
        let bad_key = format!(
            "The key for \u{201c}{}\u{201d} cannot be used.",
            entries::ATTACHED
        );
        let cases: Vec<Case<'_>> = vec![
            (
                "no models",
                vec![],
                "NoModels",
                "Add at least one model for this computer.",
            ),
            (
                "an unknown model",
                vec![model("no-such-model", Some(&socket), None, None)],
                "UnknownModel",
                "The assistant does not know a model called \u{201c}no-such-model\u{201d}.",
            ),
            (
                "no way to reach it",
                vec![model(entries::ATTACHED, None, None, None)],
                "BadAddress",
                &unreachable,
            ),
            (
                "two ways to reach it",
                vec![model(entries::ATTACHED, Some(&socket), Some(8000), None)],
                "BadAddress",
                &unreachable,
            ),
            (
                "a relative path",
                vec![model(
                    entries::ATTACHED,
                    Some(Path::new("lab.sock")),
                    None,
                    None,
                )],
                "BadAddress",
                &unreachable,
            ),
            (
                "an empty key",
                vec![model(entries::ATTACHED, Some(&socket), None, Some(" "))],
                "BadKey",
                &bad_key,
            ),
        ];
        for (what, models, name, words) in cases {
            let error = studio(models).await.expect_err(what);
            assert_eq!(
                name_and_words(&error),
                (format!("{COMPUTER_ERROR_PREFIX}{name}"), words.to_owned()),
                "{what}"
            );
            for jargon in ["socket", "loopback", "TOML", "key_file", "API"] {
                assert!(!words.contains(jargon), "{what}: {words}");
            }
        }
        // A blank name, and a name taken.
        let blank = proxy
            .add_computer(
                "  ",
                vec![model(entries::ATTACHED, Some(&socket), None, None)],
            )
            .await
            .expect_err("blank");
        assert_eq!(
            name_and_words(&blank),
            (
                format!("{COMPUTER_ERROR_PREFIX}BadName"),
                "Give the computer a name of one short line.".to_owned()
            )
        );
        studio(vec![model(entries::ATTACHED, Some(&socket), None, None)])
            .await
            .expect("added");
        let taken = proxy
            .add_computer(
                "studio pc",
                vec![model(entries::ATTACHED, Some(&socket), None, None)],
            )
            .await
            .expect_err("taken");
        assert_eq!(
            name_and_words(&taken),
            (
                format!("{COMPUTER_ERROR_PREFIX}AlreadyThere"),
                "A computer with that name is already there.".to_owned()
            )
        );
        // Nothing refused left anything behind: one computer, one key file or none.
        let rows = world.accounts.places().await.expect("places");
        assert_eq!(
            rows.iter()
                .filter(|row| row.kind == PlaceKind::OwnComputer)
                .count(),
            1
        );
        // A value of the wrong type is invalid args, not one of ours.
        let mut wrong = model(entries::ATTACHED, None, None, None);
        wrong.1.insert("port".to_owned(), text("eight thousand"));
        let invalid = proxy
            .add_computer("Other", vec![wrong])
            .await
            .expect_err("invalid");
        assert!(account_dbus::is_invalid_args(&invalid), "{invalid:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_computer_written_by_hand_is_listed_but_cannot_be_removed_or_added_again() {
        let (world, _lab) =
            placed_world_with("hand", CallerRole::Settings, "org.bardo.Settings", false).await;
        let proxy = proxy(&world).await;
        let rows = world.accounts.places().await.expect("places");
        assert!(rows.iter().any(|row| row.id.as_str() == "computer:lab"));
        for name in ["lab", "computer:lab"] {
            let error = proxy.remove_computer(name).await.expect_err("by hand");
            assert_eq!(
                name_and_words(&error),
                (
                    format!("{COMPUTER_ERROR_PREFIX}AddedByHand"),
                    "That computer was added by hand in the settings file, so it can only be removed there."
                        .to_owned()
                ),
                "{name}"
            );
        }
        let again = proxy
            .add_computer("Lab", vec![model("other-model", None, Some(1), None)])
            .await
            .expect_err("taken");
        assert_eq!(
            name_and_words(&again).0,
            format!("{COMPUTER_ERROR_PREFIX}AlreadyThere")
        );
        let model_taken = proxy
            .add_computer("Bench", vec![model(entries::ATTACHED, None, Some(1), None)])
            .await
            .expect_err("model taken");
        assert_eq!(
            name_and_words(&model_taken).0,
            format!("{COMPUTER_ERROR_PREFIX}ModelTaken")
        );
        world.accounts.places().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn only_settings_may_add_or_remove_a_computer() {
        let (world, lab) = world("callers").await;
        let proxy = proxy(&world).await;
        let denied = "org.freedesktop.DBus.Error.AccessDenied";
        for (role, who) in [
            (CallerRole::Agent, COMPANION),
            (CallerRole::SheetHost, "org.bardo.Shell"),
            (CallerRole::App, "org.bardo.Memory"),
            (CallerRole::AgentLauncher, "org.bardo.AgentLauncher"),
        ] {
            as_caller(&world, role, who);
            let add = proxy
                .add_computer(
                    "Studio PC",
                    vec![model(entries::ATTACHED, Some(&lab.socket()), None, None)],
                )
                .await
                .expect_err(who);
            assert_eq!(name_and_words(&add).0, denied, "{who} add");
            let remove = proxy.remove_computer("studio-pc").await.expect_err(who);
            assert_eq!(name_and_words(&remove).0, denied, "{who} remove");
        }
        as_caller(&world, CallerRole::Settings, "org.bardo.Settings");
        let rows = world.accounts.places().await.expect("places");
        assert!(rows.iter().all(|row| row.kind != PlaceKind::OwnComputer));
    }
}
