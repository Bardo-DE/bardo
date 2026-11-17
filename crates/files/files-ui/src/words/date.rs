//! Dates as the Date Modified column shows them.

use jiff::Zoned;

/// `at` as `2026-10-21 14:03`, in the zone `at` carries.
///
/// The UI kit has no date formatter yet (the relative "Today, 14:03" Finder shows is its to add), so
/// the column writes the full date.
#[must_use]
pub fn date_words(at: &Zoned) -> String {
    at.strftime("%Y-%m-%d %H:%M").to_string()
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use jiff::civil::date;
    use jiff::tz::TimeZone;

    use super::*;
    use crate::Zone;
    use crate::words::NONE;

    #[test]
    fn dates_are_written_in_full_and_a_missing_one_is_a_dash() {
        let at = date(2026, 10, 21)
            .at(14, 3, 59, 0)
            .to_zoned(TimeZone::UTC)
            .unwrap();
        assert_eq!(date_words(&at), "2026-10-21 14:03");
        let early = date(2026, 1, 2)
            .at(3, 4, 0, 0)
            .to_zoned(TimeZone::UTC)
            .unwrap();
        assert_eq!(date_words(&early), "2026-01-02 03:04");

        let system = SystemTime::from(at.timestamp());
        assert_eq!(Zone::Utc.words(Some(system)), "2026-10-21 14:03");
        assert_eq!(Zone::Utc.words(None), NONE);
    }
}
