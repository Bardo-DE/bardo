//! Who is logged in on the seat: logind's seat file, read each time. The `ACTIVE_UID` line is parsed by
//! `input_broker_core::identity::parse_active_uid`, the same fact the key remap daemon reads.

use base_fs::{ReadCap, read_text_capped};
use input_broker_core::identity::{ActiveUid, SeatSource, parse_active_uid};
use std::path::PathBuf;

/// Where logind records the seat's state.
pub const SEAT_FILE: &str = "/run/systemd/seats/seat0";
/// The most of the seat file that is read: it holds a few short lines.
const MAX_SEAT: ReadCap = ReadCap::bytes(4096);

/// Reads logind's seat file each time it is asked: a login changes it.
#[derive(Debug, Clone)]
pub struct SeatFile {
    path: PathBuf,
}

impl SeatFile {
    /// The seat file of this machine.
    #[must_use]
    pub fn system() -> Self {
        Self::at(SEAT_FILE)
    }

    /// A seat file somewhere else, for a test.
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl SeatSource for SeatFile {
    fn active_uid(&self) -> ActiveUid {
        match read_text_capped(&self.path, MAX_SEAT) {
            Ok(text) => parse_active_uid(&text),
            Err(_) => ActiveUid(None),
        }
    }
}
