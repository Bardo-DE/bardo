//! syncd: the program of design/31 §6, as a library so each part is tested without the binary.
//! What it runs is in the crates it is built from: the engine and the scheduler (`sync-engine`),
//! the SQLite journal (`sync-journal-sqlite`), the loop and the hub (`sync-driver`), the datasets
//! (`sync-pim-mirror`, `sync-photos`, and the storage folder and drives of `sync-storage`), and the
//! replicas over accountd's relays (`sync-relay`).
//!
//! - `keep`: what syncd keeps of the accounts accountd has: the sync switches (each is a grant
//!   accountd asks the person for the first time), "Show in Files" and removal, all through
//!   account-client. `settings`: the Settings module (`org.bardo.SettingsModule1`) over them.
//! - `daemon`: the daemon as one call, [`Config`] (typed; [`Config::from_env`] is the one place the
//!   environment is read), [`Daemon::build`] and [`Daemon::run`]. The binary only parses its
//!   arguments and calls these.
// peephole's caller types (account-dbus ProcCallers) take the daemon's futures past the default
// depth of 128.
#![recursion_limit = "256"]
pub mod daemon;
pub mod keep;
pub mod paths;
pub mod service;
pub mod settings;

pub use daemon::{Config, Daemon, StartError};
