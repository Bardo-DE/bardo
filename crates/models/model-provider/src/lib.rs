//! One trait for the models behind every engine and account, local or cloud.
//!
//! A [`Provider`] is one endpoint (a running engine or a cloud account). A turn pushes events
//! into a [`TurnSink`]; cancellation is dropping the future.
//!
//! ```
//! use model_provider::{JsonText, ProviderDetail, ProviderError, RetryClass};
//!
//! // Model output crosses the boundary as checked JSON text.
//! assert!(JsonText::new(r#"{"score": 7}"#).is_ok());
//! assert!(JsonText::new("{").is_err());
//!
//! // An error is something a caller acts on: no credit is not worth a retry.
//! let error = ProviderError::PaymentRequired(ProviderDetail::new(402, "can only afford 367 tokens"));
//! assert_eq!(error.retry_class(), RetryClass::Never);
//! ```
#![expect(
    missing_docs,
    reason = "lints baseline: this crate documents its public items when its domain is refactored (plan phases 3-7)"
)]

mod caps;
mod control;
mod detail;
mod embed;
mod event;
mod ids;
mod logprobs;
mod money;
mod provider;
mod request;
mod retry;
#[cfg(feature = "testing")]
mod scripted;
mod sequence;
mod shape;
mod units;

pub use caps::{
    Batching, Caps, ComputerUseSupport, Constraint, ImageLimits, InputKind, ShapeWithTools,
    Support, ToolSupport, Zoom,
};
/// Re-exported so a crate that builds `ImageLimits` need not depend on `computer-use-action` and
/// `model-vision-prep` itself (a dependency edge changes every consumer's lock file).
pub use computer_use_action::ModelSpace;
pub use control::{
    EngineExtras, KeepAlive, LlamaExtras, OllamaExtras, PromptCache, Sampling, ThoughtSeal,
    ToolParallelism, VllmExtras,
};
pub use detail::{ProviderDetail, redact_excerpt};
pub use embed::{
    EmbedCaps, EmbedEnd, EmbedError, EmbedPrompts, EmbedRole, EmbedTurn, EmbedVector, Embedder,
    PrefixText, plan_batches,
};
pub use event::{SafetySignal, StopReason, TurnEnd, TurnEvent, TurnSink, TurnUsage, UsageReport};
pub use ids::{
    JsonError, JsonText, ModelName, OpaqueText, SchemaText, SignatureText, ToolCallId, ToolName,
    ToolNameError,
};
pub use logprobs::{ChoiceScores, FirstTokenLogprobs, Logprob, TokenLogprob};
pub use model_vision_prep::ResizeRule;
pub use money::{Charges, Money, MoneyError};
pub use provider::{ModelInfo, Provider, ProviderError};
pub use request::{
    Effort, ImageBytes, ImageDetail, ImageInput, Limits, Message, NativeTool, OutputShape, Part,
    Reasoning, Role, ToolCall, ToolChoice, ToolResult, ToolSpec, ToolStatus, TurnRequest,
};
pub use retry::{RetryClass, RetryPolicy, Retrying, Sleeper, next_wait};
#[cfg(feature = "testing")]
pub use scripted::{Script, ScriptedProvider};
pub use sequence::{SequenceError, check as check_sequence};
pub use shape::{
    ChoiceText, Extract, Field, FieldName, IdentError, KeywordText, SchemaDialect, SchemaError,
    SchemaKeywordError, SchemaLimits, SchemaPath, Shape, ShapeError, ShapeKind, Variant,
    VariantName, sanitize_schema,
};
pub use units::{
    Attempt, BatchMax, CallIndex, CharCount, ImageCount, Knob, Milli, Permille, RepairBudget,
    RetrySeconds, Seconds, Seed, ServerStatus, SlotId, WaitMs,
};
