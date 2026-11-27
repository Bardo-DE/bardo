//! The accounts service's requests as the model layer's turns over a model of inferd's book: the turn target of a
//! `LocalModel` (its catalog entry's default sampling and output limit, its engine's flavor), and
//! the embedding limits of its entry. The mapping is `inference_bridge`'s.

use inference_bridge::{
    BridgeError, DefaultSampling, Frames, JsonReply, Target, chat_turn_for, embed_turns_for,
    task_turn_for,
};
use inference_core as pi;
use inference_router::local::LocalModel;
use model_types as sp;

/// The turn target of a model on this computer.
#[must_use]
pub fn local_target(model: &LocalModel) -> Target {
    Target {
        name: model.name.clone(),
        sampling: DefaultSampling::Entry(model.sampling()),
        max_output: model.caps().map(|caps| caps.max_output).unwrap_or_default(),
        flavor: model.flavor,
        json: model
            .caps()
            .map(|caps| JsonReply::of(&caps.output))
            .unwrap_or_default(),
    }
}

/// The turn for a chat request. Sampling and the output limit the request leaves open come from
/// the model's catalog entry.
///
/// # Errors
///
/// Returns a [`BridgeError`] when the request cannot be expressed as a turn for this model.
pub fn chat_turn(
    model: &LocalModel,
    request: &pi::ChatRequest,
    frames: &Frames,
) -> Result<sp::TurnRequest, BridgeError> {
    chat_turn_for(&local_target(model), request, frames)
}

/// A task as a chat turn: its instruction, then the text.
///
/// # Errors
///
/// Returns a [`BridgeError`] when the task cannot be expressed as a turn for this model.
pub fn task_turn(
    model: &LocalModel,
    request: &pi::TaskRequest,
    tier: account_capability::Tier,
) -> Result<sp::TurnRequest, BridgeError> {
    task_turn_for(&local_target(model), request, tier)
}

/// The embedding turns of a request: the model's prefix for the request's role goes before every
/// text, and the texts are cut into batches of the model's limit.
///
/// # Errors
///
/// Returns [`BridgeError::Unsupported`] when the model does not embed, and any other
/// [`BridgeError`] the request's texts cause.
pub fn embed_turns(
    model: &LocalModel,
    request: &pi::EmbedRequest,
) -> Result<Vec<sp::EmbedTurn>, BridgeError> {
    let embed = model.embed().ok_or(BridgeError::Unsupported)?;
    Ok(embed_turns_for(&model.name, embed, request))
}

#[cfg(test)]
mod tests;
