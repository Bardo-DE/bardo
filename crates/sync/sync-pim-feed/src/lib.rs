//! Where the items of a calendar, an address book or a task list come from (io: the `Http` seam).
//!
//! - [`PimKind`]: which kind of collection (calendar, contacts, tasks), its file extension,
//!   envelope, protocol family and data class.
//! - `discover`: the collections of an account, from the principal and home sets
//!   (account-dav-proto's walk).
//! - `source`: where the items come from (CalDAV/CardDAV, Graph calendars, Google's Calendar,
//!   People and Tasks APIs), by the capability's transport; see its module docs for the seam.

pub mod discover;
pub mod source;

pub use discover::{DiscoverError, Found, discover};

use account_core::{DataClass, Family};

/// Which kind of collection a mirror holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PimKind {
    /// Calendars (CalDAV): `.ics` files.
    Calendar,
    /// Address books (CardDAV): `.vcf` files.
    Contacts,
    /// Task lists: `.ics` files holding a `VTODO`, in their own directories.
    Tasks,
}

impl PimKind {
    /// Every kind.
    pub const ALL: [PimKind; 3] = [PimKind::Calendar, PimKind::Contacts, PimKind::Tasks];

    /// The file extension of an item, with its dot.
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            PimKind::Calendar | PimKind::Tasks => ".ics",
            PimKind::Contacts => ".vcf",
        }
    }

    /// The `BEGIN` and `END` lines a whole item opens and closes with.
    #[must_use]
    pub fn envelope(self) -> (&'static str, &'static str) {
        match self {
            PimKind::Calendar | PimKind::Tasks => ("BEGIN:VCALENDAR", "END:VCALENDAR"),
            PimKind::Contacts => ("BEGIN:VCARD", "END:VCARD"),
        }
    }

    /// The protocol family that reaches it over DAV (a task list is a `CalDAV` collection of
    /// `VTODO`s).
    #[must_use]
    pub fn family(self) -> Family {
        match self {
            PimKind::Calendar | PimKind::Tasks => Family::CalDav,
            PimKind::Contacts => Family::CardDav,
        }
    }

    /// The data class a grant for it covers.
    #[must_use]
    pub fn class(self) -> DataClass {
        match self {
            PimKind::Calendar => DataClass::Calendar,
            PimKind::Contacts => DataClass::Contacts,
            PimKind::Tasks => DataClass::Tasks,
        }
    }

    /// What its directories end in: nothing for a calendar, `-contacts` for an address book,
    /// `-tasks` for a task list (so the three never share a directory).
    #[must_use]
    pub fn dir_suffix(self) -> &'static str {
        match self {
            PimKind::Calendar => "",
            PimKind::Contacts => "-contacts",
            PimKind::Tasks => "-tasks",
        }
    }

    /// The prefix of its datasets' names (`pim_cal_personal`, `pim_card_contacts_contacts`).
    #[must_use]
    pub fn slug(self) -> &'static str {
        match self {
            PimKind::Calendar => "pim_cal",
            PimKind::Contacts => "pim_card",
            PimKind::Tasks => "pim_task",
        }
    }
}

/// Whether `bytes` is a whole iCalendar or vCard (what a reader must always find): it opens with
/// the kind's `BEGIN` line and closes with its `END` line.
pub fn is_complete(kind: PimKind, bytes: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    let (begin, end) = kind.envelope();
    let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
    let first = lines.next();
    let last = lines.next_back();
    first.is_some_and(|l| l.eq_ignore_ascii_case(begin))
        && last.is_some_and(|l| l.eq_ignore_ascii_case(end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_complete_item_opens_and_closes_with_its_envelope() {
        const CASES: &[(PimKind, &str, bool)] = &[
            (
                PimKind::Calendar,
                "BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n",
                true,
            ),
            (PimKind::Calendar, "begin:vcalendar\nEND:VCALENDAR", true),
            (PimKind::Calendar, "BEGIN:VCALENDAR\r\nUID:x\r\n", false),
            (PimKind::Calendar, "", false),
            (PimKind::Contacts, "BEGIN:VCARD\nEND:VCARD\n", true),
            (PimKind::Contacts, "BEGIN:VCALENDAR\nEND:VCALENDAR\n", false),
        ];
        for (kind, text, ok) in CASES {
            assert_eq!(is_complete(*kind, text.as_bytes()), *ok, "{text:?}");
        }
    }
}
