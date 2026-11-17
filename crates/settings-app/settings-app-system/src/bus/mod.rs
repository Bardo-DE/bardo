//! The bus every service is reached on: connecting, looking for a service's owner, and the
//! look-for-it-then-read-it steps the panes share. Nothing here starts a daemon unless a call
//! says so.

mod activate;
mod attach;
mod io;
mod pinned;
mod system;
mod watch;

pub use activate::start;
pub use attach::{attach, attach_listening, open_once, read_once};
pub use io::{ToAbsence, absence_of, connect, next_change, next_owner, open, owner_changes};
pub(crate) use io::{because, said};
pub use pinned::{Pinned, inferd, pinned, pinned_starting};
pub use system::{is_running, system_connection};
pub use watch::{every, inferd_owners, next, relay, watch_on, watch_service};
