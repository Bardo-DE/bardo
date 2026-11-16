//! The seam between porter's wire types and stoker's provider types (`model-provider`).
//!
//! `ChatRequest` to `TurnRequest`, `ToolCallPart` to `ToolCall`, stoker's `TurnEvent` to
//! `InferEvent`, `ProviderError` to `ModelError`. The mapping itself is
//! `inference-bridge` (shared with inference-client's in-process engine host); what stays here is the
//! half that reads inferd's own model book (`local`): the turn target of a `LocalModel`, and the
//! embedding limits of its entry.

mod request;

pub use inference_bridge::{
    BridgeError, DefaultSampling, Frames, Gathered, JsonReply, MAX_ATTACHMENT, NoScores, Target,
    chat_turn_for, embed_turns_for, event, extras, image_input, model_error, model_id_of,
    task_turn_for, turn_scores, usage,
};
pub use request::{chat_turn, embed_turns, local_target, task_turn};
