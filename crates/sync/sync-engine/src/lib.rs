//! The sync engine as sans-IO logic (design/31 §6): no runtime, no database, no bus, no files.
//!
//! - `dataset`: the local side of a sync, a trait (`Dataset`) the daemon's datasets implement.
//! - `engine`: one dataset against one replica through one journal, a cycle at a time.
//! - `journal`: the journal contract (`Op`, `JournalStore`); `sync-journal-sqlite` is the store.
//! - `scheduler`: the pure rules of when a dataset looks next (poll, backoff, push window,
//!   metered networks), over inputs the driver reads.
//!
//! `MemoryDataset` and `Settings::quick` are behind the `testing` feature.

pub mod dataset;
pub mod engine;
pub mod journal;
pub mod scheduler;
