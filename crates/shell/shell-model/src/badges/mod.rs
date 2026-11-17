//! App badges: `LauncherEntry` counts, progress and urgency per app, as the dock draws them.

mod command;
mod model;
mod step;
#[cfg(test)]
mod tests;

pub use command::{BadgeEvent, BadgeProp, BadgesCommand};
pub use model::{APP_ID_MAX_BYTES, BADGE_APPS_MAX, Badge, Badges, parse_app_uri};
pub use step::step;

/// `LauncherEntry` badges as a service.
///
/// `LauncherEntry` badges as a service: follows `LauncherEntry.Update` on the session bus and
/// senders leaving it (`NameOwnerChanged` clears their apps' badges). Without a session bus the
/// state stays at `Badges::default()`.
#[derive(Debug)]
pub enum BadgesService {}

impl crate::service::Service for BadgesService {
    const NAME: &'static str = "badges";
    type State = Badges;
    type Command = BadgesCommand;
}
