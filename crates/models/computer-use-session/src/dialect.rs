//! What one action dialect does differently from the others, as a trait; [`spec`] is the one
//! place that maps a [`ComputerUseDialect`] to its implementation.

use std::borrow::Cow;

use computer_use_geometry::ModelSpace;
use computer_use_parse::{ParseError, ParseLimits, Parsed, ToolContext};
use model_types::ToolSpec;
use model_types::{ActionProfile, ComputerUseDialect, ToolDialect, WireDialect};
use model_vision_prep::FrameMap;

use crate::{ComputerUseTaskText, TurnTranscript};

mod tool;
mod ui_tars;
mod wire;

/// How a model's prompt is built and its reply read. Every method has a default, so a dialect
/// overrides only what differs; only [`parse`](ActionDialect::parse) is required.
pub trait ActionDialect: Sync {
    /// The vendor protocol, when the model speaks one.
    fn wire(&self) -> Option<WireDialect> {
        None
    }

    /// Whether the model answers in action text.
    fn is_text(&self) -> bool {
        false
    }

    /// Whether the engine parses the model's tool calls (or we read them from its text).
    fn is_tool(&self) -> bool {
        false
    }

    /// The dialect's own system text, or `None` when its tool declaration carries the prompt.
    fn system(
        &self,
        _actions: ActionProfile,
        _map: &FrameMap,
        _task: &ComputerUseTaskText,
    ) -> Option<String> {
        None
    }

    /// The parameter schema of the function the dialect declares, as the model is shown it.
    fn schema(&self, _actions: ActionProfile) -> Option<Cow<'static, str>> {
        None
    }

    /// The tools the dialect declares (a request declares them unless we read the calls ourselves).
    fn tools(&self, _actions: ActionProfile, _map: &FrameMap) -> Vec<ToolSpec> {
        Vec::new()
    }

    /// What a repair message asks of the model.
    fn retry(&self, _actions: ActionProfile) -> String {
        "Answer again with one call to the computer tool.".to_owned()
    }

    /// Reads one reply.
    ///
    /// # Errors
    ///
    /// [`ParseError`] when the reply holds no action the dialect accepts or breaks the limits.
    fn parse(
        &self,
        space: ModelSpace,
        context: ToolContext,
        reply: &TurnTranscript,
        limits: ParseLimits,
    ) -> Result<Parsed, ParseError>;
}

/// The implementation of `dialect`. The only `match` on `ComputerUseDialect` that picks behaviour.
#[must_use]
pub fn spec(dialect: ComputerUseDialect) -> &'static dyn ActionDialect {
    match dialect {
        ComputerUseDialect::Text(text) => ui_tars::of(text),
        ComputerUseDialect::Tool(ToolDialect::QwenComputerUse) => &tool::Qwen,
        ComputerUseDialect::Tool(ToolDialect::Holo31) => &tool::Holo,
        ComputerUseDialect::Wire(wire) => wire::of(wire),
    }
}
