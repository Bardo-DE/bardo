//! The seam a run reaches its model through: inferd in the computer-use daemon, a script in tests.

use crate::screen::{ScreenRead, ScreenText};
use crate::turn::FrameAttachment;
use computer_use_action::{ComputerUseAction, WindowSpace};
use inference_core::{
    CuaBegin, CuaStepFailure, CuaStepReply, CuaStepRequest, ModelError, ServedBy, TurnUsage,
};
use serde::{Deserialize, Serialize};

/// What a model says while a step is still being decided.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum StepEvent {
    /// Some of the model's thought, as it streams (text of the model: `Debug` never prints it).
    ThoughtDelta(ThoughtText),
    /// An action the model proposed, in window space, before the batch is complete.
    ActionProposed(ComputerUseAction<WindowSpace>),
}

/// A piece of a model's thought. `Debug` prints its length only.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ThoughtText(pub String);

impl std::fmt::Debug for ThoughtText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ThoughtText(<{} bytes>)", self.0.len())
    }
}

/// Where a step's events go while the model works.
pub trait StepSink: Send {
    /// One event.
    fn event(&mut self, event: StepEvent);
}

/// Why a model call failed, in the words the run machine counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum ComputerUseModelError {
    /// Worth retrying: the engine is busy, loading or unreachable.
    #[error("transient")]
    Transient,
    /// The reply could not be read even after inferd's one repair; counts as a failed step.
    #[error("unparseable reply")]
    Parse,
    /// Retrying will not help.
    #[error("fatal")]
    Fatal,
    /// The model cannot read images, so it cannot do work from the screen. Never retried: the
    /// person has to choose another.
    #[error("cannot see the screen")]
    CannotSeeScreen,
}

impl From<ComputerUseStepFailure> for ComputerUseModelError {
    /// inferd's failure, sorted into what the machine can act on: an unreachable, rate-limited
    /// or not-ready engine is transient; an unreadable reply is a parse failure; a refusal, an
    /// authorisation failure or a context overflow is fatal.
    fn from(failure: ComputerUseStepFailure) -> Self {
        match failure {
            ComputerUseStepFailure::Unparseable => ComputerUseModelError::Parse,
            ComputerUseStepFailure::ModelFailed(error) => match error {
                ModelError::Unreachable | ModelError::RateLimited(_) | ModelError::NotReady => {
                    ComputerUseModelError::Transient
                }
                // A step that was all thinking chose no action: the same as one we could not read.
                ModelError::Unparseable
                | ModelError::Unreadable
                | ModelError::OnlyThought { .. } => ComputerUseModelError::Parse,
                // Out of credit, or a sign-in the provider refused: no retry gets past either.
                ModelError::Unauthorized
                | ModelError::PaymentRequired
                | ModelError::SignInRefused
                | ModelError::Refused
                | ModelError::ContextOverflow => ComputerUseModelError::Fatal,
                // A failure this build does not know is never retried.
                #[expect(
                    clippy::match_same_arms,
                    reason = "the catch-all for variants this build does not know shares the body of the known fatal ones only by choice"
                )]
                _ => ComputerUseModelError::Fatal,
            },
        }
    }
}

/// Whether a model can serve a run, asked before the run is accepted so a start the model could
/// not serve is refused with its reason instead of failing out of sight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelReach {
    /// A model can serve (or nothing says it cannot).
    Serves,
    /// No model can serve: the chosen one is not installed or has no engine.
    Missing,
}

/// Whether any model that could serve a run reads images, as far as anything says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelSight {
    /// A model reads images, or nothing says it does not (an unknown counts as seeing).
    MaySee,
    /// Every model that could serve is known not to read images.
    Blind,
}

/// Said beside computer use when [`ModelSight::Blind`].
pub const NO_SIGHT_NOTE: &str = "No model here can see the screen, so the assistant can't use apps for you. Choose one that reads images.";

/// A computer-use model for one run.
pub trait ComputerUseModel: Send {
    /// Whether a model can serve a run now. Only a model that is plainly not there is
    /// [`ModelReach::Missing`]; one that is loading, downloadable or not reachable is left to
    /// the run, which says why if it cannot go on. By default a model serves.
    fn reach(&mut self) -> impl Future<Output = ModelReach> + Send {
        async { ModelReach::Serves }
    }

    /// Whether any model that could serve reads images, asked for the status the shell shows
    /// beside computer use. By default nothing says it does not.
    fn sight(&mut self) -> impl Future<Output = ModelSight> + Send {
        async { ModelSight::MaySee }
    }

    /// The sentence a person reads when the last failure was [`ComputerUseModelError::CannotSeeScreen`];
    /// none when the model has no words of its own for it.
    fn cannot_see_words(&self) -> Option<String> {
        None
    }

    /// Starts the run's session with the goal and hints.
    fn begin(
        &mut self,
        begin: ComputerUseBegin,
    ) -> impl Future<Output = Result<(), ComputerUseModelError>> + Send;

    /// One step: the request, the frame beside it, and where to send events as they stream.
    fn step(
        &mut self,
        request: ComputerUseStepRequest,
        frame: FrameAttachment,
        sink: &mut impl StepSink,
    ) -> impl Future<Output = Result<ComputerUseStepReply, ComputerUseModelError>> + Send;

    /// Tokens used by the last step.
    fn usage(&self) -> TurnUsage;

    /// Which account and model answered, once inferd has routed the session.
    fn served(&self) -> Option<ServedBy>;

    /// Asks the quarantined reader whether `text`, read off the screen, is aimed at an assistant.
    /// The reader reads untrusted text under a closed task, has no tools and answers one of
    /// three words; nothing it says becomes text in the run. By default the model has no reader
    /// to ask, and the answer is [`ScreenRead::Unavailable`]: the deterministic check stands
    /// alone.
    fn classify_screen(&mut self, text: &ScreenText) -> impl Future<Output = ScreenRead> + Send {
        let _ = text;
        async { ScreenRead::Unavailable }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inference_core::StopReason;

    #[test]
    fn inferd_failures_sort_into_transient_parse_and_fatal() {
        const CASES: &[(&str, ComputerUseStepFailure, ComputerUseModelError)] = &[
            (
                "unparseable",
                ComputerUseStepFailure::Unparseable,
                ComputerUseModelError::Parse,
            ),
            (
                "unreachable",
                ComputerUseStepFailure::ModelFailed(ModelError::Unreachable),
                ComputerUseModelError::Transient,
            ),
            (
                "rate limited",
                ComputerUseStepFailure::ModelFailed(ModelError::RateLimited(5)),
                ComputerUseModelError::Transient,
            ),
            (
                "not ready",
                ComputerUseStepFailure::ModelFailed(ModelError::NotReady),
                ComputerUseModelError::Transient,
            ),
            (
                "model unparseable",
                ComputerUseStepFailure::ModelFailed(ModelError::Unparseable),
                ComputerUseModelError::Parse,
            ),
            (
                "unreadable",
                ComputerUseStepFailure::ModelFailed(ModelError::Unreadable),
                ComputerUseModelError::Parse,
            ),
            (
                "only thought",
                ComputerUseStepFailure::ModelFailed(ModelError::OnlyThought {
                    stop: StopReason::EndTurn,
                    thought_len: 12,
                }),
                ComputerUseModelError::Parse,
            ),
            (
                "unauthorized",
                ComputerUseStepFailure::ModelFailed(ModelError::Unauthorized),
                ComputerUseModelError::Fatal,
            ),
            (
                "refused",
                ComputerUseStepFailure::ModelFailed(ModelError::Refused),
                ComputerUseModelError::Fatal,
            ),
            (
                "overflow",
                ComputerUseStepFailure::ModelFailed(ModelError::ContextOverflow),
                ComputerUseModelError::Fatal,
            ),
        ];
        for (name, failure, want) in CASES {
            assert_eq!(ComputerUseModelError::from(*failure), *want, "{name}");
        }
    }

    #[test]
    fn thought_text_never_prints_the_thought() {
        let shown = format!(
            "{:?}",
            StepEvent::ThoughtDelta(ThoughtText("my secret plan".into()))
        );
        assert!(!shown.contains("secret"), "{shown}");
        assert!(shown.contains("<14 bytes>"), "{shown}");
    }
}
