//! What the held process itself shows, read through the privilege the service keeps: whether
//! anything is tracing it.
//!
//! Its environment is not read (NEW-FA-1). `/proc/<pid>/environ` is mode 0400 and owned by the
//! process's user, and the kernel checks that mode before ptrace access, so root holding only
//! `CAP_SYS_PTRACE` cannot open it for the mouse gestures daemon's or the key remap daemon's dynamic user. Only root can set a
//! system unit's environment, and the unit's root-owned files must unset every loader variable
//! ([`caller_identity_core::ServiceDirectives`]); the capability that would open the file would let a compromised service
//! read every file, and reading the process's memory instead would let it read every process's.

use caller_identity_core::procinfo::parse_status_tracer;
use caller_identity_core::{Pid, ProcFs};

/// The most of a process's status file that is read.
pub const STATUS_CAP: u64 = 16 * 1024;

/// Why the process does not show what a real unit's process shows.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ProcessError {
    /// Another process traces it.
    #[error("another process is tracing it")]
    Traced,
    /// A file about it could not be read, or is not what that file looks like.
    #[error("{0} could not be read")]
    Unreadable(&'static str),
    /// The file about it could not be read.
    #[error("its status could not be read")]
    Read(#[source] std::sync::Arc<std::io::Error>),
    /// The file about it is not text.
    #[error("its status is not text")]
    NotText(#[source] std::string::FromUtf8Error),
}

/// Checks the process `pid` through `fs`: nobody is tracing it.
pub fn check<P: ProcFs>(fs: &P, pid: Pid) -> Result<(), ProcessError> {
    let status = fs
        .read_capped(
            &fs.proc_root().join(pid.to_string()).join("status"),
            STATUS_CAP,
        )
        .map_err(|e| ProcessError::Read(std::sync::Arc::new(e)))?;
    let status = String::from_utf8(status).map_err(ProcessError::NotText)?;
    match parse_status_tracer(&status) {
        Some(0) => Ok(()),
        Some(_) => Err(ProcessError::Traced),
        None => Err(ProcessError::Unreadable("its status")),
    }
}
