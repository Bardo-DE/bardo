//! Atomic file writes: the one blocking place that writes small files whole.
//!
//! [`atomic::AtomicWrite`] replaces a file whole or not at all; [`atomic::AtomicFile`] is one
//! small file kept with its backup. Std only, no runtime, unless the `book` feature is named:
//! [`book::commit`] saves a kept document on the blocking pool.

pub mod atomic;
#[cfg(feature = "book")]
pub mod book;
