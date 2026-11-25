//! Settings served on a private bus, called the way actiond calls an app: from the connection
//! that owns `org.bardo.Intents1`, through `IntentProvider1`.
#![allow(
    dead_code,
    reason = "each test binary uses some of the world's helpers, and the others stay unused in it"
)]

use action_core::{AppRefusal, Hit, Invocation, Outcome, Preview, SuggestAsk, UndoFault};
use action_dbus::callers::{ACTIOND, Identity, actiond_only};
use action_dbus::{Details, INTENTS_BUS, IntentProviderProxy};
use action_vocab::{Args, CallId, EntityRef, Origin, ParamName, UndoToken, Value};
use provenance::{Actor, EntityId, Label, Labelled, Source, SpaceId};
use settings_app_callers::{BusPeers, RoleName, introduce_service, private_peers, roles};
use settings_app_files::{ConfigHome, discover_under};
use settings_app_intents::{AccountsRead, AccountsSource, SettingsProvider, serve_with};
use settings_app_model::{Absence, AccountsPane, Catalogue, Notice, OriginRule, Receiver, inbox};
use settings_app_testkit::{Scratch, seed_schemas_settable};
use std::sync::{Arc, Mutex};

use base_testkit::PrivateBus;

pub const TMP: &str = env!("CARGO_TARGET_TMPDIR");
const APP: &str = "org.bardo.Settings";

/// A window that writes down what it is told: the notices that reached its inbox.
pub struct WrittenDown(Mutex<Heard>);

struct Heard {
    receiver: Receiver,
    notices: Vec<Notice>,
}

impl WrittenDown {
    pub fn told(&self) -> Vec<Notice> {
        let mut heard = self.0.lock().expect("lock");
        while let Some(notice) = heard.receiver.try_recv() {
            heard.notices.push(notice);
        }
        heard.notices.clone()
    }
}

/// accountd's answer, whatever it is asked: a list of accounts, or why there is none.
pub struct Listed(pub Result<AccountsPane, Absence>);

impl AccountsSource for Listed {
    fn read(&self) -> AccountsRead<'_> {
        Box::pin(std::future::ready(self.0.clone()))
    }
}

/// The Settings provider serving, and actiond's stand-in calling it.
pub struct World {
    pub window: Arc<WrittenDown>,
    scratch: Scratch,
    proxy: IntentProviderProxy<'static>,
    bus: PrivateBus,
    app: zbus::Connection,
    _actiond: zbus::Connection,
    /// What the activation object judges its callers with: nobody is anyone until a test says so.
    peers: Arc<BusPeers>,
}

pub fn entity(kind: &str, key: &str) -> EntityId {
    EntityId {
        app: provenance::AppName::parse(APP).expect("app"),
        kind: provenance::EntityKind::parse(kind).expect("kind"),
        key: provenance::EntityKey::parse(key).expect("key"),
    }
}

fn space() -> SpaceId {
    SpaceId::parse("work").expect("space")
}

fn from_the_terminal<T>(value: T) -> Labelled<T> {
    Labelled::new(
        value,
        Label::untrusted(Source::Cli, account_core::DataClass::AppOwn, space()),
    )
}

/// A call as `bardo-do` makes it.
pub fn invocation(action: &str, args: &[(&str, Value)]) -> Invocation {
    let args: Args = args
        .iter()
        .map(|(name, value)| {
            (
                ParamName::parse(name).expect("param"),
                from_the_terminal(value.clone()),
            )
        })
        .collect();
    Invocation {
        call: CallId(1),
        action: provenance::ActionName::parse(action).expect("action"),
        target: action_vocab::TargetValue::Nothing,
        args,
        actor: Actor::Cli,
        origin: Origin::Cli,
        space: space(),
    }
}

pub fn key(path: &str) -> Value {
    Value::Entity(entity("settings.key", path))
}

pub fn text(words: &str) -> Value {
    Value::Text(words.to_owned())
}

impl World {
    /// Settings serving the fixture schemas, with `settable` marked agent-settable, over a
    /// scratch config home.
    pub async fn start(test: &str, settable: &[&str]) -> World {
        World::start_listing(test, settable, Listed(Ok(AccountsPane::default()))).await
    }

    /// The same, with accountd answering `accounts`.
    pub async fn start_listing(test: &str, settable: &[&str], accounts: Listed) -> World {
        let scratch = Scratch::under(TMP, test);
        let schemas = seed_schemas_settable(&scratch.join("data"), settable);
        // The scratch folder is the person's, so it is declared a system folder, as a developer
        // install is: an agent mark is honoured only from one.
        let found = discover_under(
            std::slice::from_ref(&schemas),
            &OriginRule::declared(std::slice::from_ref(&schemas)),
        );
        let catalogue = Arc::new(Catalogue::build(&found.programs));
        let (outbox, inbox) = inbox();
        let window = Arc::new(WrittenDown(Mutex::new(Heard {
            receiver: inbox.take().expect("the receiver"),
            notices: Vec::new(),
        })));
        let provider = SettingsProvider::new(
            catalogue,
            ConfigHome(scratch.join("config")),
            outbox,
            Arc::new(accounts),
        )
        .expect("a provider");

        let bus = PrivateBus::start_in(scratch.path());
        let app = bus.connect().await;
        // Every connection of a test is one process, so actiond's stand-in is introduced as the
        // router the provider answers rather than found by its unit.
        let who = Arc::new(Identity::on_fixture(
            app.clone(),
            actiond_only(),
            scratch.join("proc"),
        ));
        let peers = Arc::new(private_peers(&app));
        serve_with(&app, provider, Arc::clone(&who), Arc::clone(&peers))
            .await
            .expect("settings serves");
        let actiond = bus.connect().await;
        who.introduce_app(&actiond, ACTIOND.app, ACTIOND.role);
        actiond
            .request_name(INTENTS_BUS)
            .await
            .expect("actiond's name");
        let proxy = IntentProviderProxy::builder(&actiond)
            .destination(APP)
            .expect("destination")
            .build()
            .await
            .expect("proxy");
        World {
            window,
            scratch,
            proxy,
            bus,
            app,
            _actiond: actiond,
            peers,
        }
    }

    /// A connection to the bus, as the launcher (bardo-shell) has: the shell, which calls the app by name.
    pub async fn launcher(&self) -> zbus::Connection {
        self.caller(roles::SHELL).await
    }

    /// A connection to the bus that the desktop's caller table would know as `role`.
    pub async fn caller(&self, role: RoleName) -> zbus::Connection {
        let connection = self.stranger().await;
        introduce_service(&self.peers, &unique(&connection), &role);
        connection
    }

    /// A connection to the bus that is in no row of the caller table: any program of the person's.
    pub async fn stranger(&self) -> zbus::Connection {
        self.bus.connect().await
    }

    /// The unique name of the connection that serves Settings.
    pub fn owner(&self) -> String {
        unique(&self.app)
    }

    pub fn config_home(&self) -> ConfigHome {
        ConfigHome(self.scratch.join("config"))
    }

    pub fn read(&self, relative: &str) -> Option<String> {
        std::fs::read_to_string(
            self.config_home()
                .path_of_relative(std::path::Path::new(relative)),
        )
        .ok()
    }

    pub fn write(&self, relative: &str, text: &str) {
        let path = self
            .config_home()
            .path_of_relative(std::path::Path::new(relative));
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
        std::fs::write(path, text).expect("write");
    }

    pub async fn perform(
        &self,
        action: &str,
        args: &[(&str, Value)],
    ) -> Result<Outcome, AppRefusal> {
        let json = serde_json::to_string(&invocation(action, args)).expect("json");
        let answer = self
            .proxy
            .perform(&json, &Details::new())
            .await
            .expect("the bus call");
        serde_json::from_str(&answer).expect("an Outcome or an AppRefusal")
    }

    /// An `.open` action on the one thing `id`, as the launcher's Enter on a hit calls it.
    pub async fn open(&self, action: &str, id: EntityId) -> Result<Outcome, AppRefusal> {
        let mut call = invocation(action, &[]);
        call.target = action_vocab::TargetValue::Entities(vec![id]);
        let json = serde_json::to_string(&call).expect("json");
        let answer = self
            .proxy
            .perform(&json, &Details::new())
            .await
            .expect("the bus call");
        serde_json::from_str(&answer).expect("an Outcome or an AppRefusal")
    }

    pub async fn dry_run(
        &self,
        action: &str,
        args: &[(&str, Value)],
    ) -> Result<Preview, AppRefusal> {
        let json = serde_json::to_string(&invocation(action, args)).expect("json");
        let answer = self
            .proxy
            .dry_run(&json, &Details::new())
            .await
            .expect("the bus call");
        serde_json::from_str(&answer).expect("a Preview or an AppRefusal")
    }

    pub async fn undo(&self, token: &UndoToken) -> Result<(), UndoFault> {
        let token = serde_json::to_string(token).expect("json");
        let actor = serde_json::to_string(&Actor::Cli).expect("json");
        let answer = self.proxy.undo(&token, &actor).await.expect("the bus call");
        serde_json::from_str(&answer).expect("() or an UndoFault")
    }

    pub async fn suggest(&self, action: &str, param: &str, typed: &str) -> Vec<EntityRef> {
        let ask = SuggestAsk {
            action: action_vocab::ActionRef {
                app: provenance::AppName::parse(APP).expect("app"),
                name: provenance::ActionName::parse(action).expect("action"),
            },
            param: ParamName::parse(param).expect("param"),
            typed: typed.to_owned(),
        };
        let json = serde_json::to_string(&ask).expect("json");
        let answer = self.proxy.suggest(&json).await.expect("the bus call");
        serde_json::from_str(&answer).expect("entities")
    }

    pub async fn search(&self, words: &str) -> Vec<Hit> {
        let answer = self.proxy.search(words, 1).await.expect("the bus call");
        serde_json::from_str(&answer).expect("hits")
    }

    pub async fn preview(&self, id: &EntityId) -> Preview {
        let json = serde_json::to_string(id).expect("json");
        let answer = self.proxy.preview(&json).await.expect("the bus call");
        serde_json::from_str(&answer).expect("a Preview")
    }
}

/// The unique bus name of `connection`.
pub fn unique(connection: &zbus::Connection) -> String {
    connection
        .unique_name()
        .expect("a connection to a bus has a unique name")
        .to_string()
}

/// Peers on `connection` that know the connection named `owner` as `role`: what a program on
/// that connection judges the owner of a name with.
pub fn peers_knowing(connection: &zbus::Connection, owner: &str, role: &RoleName) -> BusPeers {
    let peers = private_peers(connection);
    introduce_service(&peers, owner, role);
    peers
}
