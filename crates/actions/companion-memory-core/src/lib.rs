//! docket's memory seam, pure: the router's audit records as the memory service's events, and the bounded
//! queue the router appends them to. No transport and no IO: `companion-memory` holds the glue that
//! drains the queue into a memory service `Memory`.
//!
//! - [`record_of`], [`kind_tag_of`], [`space_named_by`]: the router's audit records as the memory service's
//!   events.
//! - [`QueuedSink`]: the bounded event sink the host drains.

mod record;
mod sink;

pub use record::{kind_tag_of, record_of, space_named_by};
pub use sink::{QUEUE_LIMIT, QueuedSink};
