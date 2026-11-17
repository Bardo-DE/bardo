//! The time zone dates are shown in.

use std::time::SystemTime;

use jiff::Timestamp;
use jiff::tz::TimeZone;

use crate::words::{NONE, date_words};

/// Which clock a date is read on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Zone {
    /// The person's own time zone, as the system says it (`TZ`, `/etc/localtime`). The one
    /// ambient read of dates.
    #[default]
    Local,
    /// UTC: a test, or a machine with no zone set.
    Utc,
}

impl Zone {
    /// `at` as the Date Modified column writes it, or a dash when the filesystem gave no time.
    #[must_use]
    pub fn words(self, at: Option<SystemTime>) -> String {
        let Some(at) = at else {
            return NONE.to_owned();
        };
        let Ok(at) = Timestamp::try_from(at) else {
            return NONE.to_owned();
        };
        let zone = match self {
            Zone::Local => TimeZone::system(),
            Zone::Utc => TimeZone::UTC,
        };
        date_words(&at.to_zoned(zone))
    }
}
