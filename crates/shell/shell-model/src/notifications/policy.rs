//! How a notification is presented: the daemon's settings and focus mode as plain values (this
//! crate does not read `shell-settings`: `scripts/check-boundary.sh`), and the pure decisions
//! they drive.
//!
//! The daemon sends a [`NotificationsPolicy`] at start and whenever `notifications.*` or the
//! focus mode changes (`NotificationsCommand::Policy`). Until the first one the service is
//! passive and presents with the defaults below: a test that spawns the service never asks for
//! the bus name.

use serde::{Deserialize, Serialize};
use ui_core::word::Word;

use super::content::{Expiry, Notification, NotifyUrgency, Residency, SoundHint};
use super::origin::Origin;
use super::{BannerLife, ServerMode};

/// Everything the service is told by the daemon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct NotificationsPolicy {
    /// Whether to own the bus name (fixed by the daemon).
    pub server: ServerMode,
    /// Do Not Disturb (`notifications.dnd`): on holds banners and sounds back, except the
    /// shell's own critical notices; everything still enters the history.
    pub quiet: bool,
    /// `notifications.banner_style`.
    pub style: Presentation,
    /// The most notifications kept.
    pub history_cap: HistoryCap,
    /// `notifications.swipe`.
    pub swipe: Swipe,
}

/// `notifications.swipe`: what swiping a banner away does to its notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum Swipe {
    /// The banner leaves; the notification stays in the notification center (macOS).
    #[default]
    KeepInCenter,
    /// The notification is dismissed: gone from the history too.
    Dismiss,
}

/// `notifications.banner_style`: how an app's notifications show by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum Presentation {
    /// A banner that leaves by itself.
    #[default]
    Banner,
    /// A banner that stays until acted on.
    Alert,
    /// No banner: the history only.
    #[serde(rename = "none")]
    #[word(slug = "none", label = "None")]
    HistoryOnly,
}

/// The most notifications the history keeps; the oldest go first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HistoryCap(pub u16);

impl Default for HistoryCap {
    /// The history cap's value.
    fn default() -> Self {
        HistoryCap(100)
    }
}

/// How one notification shows on arrival.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Presented {
    Banner(BannerLife),
    HistoryOnly,
}

/// Whether a notification's arrival makes a sound.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Chime {
    Play(SoundHint),
    Silent,
}

/// How `note` shows under `policy`.
///
/// The shell's own critical notice always shows, as a persisting banner, through Do Not Disturb and
/// a history-only style. An app's critical one persists the same way (until `APP_CRITICAL_HOLD_MS`,
/// see `rules::sweep`) but holds back like any other while Do Not Disturb is on or the style is
/// history-only. Otherwise: nothing but the history while held back; else a banner that persists
/// for `resident`, `expire_timeout == 0` or the Alert style, and expires otherwise.
#[must_use]
pub fn presented(note: &Notification, policy: &NotificationsPolicy) -> Presented {
    let critical = note.hints.urgency == NotifyUrgency::Critical;
    let through = critical && Origin::of(&note.app) == Origin::System;
    let held_back = policy.quiet || policy.style == Presentation::HistoryOnly;
    let persists = critical
        || note.hints.residency == Residency::Resident
        || note.expiry == Expiry::Never
        || policy.style == Presentation::Alert;
    match (through, held_back, persists) {
        (true, _, _) => Presented::Banner(BannerLife::Persists),
        (false, true, _) => Presented::HistoryOnly,
        (false, false, true) => Presented::Banner(BannerLife::Persists),
        (false, false, false) => Presented::Banner(BannerLife::Expires),
    }
}

/// Whether `note`'s arrival plays a sound: never when the sender suppressed it, nor while Do
/// Not Disturb holds it back (the shell's own critical notice still sounds); else its hint (or
/// the default).
#[must_use]
pub fn chime(note: &Notification, policy: &NotificationsPolicy) -> Chime {
    let through =
        note.hints.urgency == NotifyUrgency::Critical && Origin::of(&note.app) == Origin::System;
    match (&note.hints.sound, policy.quiet) {
        (SoundHint::Suppressed, _) => Chime::Silent,
        (_, true) if !through => Chime::Silent,
        (hint, _) => Chime::Play(hint.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::{Chime, NotificationsPolicy, Presentation, Presented, chime, presented};
    use crate::notifications::BannerLife;
    use crate::notifications::Body;
    use crate::notifications::{
        AppRef, Expiry, Hints, IconRef, Notification, NotifyUrgency, Residency, SoundHint,
    };
    use crate::notifications::{NotificationId, UnixMs};

    fn system(mut note: Notification) -> Notification {
        note.app.desktop_entry = Some("shell-keycap".to_owned());
        note
    }

    fn note(urgency: NotifyUrgency, residency: Residency, expiry: Expiry) -> Notification {
        Notification {
            id: NotificationId(1),
            arrived: UnixMs(0),
            app: AppRef {
                name: "app".to_owned(),
                icon: IconRef::None,
                desktop_entry: None,
            },
            summary: "s".to_owned(),
            body: Body::default(),
            actions: Vec::new(),
            hints: Hints {
                urgency,
                residency,
                ..Hints::default()
            },
            expiry,
        }
    }

    fn policy(quiet: bool, style: Presentation) -> NotificationsPolicy {
        NotificationsPolicy {
            quiet,
            style,
            ..NotificationsPolicy::default()
        }
    }

    #[test]
    fn presentation_follows_urgency_focus_and_style() {
        use BannerLife::{Expires, Persists};
        use NotifyUrgency::{Critical, Low, Normal};
        use Presentation::{Alert, Banner, HistoryOnly};
        use Residency::{ClosesOnAction, Resident};
        let plain = |u| note(u, ClosesOnAction, Expiry::Default);
        let cases = [
            (
                "normal",
                plain(Normal),
                policy(false, Banner),
                Presented::Banner(Expires),
            ),
            (
                "low",
                plain(Low),
                policy(false, Banner),
                Presented::Banner(Expires),
            ),
            (
                "app asks 3 s",
                note(Normal, ClosesOnAction, Expiry::Ms(3000)),
                policy(false, Banner),
                Presented::Banner(Expires),
            ),
            (
                "expire 0",
                note(Normal, ClosesOnAction, Expiry::Never),
                policy(false, Banner),
                Presented::Banner(Persists),
            ),
            (
                "resident",
                note(Normal, Resident, Expiry::Default),
                policy(false, Banner),
                Presented::Banner(Persists),
            ),
            (
                "alert style",
                plain(Normal),
                policy(false, Alert),
                Presented::Banner(Persists),
            ),
            (
                "an app's critical persists",
                plain(Critical),
                policy(false, Banner),
                Presented::Banner(Persists),
            ),
            (
                "the shell's critical",
                system(plain(Critical)),
                policy(false, Banner),
                Presented::Banner(Persists),
            ),
            (
                "dnd",
                plain(Normal),
                policy(true, Banner),
                Presented::HistoryOnly,
            ),
            (
                "dnd alert",
                plain(Normal),
                policy(true, Alert),
                Presented::HistoryOnly,
            ),
            (
                "dnd holds an app's critical back",
                plain(Critical),
                policy(true, Banner),
                Presented::HistoryOnly,
            ),
            (
                "dnd lets the shell's critical through",
                system(plain(Critical)),
                policy(true, Banner),
                Presented::Banner(Persists),
            ),
            (
                "history only",
                plain(Normal),
                policy(false, HistoryOnly),
                Presented::HistoryOnly,
            ),
            (
                "history only holds an app's critical back",
                plain(Critical),
                policy(false, HistoryOnly),
                Presented::HistoryOnly,
            ),
            (
                "history only lets the shell's critical through",
                system(plain(Critical)),
                policy(false, HistoryOnly),
                Presented::Banner(Persists),
            ),
        ];
        for (name, note, policy, want) in cases {
            assert_eq!(presented(&note, &policy), want, "{name}");
        }
    }

    #[test]
    fn sounds_follow_the_hint_and_do_not_disturb() {
        let with_sound = |urgency, sound: SoundHint| {
            let mut n = note(urgency, Residency::ClosesOnAction, Expiry::Default);
            n.hints.sound = sound;
            n
        };
        let named = SoundHint::Named("message-new-email".to_owned());
        let cases = [
            (
                "default",
                with_sound(NotifyUrgency::Normal, SoundHint::Default),
                false,
                Chime::Play(SoundHint::Default),
            ),
            (
                "named",
                with_sound(NotifyUrgency::Normal, named.clone()),
                false,
                Chime::Play(named.clone()),
            ),
            (
                "suppressed",
                with_sound(NotifyUrgency::Normal, SoundHint::Suppressed),
                false,
                Chime::Silent,
            ),
            (
                "dnd",
                with_sound(NotifyUrgency::Normal, named.clone()),
                true,
                Chime::Silent,
            ),
            (
                "dnd holds an app's critical sound back",
                with_sound(NotifyUrgency::Critical, named.clone()),
                true,
                Chime::Silent,
            ),
            (
                "dnd lets the shell's critical sound through",
                system(with_sound(NotifyUrgency::Critical, named.clone())),
                true,
                Chime::Play(named),
            ),
            (
                "critical suppressed",
                system(with_sound(NotifyUrgency::Critical, SoundHint::Suppressed)),
                true,
                Chime::Silent,
            ),
        ];
        for (name, note, quiet, want) in cases {
            assert_eq!(
                chime(&note, &policy(quiet, Presentation::Banner)),
                want,
                "{name}"
            );
        }
    }
}
