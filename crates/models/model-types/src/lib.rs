//! The data a model turn is made of: requests, streaming events, capabilities, errors, ids and
//! units, the money a provider reports, and the names of the dialects a computer-use model speaks.
//!
//! A turn pushes events into a [`TurnSink`]; the `Provider` trait that runs one is in
//! `model-core`.
//!
//! ```
//! use model_types::{JsonText, ProviderDetail, ProviderError, RetryClass};
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
mod dialect;
mod embed;
mod event;
mod ids;
mod logprobs;
mod money;
mod profile;
mod provider;
mod request;
mod retry;
mod shape;
mod trained;
mod units;

pub use caps::{
    Batching, Caps, ComputerUseSupport, Constraint, ImageLimits, InputKind, ShapeWithTools,
    Support, ToolSupport, Zoom,
};
pub use control::{
    EngineExtras, KeepAlive, LlamaExtras, OllamaExtras, PromptCache, Sampling, ThoughtSeal,
    ToolParallelism, VllmExtras,
};
pub use detail::{ProviderDetail, redact_excerpt};
pub use dialect::{ComputerUseDialect, TextDialect, ToolDialect, WireDialect};
pub use embed::{
    EmbedCaps, EmbedEnd, EmbedError, EmbedPrompts, EmbedRole, EmbedTurn, EmbedVector, PrefixText,
};
pub use event::{SafetySignal, StopReason, TurnEnd, TurnEvent, TurnSink, TurnUsage, UsageReport};
pub use ids::{
    JsonError, JsonText, ModelName, OpaqueText, SchemaText, SignatureText, ToolCallId, ToolName,
    ToolNameError,
};
pub use logprobs::{ChoiceScores, FirstTokenLogprobs, Logprob, TokenLogprob};
pub use money::{Charges, Currency, Money, MoneyError};
pub use profile::{ActionProfile, DragStyle, ScrollAxis, VerbAlias, VerbAliases};
pub use provider::{ModelInfo, ProviderError};
pub use request::{
    Effort, ImageBytes, ImageDetail, ImageInput, Limits, Message, NativeTool, OutputShape, Part,
    Reasoning, Role, ToolCall, ToolChoice, ToolResult, ToolSpec, ToolStatus, TurnRequest,
};
pub use retry::RetryClass;
pub use shape::{
    ChoiceText, Extract, Field, FieldName, IdentError, KeywordText, SchemaDialect, SchemaError,
    SchemaKeywordError, SchemaLimits, SchemaPath, Shape, ShapeError, ShapeKind, Variant,
    VariantName, sanitize_schema,
};
pub use trained::{AnswerShape, CallFormat, HistoryShape, ResolutionWording};
pub use units::{
    Attempt, BatchMax, CallIndex, CharCount, ImageCount, Knob, Milli, Permille, RepairBudget,
    RetrySeconds, Seconds, Seed, ServerStatus, SlotId, WaitMs,
};
