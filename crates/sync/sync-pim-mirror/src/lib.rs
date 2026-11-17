//! `PimMirror`: calendars and address books mirrored from a server into a local vdir,
//! read-only (design/31 §6.2 "`PimMirror`", PLAN §2 rows Calendar and Contacts, D9).
//!
//! The vdir is `$XDG_DATA_HOME/bardo/sync/vdir/<account>/<collection>/`, `<account>` being
//! `base_ids::object_segment(account)`: the segment `AccountRemoved` wipes. A collection holds
//! one file per item, `<uid>.ics` or `<uid>.vcf`, and the files `displayname` and `color`
//! (vdirsyncer's storage spec, which the shell's `calendar.sources = auto` and khal read).
//!
//! - `mirror`: [`PimMirror`], the `sync_engine::dataset::Dataset` of one collection, pull-only:
//!   the engine never uploads or removes anything for it, so a file edited or deleted locally
//!   never reaches the server. Its files are `sync_vdir`'s.
//! - `plan`: collection names as directories and dataset slugs.
//! - `grants`: which granted accounts there are to mirror (a seam over account-client).
//! - `mirrors`: one account's collections kept running (replicas over the relay, one engine
//!   each) and the supervisor over every granted account.
//!
//! Where the items come from (`PimKind`, discovery, CalDAV/CardDAV, Graph and Google feeds) is
//! `sync_pim_feed`.

mod grants;
mod mirror;
mod mirrors;
mod plan;
mod supervisor;
#[cfg(test)]
mod testing;

pub use grants::{AccountdUnavailableError, ClientGrants, PimGrants, need_of};
pub use mirror::PimMirror;
pub use mirrors::{AccountMirrors, Wiring};
pub use plan::{Planned, plan};
pub use supervisor::{PimConfig, PimSupervisor};
