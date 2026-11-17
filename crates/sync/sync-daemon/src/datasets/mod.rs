//! The datasets of syncd that are still the daemon's: the others are crates of their own
//! (`sync-pim-mirror`: calendars and address books mirrored into a local vdir; `sync-photos`:
//! originals by content and an HLC-stamped metadata manifest).
//!
//! - `storage`: the app folder of a Graph account mirrored two ways, and the supervisor that
//!   also runs Photos for it.
//! - `drive`: a Storage account's whole drive, listed only (no download), for Files.

pub mod drive;
pub mod storage;
