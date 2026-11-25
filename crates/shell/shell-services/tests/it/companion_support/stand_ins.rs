//! The stand-ins the companion service talks to on a private bus: actiond's session, search,
//! control and run objects, companiond and its answer, an app that serves the summon, the computer-use daemon's
//! manager and runs, and memoryd. Each records what it was asked on the shared [`Log`].

use std::sync::{Arc, Mutex};

use action_core::{Hit, UndoEntry, UndoFault};
use action_dbus::{IntentsError, answer_path};
use action_wire::{Displayed, SessionOpened, SummonAnswer};
use memory_core::{FactView, TimelineEntry, TimelinePage};
use permission_core::Mode;
use provenance::{DataClass, Label, SessionId, Source, SpaceId};
use zbus::fdo;
use zbus::message::Header;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{ObjectPath, OwnedObjectPath};
use zbus::{Connection, interface};

use super::{Details, Log, json, task};
use action_core::{CallRefusal, Delivery, Follow, Outcome, Preview, SendRefusal, Undoable};
use action_manifest::ValidManifest;
use provenance::{MessageId, ThreadId};

// ---- actiond ----

pub struct StandInSession(pub Log);

#[interface(name = "org.bardo.Intents1.Session")]
#[expect(
    clippy::unnecessary_wraps,
    clippy::needless_pass_by_value,
    reason = "the zbus interface macro has methods answer with `fdo::Result` so a service can refuse and hands a method its arguments by value, whatever this one holds or does"
)]
impl StandInSession {
    fn turn(&self, session: String, turn: String, options: Details) -> fdo::Result<u64> {
        let _ = options;
        self.0.push(format!("turn {session} {turn}"));
        Ok(41)
    }

    /// The labelled reading the shell asks for first. Handle [`MAIL_HANDLE`] holds somebody
    /// else's words (mail); every other handle is the person's own.
    fn display_labelled(&self, session: String, handle: u64) -> fdo::Result<String> {
        self.0.push(format!("display {session} {handle}"));
        if self.0.lines().contains(&format!("close {session}")) {
            return Err(fdo::Error::Failed("NoSuchSession".to_owned()));
        }
        let label = match handle {
            MAIL_HANDLE => Label::untrusted(Source::Mail, DataClass::Mail, SpaceId::desktop()),
            _ => Label::trusted_user(),
        };
        let shown = Displayed {
            text: format!("the text of handle {handle}"),
            label,
        };
        Ok(json(&shown))
    }
}

/// What the stand-in router answers a switch of mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchAnswer {
    /// Done, and `ModeChanged` follows.
    Done,
    /// An administrator holds every conversation in Manual.
    Locked,
}

/// A router that records turns and switches of mode, and answers the switches as told.
pub struct StandInModeSession {
    pub log: Log,
    pub answer: Arc<Mutex<SwitchAnswer>>,
}

#[interface(name = "org.bardo.Intents1.Session")]
#[expect(
    clippy::unnecessary_wraps,
    clippy::needless_pass_by_value,
    reason = "the zbus interface macro has methods answer with `fdo::Result` so a service can refuse and hands a method its arguments by value, whatever this one holds or does"
)]
impl StandInModeSession {
    fn turn(&self, session: String, turn: String, options: Details) -> fdo::Result<u64> {
        let _ = options;
        self.log.push(format!("turn {session} {turn}"));
        Ok(41)
    }

    async fn set_mode(
        &self,
        session: String,
        mode: String,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<(), IntentsError> {
        self.log.push(format!("set_mode {session} {mode}"));
        let answer = *self.answer.lock().unwrap_or_else(|e| panic!("{e}"));
        match answer {
            SwitchAnswer::Locked => Err(IntentsError::Locked(String::new())),
            SwitchAnswer::Done => Self::mode_changed(&emitter, &session, &mode)
                .await
                .map_err(IntentsError::ZBus),
        }
    }

    #[zbus(signal)]
    async fn mode_changed(
        emitter: &SignalEmitter<'_>,
        session: &str,
        mode: &str,
    ) -> zbus::Result<()>;
}

/// The handle whose words are somebody else's.
pub const MAIL_HANDLE: u64 = 9;

/// A router from before the labelled reading: it has `Display` alone, so a `DisplayLabelled`
/// call is an unknown method.
pub struct StandInOlderSession(pub Log);

#[interface(name = "org.bardo.Intents1.Session")]
#[expect(
    clippy::unnecessary_wraps,
    clippy::needless_pass_by_value,
    reason = "the zbus interface macro has methods answer with `fdo::Result` so a service can refuse and hands a method its arguments by value, whatever this one holds or does"
)]
impl StandInOlderSession {
    fn turn(&self, session: String, turn: String, options: Details) -> fdo::Result<u64> {
        let _ = options;
        self.0.push(format!("turn {session} {turn}"));
        Ok(41)
    }

    fn display(&self, session: String, handle: u64) -> fdo::Result<String> {
        self.0.push(format!("display {session} {handle}"));
        Ok(format!("the text of handle {handle}"))
    }
}

/// What the stand-in message service answers a send: delivered, in the sender's Space.
pub struct StandInMessage(pub Log);

#[interface(name = "org.bardo.Intents1.Message")]
#[expect(
    clippy::unnecessary_wraps,
    clippy::needless_pass_by_value,
    reason = "the zbus interface macro has methods answer with `fdo::Result` so a service can refuse and hands a method its arguments by value, whatever this one holds or does"
)]
impl StandInMessage {
    fn send(&self, session: String, draft: String, options: Details) -> fdo::Result<String> {
        let _ = options;
        self.0.push(format!("send {session} {draft}"));
        let delivery: Result<Delivery, SendRefusal> = Ok(Delivery {
            message: MessageId::parse("m-1").unwrap_or_else(|e| panic!("{e}")),
            thread: ThreadId::parse("m-1").unwrap_or_else(|e| panic!("{e}")),
            crosses: false,
        });
        Ok(json(&delivery))
    }
}

/// The installed manifests, as the registry lists them.
pub struct StandInRegistry(pub Vec<ValidManifest>);

#[interface(name = "org.bardo.Intents1.Registry")]
impl StandInRegistry {
    fn manifests(&self) -> Vec<(String, String)> {
        self.0
            .iter()
            .map(|valid| (valid.manifest().app.as_str().to_owned(), json(valid)))
            .collect()
    }

    #[zbus(signal)]
    async fn manifest_changed(emitter: &SignalEmitter<'_>, app: &str) -> zbus::Result<()>;
}

#[derive(Clone)]
pub struct StandInSearch {
    pub log: Log,
    pub hits: Arc<Mutex<Vec<Hit>>>,
}

#[interface(name = "org.bardo.Intents1.Search")]
#[expect(
    clippy::unnecessary_wraps,
    clippy::needless_pass_by_value,
    reason = "the zbus interface macro has methods answer with `fdo::Result` so a service can refuse and hands a method its arguments by value, whatever this one holds or does"
)]
impl StandInSearch {
    fn query(&self, text: String, scope: String, generation: u64) -> fdo::Result<String> {
        let _ = scope;
        self.log.push(format!("search {text} {generation}"));
        let hits = self.hits.lock().unwrap_or_else(|e| panic!("{e}"));
        Ok(json(&*hits))
    }

    #[zbus(signal)]
    async fn hits(emitter: &SignalEmitter<'_>, generation: u64, hits: &str) -> zbus::Result<()>;
}

pub struct StandInControl(pub Arc<Mutex<Vec<UndoEntry>>>);

#[interface(name = "org.bardo.Intents1.Control")]
#[expect(
    clippy::unnecessary_wraps,
    clippy::needless_pass_by_value,
    reason = "the zbus interface macro has methods answer with `fdo::Result` so a service can refuse and hands a method its arguments by value, whatever this one holds or does"
)]
impl StandInControl {
    fn journal(&self, filter: String) -> fdo::Result<String> {
        let _ = filter;
        Ok(json(&*self.0.lock().unwrap_or_else(|e| panic!("{e}"))))
    }

    #[zbus(signal)]
    async fn journal_changed(emitter: &SignalEmitter<'_>, rows: u64) -> zbus::Result<()>;
}

pub struct StandInRun(pub Log);

#[interface(name = "org.bardo.Intents1.Run")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "the zbus interface macro hands a method its arguments by value, whatever this one holds or does"
)]
impl StandInRun {
    fn perform(
        &self,
        call: String,
        session: String,
        parent_window: String,
        options: Details,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
    ) -> fdo::Result<OwnedObjectPath> {
        let _ = options;
        self.0
            .push(format!("perform {call} {session} {parent_window}"));
        let path = OwnedObjectPath::try_from("/org/bardo/Intents1/request/2")
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;
        let to = header.sender().map(ToString::to_string);
        let (connection, at) = (connection.clone(), path.clone());
        tokio::spawn(async move {
            let end: Result<Outcome, CallRefusal> = Ok(Outcome {
                value: None,
                said: None,
                show: Preview::None,
                undo: Undoable::No,
                follow: Follow::Nothing,
            });
            let _ = connection
                .emit_signal(
                    to.as_deref(),
                    at.as_ref(),
                    "org.bardo.Intents1.Request",
                    "Response",
                    &(0u32, serde_json::to_string(&end).unwrap_or_default()),
                )
                .await;
        });
        Ok(path)
    }

    fn undo(
        &self,
        entry: u64,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
    ) -> fdo::Result<OwnedObjectPath> {
        self.0.push(format!("undo {entry}"));
        let path = OwnedObjectPath::try_from("/org/bardo/Intents1/request/1")
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;
        let to = header.sender().map(ToString::to_string);
        let (connection, at) = (connection.clone(), path.clone());
        tokio::spawn(async move {
            let end: Result<(), UndoFault> = Ok(());
            let _ = connection
                .emit_signal(
                    to.as_deref(),
                    at.as_ref(),
                    "org.bardo.Intents1.Request",
                    "Response",
                    &(0u32, serde_json::to_string(&end).unwrap_or_default()),
                )
                .await;
        });
        Ok(path)
    }
}

// ---- companiond ----

/// Whether the stand-in companion takes a card press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActsAs {
    /// It answers with the request's path.
    Accepts,
    /// It answers `AccessDenied`.
    Refuses,
}

pub struct StandInCompanion {
    /// What the answers it makes say as their `Session`.
    pub session: Option<String>,
    pub log: Log,
    pub view: Arc<Mutex<String>>,
    pub acts: ActsAs,
    /// What the answers it makes say as their `Routing`.
    pub routing: Option<Arc<Mutex<String>>>,
}

#[interface(name = "org.bardo.Companion1")]
#[expect(
    clippy::unnecessary_wraps,
    clippy::needless_pass_by_value,
    reason = "the zbus interface macro has methods answer with `fdo::Result` so a service can refuse and hands a method its arguments by value, whatever this one holds or does"
)]
impl StandInCompanion {
    fn prepare(&self, options: Details) {
        let _ = options;
        self.log.push("prepare");
    }

    fn open(&self, open: String) -> fdo::Result<String> {
        self.log.push(format!("open {open}"));
        Ok(json(&SessionOpened {
            session: SessionId::parse("s-1").unwrap_or_else(|e| panic!("{e}")),
            task: task(),
            mode: Mode::Auto,
        }))
    }

    async fn ask(
        &self,
        ask: String,
        options: Details,
        #[zbus(object_server)] server: &zbus::ObjectServer,
    ) -> fdo::Result<OwnedObjectPath> {
        let _ = options;
        self.log.push(format!("ask {ask}"));
        let path = OwnedObjectPath::try_from(answer_path(&task()))
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;
        server
            .at(
                path.clone(),
                StandInAnswer {
                    session: self.session.clone(),
                    log: self.log.clone(),
                    view: Arc::clone(&self.view),
                    acts: self.acts,
                    routing: self.routing.clone(),
                },
            )
            .await?;
        Ok(path)
    }

    fn told(&self, agent: String, space: String, turn: String) {
        self.log.push(format!("told {agent} {space} {turn}"));
    }

    fn close(&self, session: String) {
        self.log.push(format!("close {session}"));
    }

    #[zbus(signal)]
    async fn answer_added(emitter: &SignalEmitter<'_>, answer: ObjectPath<'_>) -> zbus::Result<()>;
}

pub struct StandInAnswer {
    /// The `Session` property; `None` is an older companion, whose object has no such property.
    pub session: Option<String>,
    pub log: Log,
    pub view: Arc<Mutex<String>>,
    pub acts: ActsAs,
    /// The `Routing` property's JSON; `None` is an older companion, whose object has no such
    /// property.
    pub routing: Option<Arc<Mutex<String>>>,
}

#[interface(name = "org.bardo.Companion1.Answer")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "the zbus interface macro hands a method its arguments by value, whatever this one holds or does"
)]
impl StandInAnswer {
    fn act(&self, action: String) -> fdo::Result<OwnedObjectPath> {
        self.log.push(format!("act {action}"));
        match self.acts {
            ActsAs::Accepts => OwnedObjectPath::try_from("/org/bardo/Companion1/request/1")
                .map_err(|e| fdo::Error::Failed(e.to_string())),
            ActsAs::Refuses => Err(fdo::Error::AccessDenied("not the shell".to_owned())),
        }
    }

    fn cancel(&self) {
        self.log.push("cancel");
    }

    #[zbus(property)]
    fn session(&self) -> fdo::Result<String> {
        self.session
            .clone()
            .ok_or_else(|| fdo::Error::UnknownProperty("Session".to_owned()))
    }

    #[zbus(property)]
    fn routing(&self) -> fdo::Result<String> {
        self.routing
            .as_ref()
            .map(|routing| routing.lock().unwrap_or_else(|e| panic!("{e}")).clone())
            .ok_or_else(|| fdo::Error::UnknownProperty("Routing".to_owned()))
    }

    #[zbus(property)]
    fn view(&self) -> String {
        self.view.lock().unwrap_or_else(|e| panic!("{e}")).clone()
    }

    #[zbus(signal)]
    async fn updated(emitter: &SignalEmitter<'_>, view: &str) -> zbus::Result<()>;
}

// ---- an app ----

pub struct StandInApp {
    pub answer: Option<SummonAnswer>,
    pub log: Log,
}

#[interface(name = "org.bardo.IntentProvider1")]
impl StandInApp {
    async fn summon(&self, serial: u64, origin: String) -> fdo::Result<String> {
        self.log.push(format!("summon {serial} {origin}"));
        match self.answer {
            Some(answer) => Ok(json(&answer)),
            // An app that never answers.
            None => std::future::pending().await,
        }
    }
}

// ---- The computer-use daemon ----

#[derive(Default)]
pub struct StandInManager {
    pub runs: Vec<OwnedObjectPath>,
    pub sight: Arc<Mutex<String>>,
}

#[interface(name = "org.bardo.ComputerUse1.Manager")]
impl StandInManager {
    fn runs(&self) -> Vec<OwnedObjectPath> {
        self.runs.clone()
    }

    #[zbus(property)]
    fn sight_note(&self) -> String {
        self.sight.lock().unwrap_or_else(|e| panic!("{e}")).clone()
    }

    #[zbus(signal)]
    async fn run_removed(emitter: &SignalEmitter<'_>, run: OwnedObjectPath) -> zbus::Result<()>;
}

#[derive(Clone)]
pub struct StandInRunObject {
    pub id: &'static str,
    pub log: Log,
    pub state: Arc<Mutex<String>>,
}

#[interface(name = "org.bardo.ComputerUse1.Run")]
#[expect(
    clippy::unused_self,
    reason = "zbus calls every method and property getter on the interface object, whatever it holds"
)]
impl StandInRunObject {
    fn pause(&self) {
        self.log.push(format!("pause {}", self.id));
    }

    fn cancel(&self) {
        self.log.push(format!("cancel {}", self.id));
    }

    #[zbus(property)]
    fn state(&self) -> String {
        self.state.lock().unwrap_or_else(|e| panic!("{e}")).clone()
    }

    #[zbus(property)]
    fn id(&self) -> String {
        self.id.replace('_', "-")
    }

    #[zbus(property)]
    fn step(&self) -> u32 {
        2
    }

    #[zbus(property)]
    fn goal(&self) -> String {
        "book a table".to_owned()
    }

    #[zbus(property)]
    fn window(&self) -> String {
        r#"{"app":"org.mozilla.firefox","title":"Tables"}"#.to_owned()
    }

    #[zbus(property)]
    fn mode(&self) -> String {
        "in_place".to_owned()
    }

    #[zbus(property)]
    fn budget(&self) -> String {
        String::new()
    }

    #[zbus(signal, name = "StateChanged")]
    async fn phase_changed(emitter: &SignalEmitter<'_>, state: &str) -> zbus::Result<()>;
}

// ---- memoryd ----

pub struct StandInMemory(pub Vec<TimelineEntry>);

#[interface(name = "org.bardo.Memory1.Control")]
impl StandInMemory {
    fn timeline(&self, space: String, query: String) -> String {
        let _ = (space, query);
        json(&TimelinePage {
            entries: self.0.clone(),
            next: None,
        })
    }
}

/// memoryd with facts waiting for the person: it lists them and logs how each was settled.
pub struct StandInWaiting {
    pub facts: Vec<FactView>,
    pub log: Log,
}

#[interface(name = "org.bardo.Memory1.Control")]
#[expect(
    clippy::unused_self,
    clippy::needless_pass_by_value,
    reason = "the zbus interface macro calls every method and property getter on the interface object and hands a method its arguments by value, whatever this one holds or does"
)]
impl StandInWaiting {
    fn timeline(&self, space: String, query: String) -> String {
        let _ = (space, query);
        json(&TimelinePage {
            entries: Vec::new(),
            next: None,
        })
    }

    /// The desktop and a "work" Space; what waits is in "work", where a helper's job remembers.
    fn spaces(&self) -> String {
        let space = |id: &str| {
            serde_json::json!({
                "id": id, "state": {"kind": "open"}, "vault": "sealed", "created": 0
            })
        };
        serde_json::json!([space("desktop"), space("work")]).to_string()
    }

    fn pending(&self, space: String) -> String {
        match space.as_str() {
            "work" => json(&self.facts),
            _ => "[]".to_owned(),
        }
    }

    fn settle(&self, fact: String, verdict: String) {
        self.log.push(format!("settle {fact} {verdict}"));
    }
}
