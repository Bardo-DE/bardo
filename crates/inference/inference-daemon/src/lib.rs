//! inferd's parts, as a library so the daemon's modules are testable.
//!
//! inferd's parts, as a library so the daemon's modules are tested without a bus: the fd session
//! server (`serve`) and the real seams it is hosted over: the router (`engines`), the engine host
//! (`engines`, `supervise`, `hosts`), the model turns (`runner`, which pins a session's turns to a
//! model and adds the hosted and speech ones) and the audit file's wiring; `service` is the
//! `Inference1` object on the bus and `peers` who is calling. Stubs behind frozen interfaces
//! (`speech`) are listed in `FINDINGS.md`.
//!
//! This crate is the program, not the logic. The logic lives in libraries:
//!
//! - `inference-router`: routing and the session machine (the router, the plan, the catalogue, the
//!   local model book, the attached engines as values, the seams);
//! - `inference-turns`: running turns (a local model's chat, embedding and computer-use turns, the
//!   structured-reply check, the audit trail, the stages of a pipeline);
//! - `inference-bridge`: the mapping between the broker's wire types and the providers' turns;
//! - `inference-settings`: the `[ai]` rows, what they resolve to, and the spend book;
//! - `inference-agent-proto`: the Anthropic and `OpenAI` protocols of the agent endpoint;
//! - `inference-fs`: the files and descriptors the pure ones are given.
//!
//! inferd keeps the bus objects, the process hosts, the settings wiring, the listeners and the
//! start-up. Its modules are private; what a test or a program that hosts the daemon needs is
//! re-exported here.
//!
//! The daemon as one call is `daemon`: [`Config`] (typed; [`Config::from_env`] is the one place
//! the environment is read), [`Daemon::build`] and [`Daemon::run`]. The binary only parses its
//! arguments, listens for signals and calls these.

mod admit;
mod agent;
mod attached;
mod broker;
mod cloud;
mod config;
mod daemon;
mod engines;
mod failure;
mod hosts;
mod installed;
mod peers;
mod pipeline;
mod probe;
mod probed;
mod replay;
mod report;
mod runner;
mod seats;
mod serve;
mod service;
mod settings;
mod shutdown;
mod speech;
mod spend_rows;
mod supervise;
mod tailnet;
mod toolproof;
mod watch;

pub use agent::Agents;
pub use attached::{
    AddedFile, Attached, AttachedBook, Computers, KeyFileError, NotReady, Reach, Site, Target,
    models as attached_models, probe as probe_attached,
};
pub use broker::{AccountList, AccountdBroker, follow as follow_accounts};
pub use cloud::Cloud;
pub use cloud::accountd::{AccountVerdict, Accountd, AccountdFault, Boxed};
pub use cloud::own::{OWN_PROVIDER, OwnServer};
pub use cloud::wire::{Door, Doors};
pub use config::{Dirs, InferdConfig, ProgramSearch};
pub use daemon::{Config, Daemon, StartError, StopError};
pub use engines::{CLOUD_ACCOUNT, Engines};
pub use hosts::{HealthProbe, ProcessHost};
pub use installed::Installed;
pub use peers::{CallerTable, TablePeers};
pub use probe::{Found, ProbeConfig, Probed, Runtime};
pub use probed::Standing;
pub use replay::Replays;
pub use report::{NoReports, ReportError, Reports};
pub use runner::{Pin, Pinned, Turns};
pub use serve::{guarded, serve_session};
pub use service::{Inference, serve_on, serve_with_settings};
pub use settings::{ConfigFile, InferdSettings, LocalEngines, Reload};
pub use shutdown::{Ended, Signals, on_signal};
pub use speech::Ears;
pub use supervise::{Failure as EngineFailure, Ports, Supervised};
pub use tailnet::words::refusal as tailnet_refusal;
pub use tailnet::{BusMachines, Parts, Tailnet};
pub use watch::{Probing, Watch};

/// The catalog entries the hosted tests use, shared with the unit tests.
#[cfg(test)]
use inference_router::testkit::entries;

/// The lab engine (an OpenAI-compatible server on a socket) the attached-engine tests share.
#[cfg(test)]
// `say` and `chats` (tests/it/hosting/lab.rs) are used by no test yet; the allow stays for them.
#[allow(dead_code, reason = "`say` and `chats` are used by no test yet")]
mod lab;

/// The fake speech host the speech tests share with the hosted ones.
#[cfg(test)]
mod speech_host;

#[cfg(test)]
mod testkit;
