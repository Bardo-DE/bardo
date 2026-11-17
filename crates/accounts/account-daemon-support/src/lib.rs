//! What the desktop's daemons share and need no bus, runtime or environment for.
//!
//! One small piece, used by several daemons that used to carry a copy: [`registry_words`], the
//! words a registry file's errors are told in. (The watch on a settings file is
//! `settings_store_fs::watch_file`.)
//!
//! The library reads no environment variable. The daemon passes in how it looks one up, so a
//! test (or a program that runs a daemon inside itself) passes a table instead. Which `/proc` and
//! which caller tables a daemon reads are in `account-callers`.

mod registry;

pub use registry::registry_words;
