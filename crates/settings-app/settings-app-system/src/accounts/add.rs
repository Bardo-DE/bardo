//! Opening the add sheet from Settings, through porter's client: one sheet in the account service
//! adds everything, an identity or an AI key. The Settings role may call `AddAccount`. The sheet
//! is asked of the owner of the service's name that was confirmed, never of the name.

use crate::bus::pinned_starting;
use account_client::AccountsError;
use account_core::ProviderHint;
use account_wire::names::ACCOUNTS_BUS;
use account_wire::{ParentWindow, Refusal};
use settings_app_link::bus::BusSource;
use settings_app_link::{Deadline, answered, peer};
use settings_app_model::{AccountKey, AddEnd};

/// Asks for the add sheet and waits for it to end.
pub async fn add_account(bus: &BusSource) -> AddEnd {
    let Ok(accountd) = pinned_starting(bus, ACCOUNTS_BUS).await else {
        return AddEnd::Failed;
    };
    let Ok(porter) = accountd.accounts() else {
        return AddEnd::Failed;
    };
    let registry = porter.accounts();
    let asked = answered(
        Deadline::PROMPT,
        peer::ACCOUNTS,
        Box::pin(registry.add(ProviderHint::Any, &ParentWindow::Unparented)),
    )
    .await;
    match asked {
        Some(Ok(_)) => AddEnd::Added,
        Some(Err(error)) => end_of(&error),
        None => AddEnd::Failed,
    }
}

/// How a failed ask ended.
fn end_of(error: &AccountsError) -> AddEnd {
    match error {
        AccountsError::AlreadyAdded(id) => AddEnd::AlreadyThere(AccountKey(id.as_str().to_owned())),
        AccountsError::Refused(Refusal::Dismissed) => AddEnd::Closed,
        AccountsError::Busy => AddEnd::Busy,
        _ => AddEnd::Failed,
    }
}
