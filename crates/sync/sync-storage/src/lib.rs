//! The storage datasets of syncd (io: tokio, the file system, FUSE, the `Http` seam).
//!
//! - `storage`: the app folder of a Graph or Google account mirrored two ways, and the
//!   supervisor that also runs Photos for it and the whole drives for Files.
//! - `drive`: a Storage account's whole drive, listed only (no download), for Files, and the
//!   files on demand over it (`drive_fs::Drive`, shown at `~/Cloud/<Drive>`).
//! - `knobs`: the `[storage]` table of syncd's own settings file: the cache limit and which
//!   drives are paused or kept downloaded whole.
//! - `words`: what Files shows for the typed reasons of drive-fs, drive-fuse and
//!   bardo-drive-mount.

pub mod drive;
pub mod knobs;
pub mod storage;
pub mod words;

#[cfg(test)]
mod testing;
