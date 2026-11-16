//! The grammar of an id and the [`slug_id!`] macro that declares one, the text-newtype items every
//! validated string shares ([`string_newtype!`]), enum variants as their serde word ([`word`]), and
//! units: every number that means something has a type. Each value is parsed once where it enters,
//! so the rest of the code holds a value that is already safe in a file name and, after
//! [`object_segment`], in a D-Bus object path:
//!
//! ```
//! use base_ids::{is_id, object_segment};
//!
//! assert!(is_id("work.nextcloud"));
//! // On the bus the id is one path segment: `.` and `-` become `_`.
//! assert_eq!(object_segment("work.nextcloud"), "work_nextcloud");
//! // Capital letters are not in the grammar.
//! assert!(!is_id("Work"));
//! ```
//!
//! No runtime, no file system, no I/O.

mod error;
mod id;
mod units;
pub mod word;

pub use error::IdError;
pub use id::{is_id, object_segment};
pub use units::{Bytes, Count, Dims, Permille, Px, Tokens, TurnCount};
