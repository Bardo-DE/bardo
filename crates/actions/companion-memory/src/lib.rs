//! docket's memory seam over a transport: what the router asks of memory and what it leaves there,
//! over any `memory_client::Transport` (memoryd over D-Bus on the desktop, the service hosted in
//! the app's own process elsewhere). The pure half, the audit records as the memory service's events and the
//! bounded queue, is `companion-memory-core`.
//!
//! - [`MemoryServiceLink`]: the router's `MemoryLink` over a memory service `Memory`.
//! - [`MemorySessionLog`]: a session's durable log (`agent-session`'s `SessionLog`) over the memory service's
//!   `RecordDurable` and `Entries`.
//! - [`MemoryProvider`]: `org.bardo.Memory`, the actions the planner reaches memory through, as an
//!   `IntentProvider` over a memory service `Memory`.
//! - [`AuditState`]: draining the queue into memory, Space by Space, with retry accounting.

mod audit;
mod cursor_cache;
mod memory;
mod provider;
mod session_log;

pub use audit::{AuditState, Flushed, Link, Report};
pub use memory::MemoryServiceLink;
pub use provider::{MEMORY_APP, MemoryProvider};
pub use session_log::MemorySessionLog;
