//! What the notifications server tells the services that posted a notice: the answer to a
//! button, and the end of the notice.

use super::{ActionKey, CloseReason, NotificationId};

/// What happened to a notification the server held.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Said {
    /// A notification bardo-shell itself posted (not one an app sent over the bus) was given `id`,
    /// and carries buttons with these action keys. Said before any press of `id` can be, on the
    /// same stream, so a poster learns its ids in order with the presses.
    Posted {
        id: NotificationId,
        actions: Vec<ActionKey>,
    },
    /// An action was invoked: which notification, and which action of it.
    Pressed { id: NotificationId, key: ActionKey },
    /// The notification closed, and why.
    Closed {
        id: NotificationId,
        reason: CloseReason,
    },
}
