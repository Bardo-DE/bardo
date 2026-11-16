//! What the Local models group says: a model's name, what it is doing, the memory it takes and
//! the action it gets.

use settings_app_model::{RunAction, RunStanding};

/// The word the row shows for what the model is doing.
pub(crate) fn standing(standing: &RunStanding) -> &'static str {
    match standing {
        RunStanding::Stopped => "Stopped",
        RunStanding::Starting => "Starting\u{2026}",
        RunStanding::Ready => "Ready",
        RunStanding::Stopping => "Stopping\u{2026}",
        RunStanding::Retrying => "Trying again\u{2026}",
        RunStanding::Failed(_) => "Couldn\u{2019}t start",
    }
}

/// The words on the row's button.
pub(crate) fn action(action: RunAction) -> &'static str {
    match action {
        RunAction::Start => "Start",
        RunAction::Stop => "Stop",
    }
}

/// The name of a model `account/model`: the model alone, since the account is the computer's own.
pub(crate) fn model_name(model: &str) -> &str {
    model.split_once('/').map_or(model, |(_, name)| name)
}

/// `mib` MiB as the person reads memory: gigabytes rounded to one decimal from 1 GB, else
/// megabytes.
pub(crate) fn memory_words(mib: u32) -> String {
    match mib {
        0..=1023 => format!("{mib} MB"),
        _ => {
            let tenths = (u64::from(mib) * 10 + 512) / 1024;
            format!("{}.{} GB", tenths / 10, tenths % 10)
        }
    }
}
