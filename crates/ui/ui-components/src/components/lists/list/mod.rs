//! List: the container rows live in, its items and its keys.

pub(crate) mod entry;
pub(crate) mod keys;
#[expect(clippy::module_inception, reason = "the layout names the file for its one concept")]
pub(crate) mod list;
pub mod model;
pub(crate) mod reorder;
