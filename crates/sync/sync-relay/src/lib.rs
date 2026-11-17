//! The replicas as syncd runs them (io: accountd's relays): provider glue beside the replica
//! crates. Every connection is a stream accountd opens (`Tokens.OpenAuthenticated`, or
//! `OpenLinked` for Graph's pre-authenticated links), so syncd never holds an account's
//! credential.
//!
//! - `webdav`: the `WebDAV` replica of a folder under the account's endpoint, and `webdav::BuildError`.
//! - `gdrive`: the Google Drive replica of a folder in the account's app data folder.
//! - `graph`: the Graph (`OneDrive`) replica of a folder in the account's app folder.
//! - `pim`: the connections and the replica of one `CalDAV` or `CardDAV` collection.

pub mod gdrive;
pub mod graph;
pub mod pim;
pub mod webdav;
