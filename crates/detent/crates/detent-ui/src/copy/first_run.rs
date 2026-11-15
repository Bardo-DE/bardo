//! The first-run window's words: how a local runtime stands.

use detent_model::{AccountStatus, Runtime};

/// What a person reads for how `runtime` stands: a runtime that is up is running.
pub(crate) fn runtime_state(runtime: &Runtime) -> String {
    match &runtime.status {
        AccountStatus::Ok => "Running".to_owned(),
        status => super::accounts::status_label(status),
    }
}

/// The words "The model settings are not installed.".
pub(crate) const THE_MODEL_SETTINGS_ARE_NOT_INSTALLED: &str =
    "The model settings are not installed.";

/// The words "Could not remember this: {error}".
pub(crate) fn could_not_remember_this(error: impl std::fmt::Display) -> String {
    format!("Could not remember this: {error}")
}

/// The words "Welcome".
pub(crate) const WELCOME: &str = "Welcome";

/// The words "Keep AI on this computer".
pub(crate) const KEEP_AI_ON_THIS_COMPUTER: &str = "Keep AI on this computer";
