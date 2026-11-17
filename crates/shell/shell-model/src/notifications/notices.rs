//! Notices a service posts through the notifications service and answers when their buttons are
//! pressed.
//!
//! Which notification id carries which key, learnt from the server's own word when it posts, so a
//! press is answered only when the id is one bardo-shell itself posted for that key. The accounts, sync-hold and guest-asks services each
//! keep one of these over their own key (an account, a dataset, a computer); they differ only in
//! the words and in how an action key names its notice.

use std::collections::HashMap;
use std::hash::Hash;

use super::{ActionKey, NotesState, NotificationId};

/// The id of the notification posted for each key, kept from when the server said it posted it
/// until the server says it closed or the key's notice is withdrawn.
///
/// Only ids the server said for bardo-shell's own posts are kept, so an app's look-alike notice
/// answers nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notices<K: Eq + Hash>(HashMap<K, NotificationId>);

impl<K: Eq + Hash> Default for Notices<K> {
    fn default() -> Self {
        Notices(HashMap::new())
    }
}

impl<K: Eq + Hash + Clone> Notices<K> {
    /// A notification bardo-shell posted was given `id` and carries buttons with `actions`
    /// (`Said::Posted`): it is the notice of each key `read` finds in those actions (`key_of`
    /// names the key in what `read` returns). One posted afresh replaces the id kept for its key.
    ///
    /// Told by the notifications service itself, in order with the presses, so a press is never
    /// weighed before the id it names is known; nothing here reads the notes.
    pub fn posted<R>(
        &mut self,
        id: NotificationId,
        actions: &[ActionKey],
        read: impl Fn(&ActionKey) -> Option<R>,
        key_of: impl Fn(&R) -> &K,
    ) {
        for found in actions.iter().filter_map(read) {
            self.0.insert(key_of(&found).clone(), id);
        }
    }

    /// The id of the notice posted for `key`, while it stands.
    #[must_use]
    pub fn id_of(&self, key: &K) -> Option<NotificationId> {
        self.0.get(key).copied()
    }

    /// The server closed `id`: it answers nothing from now on.
    pub fn closed(&mut self, id: NotificationId) {
        self.0.retain(|_, kept| *kept != id);
    }

    /// The key's notice ended, or is being replaced: its id is no longer one.
    pub fn forget(&mut self, key: &K) {
        self.0.remove(key);
    }

    /// What a press answers: `pressed`, which a service reads from the button's action key, and
    /// only when `id` is the notification posted for the key it names (`key_of`). Any other id,
    /// or the right one with another key's button, answers nothing.
    pub fn answers<R>(
        &self,
        id: NotificationId,
        pressed: Option<R>,
        key_of: impl Fn(&R) -> &K,
    ) -> Option<R> {
        pressed.filter(|pressed| self.0.get(key_of(pressed)) == Some(&id))
    }
}

/// The notification already held, banner or history, that belongs to the app named `entry` and
/// carries a button with the action `key`.
#[must_use]
pub fn shown_with_action(
    notes: &NotesState,
    entry: &str,
    key: &ActionKey,
) -> Option<NotificationId> {
    let banners = notes.live.banners.iter().map(|banner| &banner.note);
    let kept = notes.history.groups.iter().flat_map(|group| &group.notes);
    banners
        .chain(kept)
        .filter(|note| note.app.desktop_entry.as_deref() == Some(entry))
        .find(|note| note.actions.iter().any(|action| action.key == *key))
        .map(|note| note.id)
}

#[cfg(test)]
mod tests {
    use super::{Notices, shown_with_action};
    use crate::notifications::{
        Action, ActionKey, AppGroup, AppKey, AppRef, BannerLife, Body, Expiry, Hints, IconRef,
        LiveBanner, NotesState, Notification, NotificationId, UnixMs,
    };

    const ENTRY: &str = "shell-test";

    fn key_of(name: &str) -> ActionKey {
        ActionKey(format!("press:{name}"))
    }

    fn name_of(key: &ActionKey) -> Option<String> {
        key.0.strip_prefix("press:").map(str::to_owned)
    }

    fn shown_for(notes: &NotesState, name: &str) -> Option<NotificationId> {
        shown_with_action(notes, ENTRY, &key_of(name))
    }

    fn note(id: u32, entry: &str, name: &str) -> Notification {
        Notification {
            id: NotificationId(id),
            arrived: UnixMs(0),
            app: AppRef {
                name: String::new(),
                icon: IconRef::None,
                desktop_entry: Some(entry.to_owned()),
            },
            summary: String::new(),
            body: Body::plain(""),
            actions: vec![Action {
                key: key_of(name),
                label: String::new(),
            }],
            hints: Hints::default(),
            expiry: Expiry::Default,
        }
    }

    fn banner(note: Notification) -> LiveBanner {
        LiveBanner {
            note,
            life: BannerLife::default(),
        }
    }

    /// What a press of `key` answers with id `id`, reading the key as the services do.
    fn answer(notices: &Notices<String>, id: u32, key: &ActionKey) -> Option<String> {
        notices.answers(NotificationId(id), name_of(key), |name| name)
    }

    /// `Said::Posted` for `id`, with buttons named by `names`: a bare name is a press of it.
    fn posted(notices: &mut Notices<String>, id: u32, names: &[&str]) {
        let keys: Vec<ActionKey> = names
            .iter()
            .map(|name| {
                if name.contains(':') || *name == "default" {
                    ActionKey((*name).to_owned())
                } else {
                    key_of(name)
                }
            })
            .collect();
        notices.posted(NotificationId(id), &keys, name_of, |name| name);
    }

    #[test]
    fn the_notice_shown_is_found_by_its_key_among_banners_and_history() {
        let (a, b) = ("a".to_owned(), "b".to_owned());
        let mut notes = NotesState::default();
        notes.live.banners = vec![
            banner(note(1, ENTRY, "a")),
            // Another app's action that happens to carry the same key is not ours.
            banner(note(2, "other", "b")),
        ];
        assert_eq!(shown_for(&notes, &a), Some(NotificationId(1)));
        assert_eq!(shown_for(&notes, &b), None);

        notes.live.banners.clear();
        notes.history.groups = vec![AppGroup {
            key: AppKey(ENTRY.to_owned()),
            notes: vec![note(7, ENTRY, "b")],
        }];
        assert_eq!(shown_for(&notes, &b), Some(NotificationId(7)));
        assert_eq!(shown_for(&notes, &a), None);
    }

    #[test]
    fn a_press_answers_only_for_the_id_posted_for_that_key() {
        let (a, b) = ("a".to_owned(), "b".to_owned());
        let mut notices = Notices::default();
        posted(&mut notices, 1, &["a"]);
        posted(&mut notices, 2, &["b"]);
        assert_eq!(answer(&notices, 1, &key_of("a")), Some(a.clone()));
        assert_eq!(answer(&notices, 2, &key_of("b")), Some(b.clone()));
        // The right id with the wrong key, a foreign id, and a key that is not a press.
        assert_eq!(answer(&notices, 1, &key_of("b")), None);
        assert_eq!(answer(&notices, 9, &key_of("a")), None);
        assert_eq!(answer(&notices, 1, &ActionKey("default".to_owned())), None);
        // Closed by the server: it answers nothing, and the other key's is untouched.
        notices.closed(NotificationId(1));
        assert_eq!(answer(&notices, 1, &key_of("a")), None);
        assert_eq!(answer(&notices, 2, &key_of("b")), Some(b.clone()));
        // A key whose notice ended forgets its id.
        notices.forget(&b);
        assert_eq!(answer(&notices, 2, &key_of("b")), None);
    }

    #[test]
    fn the_id_is_learned_from_the_stream() {
        let a = "a".to_owned();
        let mut notices = Notices::default();
        assert_eq!(notices.id_of(&a), None);
        // A post that carries no button of this service's teaches nothing.
        posted(&mut notices, 3, &["default", "other:a"]);
        assert_eq!(notices.id_of(&a), None);
        posted(&mut notices, 4, &["default", "press:a"]);
        assert_eq!(notices.id_of(&a), Some(NotificationId(4)));
    }

    #[test]
    fn a_press_with_an_unknown_id_is_ignored() {
        let mut notices = Notices::default();
        // An app's look-alike notice carries the same key but was never said as posted by us.
        assert_eq!(answer(&notices, 7, &key_of("a")), None);
        posted(&mut notices, 1, &["a"]);
        assert_eq!(answer(&notices, 7, &key_of("a")), None);
        assert_eq!(answer(&notices, 1, &key_of("a")), Some("a".to_owned()));
    }

    #[test]
    fn a_press_before_the_notes_show_the_notice_still_answers() {
        // The notes are never consulted: the id was said when posted, and the press follows on
        // the same stream, whether or not the notes ever held the notice.
        let a = "a".to_owned();
        let notes = NotesState::default();
        assert_eq!(shown_for(&notes, &a), None);
        let mut notices = Notices::default();
        posted(&mut notices, 5, &["a"]);
        assert_eq!(answer(&notices, 5, &key_of("a")), Some(a));
    }

    #[test]
    fn a_fresh_post_replaces_the_id_kept() {
        let a = "a".to_owned();
        let mut notices = Notices::default();
        posted(&mut notices, 1, &["a"]);
        posted(&mut notices, 5, &["a"]);
        assert_eq!(answer(&notices, 1, &key_of("a")), None);
        assert_eq!(answer(&notices, 5, &key_of("a")), Some(a));
    }
}
