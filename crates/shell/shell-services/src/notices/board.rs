//! One service's notices on the notifications service: post, replace, withdraw, and know which
//! notification ids are its own.

use std::hash::Hash;

use shell_model::notifications::{
    ActionKey, NotesState, Notices, NotificationId, NotificationsCommand, Posting,
};
use tokio::sync::watch;

use shell_model::service::Control;

/// The notices of one service, one per key (an account, a dataset, a computer; `()` for a service
/// with a single notice).
#[derive(Debug)]
pub(crate) struct Board<K: Eq + Hash> {
    notifications: Control<NotificationsCommand>,
    notes: watch::Receiver<NotesState>,
    posted: Notices<K>,
    /// The notification already held for a key, banner or history.
    shown_for: fn(&NotesState, &K) -> Option<NotificationId>,
}

impl<K: Eq + Hash + Clone> Board<K> {
    /// A board over the notifications service, finding a key's held notice with `shown_for`.
    pub(crate) fn new(
        notifications: Control<NotificationsCommand>,
        notes: watch::Receiver<NotesState>,
        shown_for: fn(&NotesState, &K) -> Option<NotificationId>,
    ) -> Board<K> {
        Board {
            notifications,
            notes,
            posted: Notices::default(),
            shown_for,
        }
    }

    /// The server said it posted `id` with buttons `actions` (`Said::Posted`): keep it as the
    /// notice of each key `read` finds in them.
    pub(crate) fn posted<R>(
        &mut self,
        id: NotificationId,
        actions: &[ActionKey],
        read: impl Fn(&ActionKey) -> Option<R>,
        key_of: impl Fn(&R) -> &K,
    ) {
        self.posted.posted(id, actions, read, key_of);
    }

    /// Post the notice for `key`, in place of the one held when there is one. `posting` is given
    /// the id to replace.
    pub(crate) fn post(
        &mut self,
        key: &K,
        posting: impl FnOnce(Option<NotificationId>) -> Posting,
    ) {
        let replaces = self.held(key);
        self.posted.forget(key);
        self.send(NotificationsCommand::Post(Box::new(posting(replaces))));
    }

    /// Take the notice for `key` back, when one is held.
    pub(crate) fn withdraw(&mut self, key: &K) {
        let held = self.held(key);
        self.posted.forget(key);
        if let Some(id) = held {
            self.send(NotificationsCommand::Dismiss(id));
        }
    }

    /// The server closed `id`: it answers nothing from now on.
    pub(crate) fn closed(&mut self, id: NotificationId) {
        self.posted.closed(id);
    }

    /// What a press answers (see [`Notices::answers`]).
    pub(crate) fn answers<R>(
        &self,
        id: NotificationId,
        pressed: Option<R>,
        key_of: impl Fn(&R) -> &K,
    ) -> Option<R> {
        self.posted.answers(id, pressed, key_of)
    }

    /// The notifications service itself, for a notice that is not tied to a key.
    pub(crate) fn notifications(&self) -> Control<NotificationsCommand> {
        self.notifications.clone()
    }

    /// The notification held for `key`: the one said as posted, else one the notes hold from
    /// before (banner or history).
    fn held(&self, key: &K) -> Option<NotificationId> {
        self.posted
            .id_of(key)
            .or_else(|| (self.shown_for)(&self.notes.borrow(), key))
    }

    fn send(&self, command: NotificationsCommand) {
        // A stopped notifications service has nobody to tell.
        let _ = self.notifications.send(command);
    }
}
