//! The rig a jailed scenario drives ferry with (test tooling: `publish = false`, never installed,
//! never in `dist/`).
//!
//! Two binaries, built on the modules here:
//!
//! - `ferry-rig-servers`: starts the Microsoft Graph fake (`sync-fake-graph`: the drive and the
//!   calendars) behind porter's fake OAuth issuer, plants the tokens `account-rig` plants, writes
//!   `rig.json` (`rigfile`) and serves the scenario's levers (`control`); `servers` is what it
//!   starts.
//! - `ferry-rig-client`: acts as an app over the session bus and talks to syncd (`client`); the
//!   identity the daemons' `test-proc-root` builds read is `account_rig::fixture`'s.
//!
//! `porter-rig-storage`, the fake `org.quire.Storage1` for building Files, lives with syncd's
//! fakes in the `sync-fakes` crate.

pub mod client;
pub mod control;
pub mod rigfile;
pub mod servers;
