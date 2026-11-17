//! The sheets' seam (the accounts service PLAN G5): the host that draws for accountd.
//!
//! A consent is one call; a
//! conversation (add an account, sign in again, a device code) is a link accountd updates with
//! views and reads the person's inputs from. accountd serves it over `org.bardo.AccountsSheet1`
//! (the shell draws), an app hosting the accounts service in process draws it itself, tests script it.

use crate::{SheetInput, SheetView};
use account_core::consent::{ConsentAnswer, ConsentAsk};
use account_wire::ParentWindow;

/// What a conversation opens with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetOpen {
    /// The window the sheet attaches to.
    pub window: ParentWindow,
    /// What it shows first.
    pub view: SheetView,
}

/// Why a conversation could not go on.
#[derive(Debug, thiserror::Error)]
pub enum SheetFault {
    /// No sheet host is reachable (the shell is not running, no app draws).
    #[error("no sheet host reachable")]
    Unavailable,
    /// The host closed the sheet or left.
    #[error("sheet closed")]
    Closed,
    /// The host could not be reached: the bus refused a call that finds or checks it.
    #[error("sheet host could not be reached")]
    Unreachable(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// The host did not take a call on an open sheet (it is gone, or it refused).
    #[error("sheet host did not take the call")]
    Refused(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl SheetFault {
    /// The host could not be reached, because of `cause`.
    pub fn unreachable(cause: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::Unreachable(cause.into())
    }

    /// The host did not take a call, because of `cause`.
    pub fn refused(cause: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::Refused(cause.into())
    }
}

/// Shows sheets for accountd and returns what the person did.
pub trait Sheets: Send + Sync {
    /// The open conversation `conversation` returns.
    type Link: SheetLink;

    /// Shows the chooser and consent sheet and returns the user's answer.
    fn consent(
        &self,
        ask: ConsentAsk,
        window: &ParentWindow,
    ) -> impl Future<Output = ConsentAnswer> + Send;

    /// Opens a conversation: the sheet is shown with `open.view`, and the link carries the
    /// rest. Dropping the link closes the sheet.
    fn conversation(
        &self,
        open: SheetOpen,
    ) -> impl Future<Output = Result<Self::Link, SheetFault>> + Send;
}

/// One open conversation with a sheet.
pub trait SheetLink: Send {
    /// Replaces what the sheet shows.
    fn update(&mut self, view: SheetView) -> impl Future<Output = Result<(), SheetFault>> + Send;

    /// The person's next input. Only the sheet's owner's input counts; the host checks that.
    fn input(&mut self) -> impl Future<Output = Result<SheetInput, SheetFault>> + Send;
}
