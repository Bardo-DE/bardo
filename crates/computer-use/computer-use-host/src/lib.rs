//! computer-use-host: the run executor as a library.
//!
//! The pure machine ([`computer_use_run::Run`]) says what to do; the [`Supervisor`] does it against the
//! seams ([`computer_use_run::Seams`]) and feeds the answers back: a task per run over [`computer_use_run::Exec`],
//! with the person's controls, the desktop's events and the clock racing whatever the run waits
//! on. An app that wants to run a computer-use task in its own process builds the seams (the
//! backends are its own, `computer-use-fake` in tests) and a [`Supervisor`] over them.
//!
//! Nothing here opens a connection or starts a backend: this crate links no wayland, EIS,
//! accessibility or bus crate, and no file watcher. What is here:
//!
//! - the supervisor, its drive loop and its event stream ([`SupervisorEvent`]);
//! - the safety tables the supervisor applies to every start and every call ([`guard`],
//!   [`CuadConfig`]);
//! - the links over docket's and the memory service's transports: the router's gate ([`IntentsGate`]), the
//!   message path and the record sink ([`DaemonLinks`], [`record_of`]);
//! - the person's settings, read from text ([`read`], [`locator`] over the caller's
//!   [`settings_store_fs::ReadText`]) and applied with `Supervisor::adopt` as the caller's watch reports each change.
//!
//! `cuad` is the daemon over this crate: the system seams, the D-Bus service and the file watch.
//!
//! # A supervisor over the fakes
//!
//! ```
//! use computer_use_desktop::BackendKind;
//! use computer_use_fake::FakePdp;
//! use computer_use_fake::scene::world;
//! use computer_use_host::{CuadConfig, Supervisor};
//! use computer_use_run::CuaConfig;
//!
//! let config = CuadConfig {
//!     settings: CuaConfig {
//!         enabled: true,
//!         ..CuaConfig::default()
//!     },
//!     ..CuadConfig::default()
//! };
//! let parts = world(BackendKind::KwinStock, Vec::new(), FakePdp::allow_all());
//! let supervisor = Supervisor::new(parts, config);
//! assert!(supervisor.settings().enabled);
//! assert_eq!(supervisor.started(), 0);
//! assert!(supervisor.runs().is_empty());
//! ```

mod clock;
mod config;
mod drive;
mod events;
mod gate;
mod guard;
mod links;
mod lock;
mod rules;
mod settings;
mod supervisor;

pub use clock::SystemClock;
pub use config::{A11yChoice, BackendChoice, ConfigError, CuadConfig};
pub use events::{RunView, SupervisorEvent};
pub use gate::{IntentsCheck, IntentsGate};
pub use guard::guard;
pub use links::{DaemonLinks, RecordContext, RecordError, record_of};
pub use rules::{RULES_FILE, RulesError, SHIPPED_RULES, parse_rules, shipped_rules};
pub use settings::{
    AppAllow, AppRule, Apps, CuaSettings, SCHEMA, SETTINGS_FILE, is_browser, locator, read,
};
pub use supervisor::{Supervisor, mint_run_id};
