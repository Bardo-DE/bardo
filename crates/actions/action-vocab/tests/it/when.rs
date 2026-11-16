//! RFC 3339 dates and instants, as the terminal and the reader both read them.

use action_vocab::CivilDate;
use action_vocab::when::{self, date, instant};
use provenance::UnixSeconds;

#[test]
fn dates_table() {
    let rows: [(&str, Option<CivilDate>); 7] = [
        (
            "2026-10-03",
            Some(CivilDate {
                year: 2026,
                month: 10,
                day: 3,
            }),
        ),
        (
            "2026-10-03T09:30:00Z",
            Some(CivilDate {
                year: 2026,
                month: 10,
                day: 3,
            }),
        ),
        (
            "2024-02-29",
            Some(CivilDate {
                year: 2024,
                month: 2,
                day: 29,
            }),
        ),
        ("2026-02-29", None),
        ("tomorrow", None),
        ("2026-13-01", None),
        ("2026-10-03Tnine", None),
    ];
    for (text, want) in rows {
        assert_eq!(date(text).ok(), want, "{text}");
    }
}

#[test]
fn instants_table() {
    // 2026-10-03 is 20 729 days after the epoch.
    let rows: [(&str, Option<UnixSeconds>); 6] = [
        ("1970-01-01", Some(UnixSeconds(0))),
        ("1970-01-02T00:00:00Z", Some(UnixSeconds(86_400))),
        (
            "2026-10-03T09:30:00+02:00",
            Some(UnixSeconds(1_791_012_600)),
        ),
        ("2026-10-03T09:30:00.5Z", Some(UnixSeconds(1_791_019_800))),
        ("2026-10-03T25:00:00Z", None),
        ("soon", None),
    ];
    for (text, want) in rows {
        assert_eq!(instant(text).ok(), want, "{text}");
    }
}

#[test]
fn leap_seconds_lowercase_t_and_spaces_read_as_before() {
    let rows: [(&str, Option<UnixSeconds>); 4] = [
        ("1970-01-01T00:00:60Z", Some(UnixSeconds(60))),
        ("1970-01-02t00:00:00z", Some(UnixSeconds(86_400))),
        ("1970-01-02 00:00:00Z", Some(UnixSeconds(86_400))),
        ("+970-01-02T00:00:00Z", None),
    ];
    for (text, want) in rows {
        assert_eq!(instant(text).ok(), want, "{text}");
    }
}

#[test]
fn utc_instants_are_rfc3339() {
    let cases = [
        (0, "1970-01-01T00:00:00Z"),
        (951_782_400, "2000-02-29T00:00:00Z"),
        (1_760_000_000, "2025-10-09T08:53:20Z"),
        (-1, "1969-12-31T23:59:59Z"),
    ];
    for (at, instant) in cases {
        assert_eq!(when::utc_instant(UnixSeconds(at)).as_deref(), Some(instant));
    }
}

#[test]
fn an_offset_without_its_colon_is_refused() {
    when::instant("2026-10-03T09:30:00+0200").unwrap_err();
    when::date("2026-10-03T09:30:00+0200").unwrap_err();
}
