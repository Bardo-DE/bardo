//! The accounts service as the shell sees it: which accounts need the person to sign in again, and
//! the notification that tells them (design/31 §4.4, PLAN §2.8).
//!
//! The events that move the state, the pure step over them, and the notice's words, its action and
//! how to find the notice already shown.

mod command;
mod model;
mod notice;
pub mod sheet;
mod step;
#[cfg(test)]
mod tests;

pub use command::{AccountsCommand, AccountsEffect, AccountsEvent};
pub use model::{AccountKey, AccountStanding, AccountsState, NeedsSignIn, SignInAsk};
pub use notice::{
    NOTICE_ENTRY, Notice, account_of, no_launcher_text, notice_text, shown_for, sign_in_key,
};
pub use step::step;

/// The accounts that need signing in again, on `env`'s session bus, told through the
/// notifications service in its `AccountsLinks`.
#[derive(Debug)]
pub enum AccountsService {}

impl crate::service::Service for AccountsService {
    const NAME: &'static str = "accounts";
    type State = AccountsState;
    type Command = AccountsCommand;
}
