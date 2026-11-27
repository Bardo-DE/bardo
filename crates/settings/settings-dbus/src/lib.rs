//! Live settings modules: `org.bardo.SettingsModule1` (design/22-SETTINGS.md section 9.4).
//!
//! Runtime state that is not a file (accounts, local runtimes, later Wi-Fi and Bluetooth) is
//! served by its daemon on the session bus and rendered by the Settings app with the same widgets
//! as a file schema. This module holds no state of its own:
//!
//! - the daemon implements [`LiveModule`] over its own state and hands it to [`serve`];
//! - the Settings app talks to it through [`LiveClient`] (or the raw [`SettingsModule1Proxy`]).
//!
//! The types both sides name ([`LiveSchema`](settings_proto::LiveSchema), [`Snapshot`](settings_proto::Snapshot), the limits, the timeouts, the interface text) live in `settings-proto`.
//!
//! The wire interface is frozen; `org.bardo.SettingsModule1.xml` beside this file is its
//! introspection XML and a test keeps the skeleton equal to it:
//!
//! | Member | Signature | Meaning |
//! | --- | --- | --- |
//! | `Describe` | `() -> s` | the module's [`LiveSchema`] as JSON |
//! | `Get` | `(s) -> v` | a key's current value |
//! | `Snapshot` | `() -> (s, a{sv})` | the schema (as `Describe`) and every key's value (as `Get`), read once |
//! | `Set` | `(s, v) -> ()` | set a value, or run an action key (any value; the client sends `true`) |
//! | `Changed` | `(s, v)` signal | a key's new value, sent to each peer the module let read, and to no one else |
//!
//! `Changed` is never a broadcast: a signal with no destination reaches everyone who added a match
//! rule, and `permit` cannot refuse a match rule. The skeleton sends it to the peers it let read
//! (a peer that called `Describe`, `Get` or `Snapshot`, or [`LiveClient::changes`]), by unique
//! name; a peer it refused hears nothing. A client takes a signal only from the module it asked
//! (the proxy filters on the module's unique name), and every call a client makes has a
//! deadline ([`Timeouts`](settings_proto::Timeouts), [`LiveError::Timeout`]).
//!
//! Who owns the name: a well-known name goes to whoever asks first, so [`LiveClient`] takes an
//! [`OwnerCheck`] and asks it about the name's owner before the first read or `Set`, and again
//! whenever the owner is another connection. A refused owner is [`LiveError::NotTheService`] and
//! nothing is sent to it, so a typed secret never reaches a squatter; the client then speaks to the
//! owner that passed by its unique name. What a module's replies may carry is capped (`settings_proto::limits`).
//!
//! Values travel as `b` (bool), `x` (integer), `s` (string) or `as` (list of strings); see
//! [`value`].
//!
//! Agent-never-settable (section 9.7): a schema that marks an `accounts.`, `ai.`, `privacy.`, `network.` or `security.` key (or any
//! [`AGENT_NEVER_SETTABLE`](settings_types::schema::AGENT_NEVER_SETTABLE) prefix) agent-settable is
//! refused by [`LiveSchema::check`](settings_proto::LiveSchema::check), which [`serve`] runs before it exports anything and
//! `Describe` runs on every call.
#![expect(
    missing_docs,
    reason = "lints baseline: this crate documents its public items when its domain is refactored (plan phases 3-7)"
)]

mod client;
mod error;
mod limits;
mod link;
mod module;
mod owner;
mod readers;
mod skeleton;
mod snapshot;
pub mod value;

pub use base_time::deadline::Deadline;
pub use client::{Change, Changes, LiveClient, SettingsModule1Proxy};
pub use error::LiveError;
pub use module::{Access, Caller, LiveModule, SenderName, Verdict};
#[cfg(feature = "testing")]
pub use owner::AnyOwner;
pub use owner::{OwnerAnswer, OwnerCheck};
pub use skeleton::{Served, serve};
pub use snapshot::snapshot_from_reads;
