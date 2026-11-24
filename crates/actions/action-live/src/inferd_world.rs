//! The smallest world a corpus run needs: a private bus, accountd and inferd.
//!
//! The smallest world a corpus run needs: a private bus, the packaged accountd (inferd asks it for
//! the AI keys) and inferd, and this
//! process placed in the fake proc root as `bardo-actiond.service` so inferd serves it as
//! `org.bardo.Intents` (the policy writer and the reviewers). Everything is scratch: HOME, every
//! XDG directory, the bus. The model is a cassette or the owner's own `inferd.toml`.
//!
//! The corpus cases play the router in this process (a hijacked planner mints handles in router
//! state, which the bus has no way to do), so intentd, readerd, companiond and memoryd are not
//! started here; `dev/live-smoke.sh` runs all of them.

use action_accept::world::{
    Binaries, Cgroup, DaemonEnv, GIVE_UP, ModelSource, Options, Scratch, env_of, inferd_toml,
    place, spawn, until_owned, write,
};
use action_dbus::BusConnection;
use base_testkit::{PrivateBus, Reaped};
use inference_core::ChargeTotal;
use model_provider::Money;
use rust_decimal::Decimal;
use std::path::{Path, PathBuf};

/// The running world.
pub struct InferdWorld {
    // Declared first: inferd is killed before accountd and the bus it talks on go.
    inferd: Reaped,
    _accountd: Reaped,
    /// The scratch root.
    pub dir: Scratch,
    bus: PrivateBus,
}

impl std::fmt::Debug for InferdWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InferdWorld")
    }
}

impl InferdWorld {
    /// Starts the bus, accountd and inferd (replaying or live, as `model` says) and waits for
    /// their names.
    ///
    /// # Panics
    ///
    /// Panics when the scratch directory cannot be made or a daemon does not start: this is a test
    /// harness, and a world that is not up has nothing to run.
    pub async fn start(binaries: &Binaries, model: &ModelSource, options: &Options) -> Self {
        let dir = Scratch::made(options.keep_in.as_deref()).expect("scratch");
        let root = dir.path();
        for sub in ["logs", "data", "config", "cache", "run"] {
            std::fs::create_dir_all(root.join(sub)).expect("dirs");
        }
        std::fs::set_permissions(
            root.join("run"),
            std::os::unix::fs::PermissionsExt::from_mode(0o700),
        )
        .expect("run dir mode");
        action_accept::runlink::link_run(root).expect("short runtime name");
        action_accept::world::copy_catalog(options.catalog.as_deref(), root);
        let bus = PrivateBus::start_in(root);
        let client = bus.connect().await;
        place(root, std::process::id(), Cgroup::Unit("bardo-actiond"));
        action_accept::world::write_system_callers(root).expect("the system callers files");
        if let ModelSource::Scripted(cassette) = model {
            write(&root.join("cassette.jsonl"), cassette);
        }
        write(
            &root.join("config/bardo/inference/inferd.toml"),
            &inferd_toml(root, model, None),
        );
        let keys = options.keys_home.as_ref().map(|home| {
            home.seed(root).expect("the key home's records are copied");
            home.keys_setting()
        });
        let daemon_env = DaemonEnv {
            keys: keys.as_deref(),
            engine_cache: binaries.engine_cache.as_deref(),
        };
        let start = async |name: &'static str, binary: &Path, owns: &str| {
            let (name, daemon) = spawn(root, bus.address(), name, binary, None, daemon_env)
                .unwrap_or_else(|e| panic!("{name} starts: {e}"));
            place(root, daemon.pid(), Cgroup::Unit(name));
            until_owned(&client, owns).await;
            daemon
        };
        let accountd = start("bardo-accountd", &binaries.accountd, "org.bardo.Accounts1").await;
        let inferd = start("bardo-inferd", &binaries.inferd, "org.bardo.Inference1").await;
        Self {
            inferd,
            _accountd: accountd,
            dir,
            bus,
        }
    }

    /// A fresh connection to the private bus: the corpus run's link to inferd.
    pub async fn connect(&self) -> BusConnection {
        self.bus.connect().await
    }

    /// The scratch root.
    #[must_use]
    pub fn root(&self) -> &Path {
        self.dir.path()
    }

    /// inferd's standard error, for a failing run to print.
    #[must_use]
    pub fn log(&self) -> String {
        std::fs::read_to_string(self.root().join("logs/inferd.log")).unwrap_or_default()
    }

    /// The charges the providers have reported so far, from inferd's own ledger (money and
    /// token counts, never content): the largest daily total of any scope, which is the whole
    /// reported spend because every request is counted once on its account and once on its app.
    /// Zero US dollars when nothing is reported or the ledger cannot be read.
    #[must_use]
    pub fn reported_charge(&self) -> Money {
        let none = Money::usd(Decimal::ZERO);
        let file: PathBuf = self.root().join(".local/state/bardo/inference/spend.json");
        let Ok(text) = std::fs::read_to_string(file) else {
            return none;
        };
        let Ok(doc) = serde_json::from_str::<serde_json::Value>(&text) else {
            return none;
        };
        doc["rows"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|row| row["period"] == "daily")
            .filter_map(|row| {
                serde_json::from_value::<ChargeTotal>(row["spent"]["charges"].clone()).ok()
            })
            .map(|charges| charges.reported)
            .max_by_key(|reported| reported.amount)
            .unwrap_or(none)
    }

    /// How long a wait may take before a run fails instead of hanging.
    #[must_use]
    pub fn give_up() -> std::time::Duration {
        GIVE_UP
    }

    /// The inferd process id (for the owner's `ps`).
    #[must_use]
    pub fn inferd_pid(&self) -> u32 {
        self.inferd.pid()
    }

    /// `env` for a command that should talk to this world's bus as the owner's own shell
    /// would: scratch HOME and XDG directories and the private bus address.
    #[must_use]
    pub fn env(&self) -> Vec<(String, String)> {
        let mut env: Vec<(String, String)> = env_of(self.root())
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect();
        env.push((
            "DBUS_SESSION_BUS_ADDRESS".to_owned(),
            self.bus.address().to_owned(),
        ));
        env
    }
}
