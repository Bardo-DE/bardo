//! What a client reads: the notifications server, the lock, the polkit agent, the clipboard
//! history, idle and keycap.
//!
//! Each answer is the `shell-model` state, serialised; the two that hold more than a client should
//! see (a clipboard entry's text, a polkit prompt's cookie) are reduced to counts and names here.

use serde::{Deserialize, Serialize};
use shell_model::clipboard::{ClipStatus, Clipboard};
use shell_model::idle::Idle;
use shell_model::intents::terminal::{Breaker, ControlReach, grant_name};
use shell_model::intents::{Intents, IntentsLink};
use shell_model::keycap::KeycapState;
use shell_model::locker::Lock;
use shell_model::notifications::{History, Notifications, ServerStatus};
use shell_model::polkit_agent::{ActionId, Polkit, Registration};

/// `bardo-shell notifications status|...`, `bardo-shell lock status`, `bardo-shell polkit status`, `bardo-shell clipboard
/// status`, `bardo-shell idle status`, `bardo-shell keycap status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusRequest {
    /// Who holds `org.freedesktop.Notifications`, and how many banners and kept notifications
    /// there are.
    Notifications,
    /// What the lock screen is doing.
    Lock,
    /// Whether bardo-shell's agent is registered and what it shows.
    Polkit,
    /// Whether history is kept, how, and how many entries; never what they hold.
    Clipboard,
    /// Who owns idle, where it has got to, and what holds it off.
    Idle,
    /// Whether keycap runs, and what it says about its mode and the keyboards.
    Keycap,
    /// Whether bardo-shell serves intentd's sheet, what waits on it, and the terminal's grants.
    Intents,
}

/// The one answer to a [`StatusRequest`], the same variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum StatusReport {
    Notifications(NotificationsStatus),
    Lock(Lock),
    Polkit(PolkitStatus),
    Clipboard(ClipboardStatus),
    Idle(Idle),
    Keycap(KeycapState),
    Intents(IntentsStatus),
}

/// The notifications server and what it holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationsStatus {
    pub server: ServerStatus,
    /// Live banners, the queued ones included.
    pub banners: u32,
    /// Notifications the history keeps.
    pub kept: u32,
}

impl NotificationsStatus {
    /// The status of the service's `live` state and its `history`.
    #[must_use]
    pub fn of(live: &Notifications, history: &History) -> Self {
        Self {
            server: live.server.clone(),
            banners: count(live.banners.len()),
            kept: count(history.len()),
        }
    }
}

/// The clipboard history's status and its size: a count, never an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClipboardStatus {
    pub status: ClipStatus,
    pub entries: u32,
}

impl ClipboardStatus {
    /// The status of `history`.
    #[must_use]
    pub fn of(history: &Clipboard) -> Self {
        Self {
            status: history.status,
            entries: count(history.entries.len()),
        }
    }
}

/// bardo-shell's polkit agent: whether it is registered, the action on screen and how many wait.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PolkitStatus {
    pub registration: Registration,
    /// The action on screen, if any.
    pub showing: Option<ActionId>,
    pub waiting: u32,
}

impl PolkitStatus {
    /// The status of `polkit`.
    #[must_use]
    pub fn of(polkit: &Polkit) -> Self {
        Self {
            registration: polkit.registration.clone(),
            showing: polkit.current.as_ref().map(|view| view.action_id.clone()),
            waiting: polkit.waiting,
        }
    }
}

/// Where bardo-shell stands with intentd: whether it serves the sheet, what is on it, and the terminal.
/// Names and counts only: a question's arguments are the person's, never on the wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentsStatus {
    pub link: LinkStatus,
    /// The confirmation on screen (its id), if any.
    pub showing: Option<String>,
    /// Questions waiting behind it.
    pub waiting: u32,
    /// The terminal's standing grants, `app:action`.
    pub grants: Vec<String>,
    pub breaker: Breaker,
    /// Whether intentd's `Control` answered the last read.
    pub reach: ControlReach,
}

/// Whether bardo-shell serves the sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkStatus {
    /// Not asked to serve yet.
    Passive,
    /// `org.bardo.Confirm1` and `org.bardo.Shell` are bardo-shell's.
    Serving,
    /// It could not take the names (no bus, a name taken); the log says which.
    Unavailable,
}

impl IntentsStatus {
    /// The status of `intents`.
    pub fn of(intents: &Intents) -> Self {
        IntentsStatus {
            link: match intents.link {
                IntentsLink::Passive => LinkStatus::Passive,
                IntentsLink::Serving => LinkStatus::Serving,
                IntentsLink::Unavailable(_) => LinkStatus::Unavailable,
            },
            showing: intents
                .sheets
                .current()
                .map(|pending| pending.request.id.as_str().to_owned()),
            waiting: count(intents.sheets.waiting()),
            grants: intents.terminal.grants.iter().map(grant_name).collect(),
            breaker: intents.terminal.breaker,
            reach: intents.terminal.reach,
        }
    }
}

fn count(len: usize) -> u32 {
    u32::try_from(len).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use shell_launcher::{ClipKey, UnixTime};
    use shell_model::clipboard::{ClipContent, ClipEntry, ClipProtocol, ClipText, HistoryMode};
    use shell_model::notifications::{NameHolder, NameOwner, ServerMode};

    use super::*;

    #[test]
    fn the_notifications_status_carries_the_holder_and_the_counts() {
        let live = Notifications {
            server: ServerStatus {
                mode: ServerMode::Own,
                holder: NameHolder::Other(NameOwner {
                    unique: ":1.9".to_owned(),
                    pid: Some(42),
                    command: Some("plasmashell".to_owned()),
                }),
            },
            banners: Vec::new(),
        };
        let status = NotificationsStatus::of(&live, &History::default());
        assert_eq!(status.server, live.server);
        assert_eq!((status.banners, status.kept), (0, 0));
        let passive = NotificationsStatus::of(&Notifications::default(), &History::default());
        assert_eq!(passive.server.mode, ServerMode::Passive);
        assert_eq!(passive.server.holder, NameHolder::NoBus);
    }

    #[test]
    fn the_clipboard_status_counts_entries_and_says_nothing_of_them() {
        let history = Clipboard {
            status: ClipStatus {
                mode: HistoryMode::Memory,
                protocol: ClipProtocol::Wlr,
            },
            entries: vec![ClipEntry {
                key: ClipKey(1),
                content: ClipContent::Text(ClipText(Arc::from("private"))),
                copied: UnixTime(0),
            }],
        };
        let status = ClipboardStatus::of(&history);
        assert_eq!(status.status, history.status);
        assert_eq!(status.entries, 1);
        let wire = serde_json::to_string(&status).unwrap_or_else(|e| panic!("{e}"));
        assert!(!wire.contains("private"), "{wire}");
    }

    #[test]
    fn a_polkit_refusal_and_the_action_on_screen_are_reported() {
        let status = PolkitStatus::of(&Polkit {
            registration: Registration::Refused("an agent is registered".to_owned()),
            current: None,
            waiting: 0,
        });
        assert_eq!(
            status.registration,
            Registration::Refused("an agent is registered".to_owned())
        );
        assert_eq!(status.showing, None);
    }
}
