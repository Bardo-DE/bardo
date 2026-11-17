//! The registry file store, as a library: `registry.json` in a directory the host chooses, saved
//! atomically with the previous file kept as `registry.json.bak`, behind account-core's
//! [`RegistryStore`](account_core::store::RegistryStore) seam. accountd keeps its registry here, and
//! an app that hosts the accounts service's core itself (account-client-inproc's `Hosted`) keeps its own the same
//! way, with its own directory.
//!
//! The directory is a path the host names; the library reads no environment and no
//! XDG variable. A file that cannot be read is refused and never written over:
//!
//! ```
//! use account_store::FileStore;
//!
//! let store = FileStore::new("/home/ada/.local/state/my-app");
//! assert!(store.path().ends_with("my-app/registry.json"));
//! assert!(store.backup_path().ends_with("my-app/registry.json.bak"));
//! ```
//!
//! This crate is separate from account-core because the file calls run on tokio's blocking pool,
//! and account-core depends on no executor (`scripts/check-boundary.sh` holds it to that).

mod file;

pub use file::{FileStore, RegistryRefusal};
