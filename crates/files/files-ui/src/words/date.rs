//! Dates as the Date Modified column shows them.

use std::fmt::Display;

use chrono::{DateTime, TimeZone};

/// `at` as `2026-10-21 14:03`, in the zone `at` carries.
///
/// quire has no date formatter yet (the relative "Today, 14:03" Finder shows is its to add), so
/// the column writes the full date.
pub fn date_words<Tz>(at: &DateTime<Tz>) -> String
where
    Tz: TimeZone,
    Tz::Offset: Display,
{
    at.format("%Y-%m-%d %H:%M").to_string()
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use chrono::Utc;

    use super::*;
    use crate::Zone;
    use crate::words::NONE;

    #[test]
    fn dates_are_written_in_full_and_a_missing_one_is_a_dash() {
        let at = Utc.with_ymd_and_hms(2026, 10, 21, 14, 3, 59).unwrap();
        assert_eq!(date_words(&at), "2026-10-21 14:03");
        let early = Utc.with_ymd_and_hms(2026, 1, 2, 3, 4, 0).unwrap();
        assert_eq!(date_words(&early), "2026-01-02 03:04");

        let system: SystemTime = at.into();
        assert_eq!(Zone::Utc.words(Some(system)), "2026-10-21 14:03");
        assert_eq!(Zone::Utc.words(None), NONE);
    }
}
