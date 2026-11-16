//! Computer-use steps as one run on one session: its goal and what it did so far.
//!
//! A step runs one model turn through the pinned engine; the prompt, the parse (with one
//! repair) and the mapping into window space are stoker's `CuaSession`, which `cua_step` drives.
//! The history lives in the session value the run holds, so the run is pinned to one model.

use crate::bridge::Frames;
use crate::cua_step::{self, Failed};
use computer_use_session::CuaSession;
use inference_core::{
    ChatSink, CuaBegin, CuaStepFailure, CuaStepReply, CuaStepRequest, ModelError,
};
use inference_router::local::LocalModel;
use model_provider::{Provider, Tokens};

/// Only `Screen` data may enter a computer-use session.
///
/// The rule moved to `inference_router::router`, which the route and the session machine apply too.
pub use inference_router::router::check_class;

/// One run's state on one session.
///
/// It holds the goal, and the stoker session once the first step has
/// chosen the model's profile (the model is the session's pin, known to the step and not to the
/// `CuaBegin` request).
#[derive(Clone)]
pub struct CuaRun {
    begin: CuaBegin,
    session: Option<CuaSession>,
}

// The goal is what the person asked for: Debug shows sizes only.
impl std::fmt::Debug for CuaRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CuaRun({:?}, {} remembered steps)",
            self.begin,
            self.session.as_ref().map_or(0, CuaSession::remembered)
        )
    }
}

/// Everything one step needs beyond the run: the pinned model, the provider that reaches its
/// engine, the bytes behind the request's `ImageSource::Attached` and the request itself.
#[derive(Debug)]
pub struct StepJob<'a, P> {
    /// The model the session is pinned to.
    pub model: &'a LocalModel,
    /// The engine's provider.
    pub provider: &'a P,
    /// The descriptors that arrived with the request's frame.
    pub frames: &'a Frames,
    /// The step.
    pub request: &'a CuaStepRequest,
    /// The most a reply may write (`LocalModel::reply_ceiling`).
    pub ceiling: Tokens,
}

impl CuaRun {
    /// A run for this goal, with nothing done.
    #[must_use]
    pub fn begin(begin: CuaBegin) -> Self {
        Self {
            begin,
            session: None,
        }
    }

    /// The window-space actions for one step.
    ///
    /// Prepare the frame, one model turn (thoughts as
    /// they stream, then `ActionProposed` for each action, into `sink`; a `Stop` from it ends
    /// the proposals), parse (one more turn when the reply must be repaired), map. A step that
    /// succeeds is remembered in the run, and so is one whose reply never parsed (the model took
    /// it); one that fails otherwise is not, so the same step can be asked again.
    ///
    /// # Errors
    ///
    /// Returns the [`CuaStepFailure`] of the step: the model's failure, or a reply that could not
    /// be parsed or mapped into window space.
    pub async fn step<P: Provider>(
        &mut self,
        job: StepJob<'_, P>,
        sink: &mut impl ChatSink,
    ) -> Result<CuaStepReply, CuaStepFailure> {
        if self.session.is_none() {
            self.session = Some(
                cua_step::open(job.model, &self.begin, job.ceiling)
                    .ok_or(CuaStepFailure::ModelFailed(ModelError::Unreadable))?,
            );
        }
        let session = self
            .session
            .as_mut()
            .ok_or(CuaStepFailure::ModelFailed(ModelError::Unreadable))?;
        let forward = |event| sink.event(event);
        match cua_step::step(
            session,
            job.model,
            job.provider,
            job.request,
            job.frames,
            forward,
        )
        .await
        {
            Ok(reply) => Ok(reply),
            Err(Failed { failure }) => Err(failure),
        }
    }
}

#[cfg(test)]
mod tests;
