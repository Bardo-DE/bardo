//! The local zone, read from the system through `jiff`. This is the one place that
//! touches the system for it; everything below is handed a [`Zone`].

use companion_model::routine::Zone;
use jiff::Timestamp;
use jiff::tz::TimeZone;
use provenance::UnixSeconds;

/// The system's zone.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemZone;

impl SystemZone {
    /// The zone at `at`, daylight saving included: UTC when `at` is beyond the calendar.
    #[must_use]
    pub fn at(&self, at: UnixSeconds) -> Zone {
        Timestamp::from_second(at.0).map_or(Zone::UTC, |utc| {
            Zone(TimeZone::system().to_offset(utc).seconds())
        })
    }
}
