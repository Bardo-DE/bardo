//! The rig a jailed scenario drives porter with (test tooling: `publish = false`, never
//! installed, never in `dist/`; check-boundary lists it so nothing shipped depends on it).
//!
//! Three binaries, built on the modules here:
//!
//! - `account-rig-servers`: starts `account-fake-servers`' fakes on loopback, plants known secrets
//!   (`planted`), writes `rig.json` (`rigfile`) and serves the scenario's levers (`control`);
//!   `servers` is which flag starts which fake.
//! - `account-rig-client`: acts as an app over the session bus (`client`); `fixture` makes the
//!   identity the daemons' `test-proc-root` builds read.
//! - `account-rig-secrets`: a fake `org.freedesktop.secrets` (the `account-fake-secret-service` crate), in
//!   memory, plain algorithm, so the real accountd's `Oo7Secrets` works in a jail with no keyring.
//!
//! The rig for the sync daemon (the Graph fake behind porter's issuer, the client for
//! `org.bardo.Sync1`) and `sync-rig-storage`, the fake `org.bardo.Storage1` for building Files,
//! are the sync service's: `sync-rig` and `sync-fakes`.

pub mod client;
pub mod control;
pub mod fixture;
pub mod planted;
pub mod rigfile;
pub mod servers;
