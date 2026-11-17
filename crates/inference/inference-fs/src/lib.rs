//! The file io of the inference libraries, in one place.
//!
//! `inference-router` and `inference-bridge` are pure: every function in them works over values it
//! is given. What they are given comes from files and descriptors, and reading those is here:
//!
//! - [`catalog`]: the catalogue directories as a [`inference_router::catalog::Catalog`].
//! - [`key`]: an attached engine's bearer token from its private file.
//! - [`socket`]: the check of an engine's socket path before anything is spawned on it.
//! - [`attachments`]: the bytes of the descriptors that came with one frame.
//!
//! Nothing here reads the environment or the clock, and nothing needs a runtime.

pub mod attachments;
pub mod catalog;
pub mod key;
pub mod socket;

pub use attachments::read_frames;
pub use catalog::read_catalog;
pub use key::read_key;
pub use socket::clear_socket;
