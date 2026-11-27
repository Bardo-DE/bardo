//! Which program is calling: the pure half of the caller-identity library.
//!
//! This crate holds the vocabulary (ids, the facts read about a process, the caller table) and
//! the judgement over them. It does no I/O and reads no clock, environment or fixed path: a
//! [`Prober`] reads the facts of a [`Held`] process through a [`ProcFs`] (the I/O crate
//! `caller-identity-linux` supplies the real `/proc` and the pidfd) and fills in [`Facts`];
//! [`judge`] turns facts and a [`Table`] into a [`Standing`]; a method says what it accepts with
//! [`Requirement`]s and [`Standing::holds`]. The names of the roles themselves belong to the
//! services that own them; this crate has only the [`RoleName`] type.

mod cgroup;
mod error;
mod facts;
mod ids;
mod judge;
mod probe;
mod procfs;
mod requirement;
mod standing;
mod table;
mod unitfile;
mod walk;

pub mod procinfo;

pub use cgroup::{CgroupPath, escape_unit, flatpak_scope_in, scope_app, unescape_unit};
pub use error::{IdError, NotHeldError, ProbeError, RowError, TableError};
pub use facts::{
    DropIns, Engine, Exe, Facts, FileTrust, Fragment, LOADER_VARIABLES, Loader, Pin, PinNeed,
    SandboxFacts, Transient, UnitFacts, Writer,
};
pub use ids::{AppKey, ExePath, ExePattern, Pid, RoleName, Uid, UnitName};
pub use judge::judge;
pub use probe::{Held, PROBE_MAX_READS, PROBE_READ_CAP, Prober};
pub use procfs::{Follow, ProcFs, trust_of_mode};
pub use requirement::Requirement;
pub use standing::{Proof, Standing, WhyNot};
pub use table::{NAME_MAX, Row, TABLE_BYTES_MAX, TABLE_ROWS_MAX, Table};
pub use unitfile::ServiceDirectives;
pub use walk::{WALK_MAX, dirs_trust, path_trust};
