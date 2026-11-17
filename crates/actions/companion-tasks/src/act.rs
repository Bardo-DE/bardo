//! A card the person pressed: `Companion1.Answer.Act`. The card's `CallRequest` goes to the
//! router as a watched `Run.Perform` in the answer's session, like any call of the task, so it is
//! gated, confirmed and audited the same way. The answer follows it: the card becomes a plan
//! step, `Running` once dispatched, `NeedsYou(Confirm)` while the sheet is up, then
//! `Done { undo }` or `Failed`. Nothing here decides what is allowed.

use crate::deadline::timed;
use crate::fault::ServeError;
use crate::plan::phase_of;
use crate::runtime::Companion;
use crate::seams::Surface;
use action_client::{
    ClientError, PerformEvent, PerformWatch, Transport as IntentsTransport, TransportError,
};
use action_vocab::{CallId, CallRequest, CardActionId, Clock};
use agent_planner::Planner;
use companion_wire::AnswerPhase;
use provenance::{Effect, TaskId};

/// A card's call that has begun: the request is in flight and its events are still to be read.
pub struct Acting {
    task: TaskId,
    id: CallId,
    call: CallRequest,
    effect: Effect,
    watch: PerformWatch,
    patience: std::time::Duration,
}

impl std::fmt::Debug for Acting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Acting({}, {:?})", self.task, self.id)
    }
}

impl Acting {
    /// Where the request lives, when the router has an object for it (a bus); the router in
    /// process has none.
    #[must_use]
    pub fn request(&self) -> Option<&str> {
        self.watch.request()
    }

    /// The call's number in its task.
    #[must_use]
    pub fn id(&self) -> CallId {
        self.id
    }

    /// How long the router may be silent on the call (a sheet may be up) before it is given up on.
    #[must_use]
    pub fn patience(&self) -> std::time::Duration {
        self.patience
    }

    /// The next thing the router says of the call.
    ///
    /// # Errors
    ///
    /// Returns a [`ClientError`] when the router stops answering or refuses the call.
    pub async fn event(&mut self) -> Result<PerformEvent, ClientError> {
        self.watch.next().await
    }
}

/// The card named by the text `Act` was given: the id itself or its JSON string.
///
/// # Errors
///
/// Returns [`ServeError::NoSuchCard`] when `text` names no card.
pub fn card_id(text: &str) -> Result<CardActionId, ServeError> {
    // Text that is neither form names no card: a lookup that missed, not a failure with a cause.
    serde_json::from_str::<CardActionId>(text)
        .ok()
        .or_else(|| CardActionId::parse(text).ok())
        .ok_or(ServeError::NoSuchCard)
}

impl<P: Planner, I: IntentsTransport, K: Clock, S: Surface> Companion<P, I, K, S> {
    /// Starts the card's call and shows it as a step. The answer must offer the card.
    ///
    /// # Errors
    ///
    /// Returns [`ServeError::NoSuchCard`] when the answer offers no card by that id, or a
    /// [`ServeError`] from the router.
    pub async fn begin_act(
        &mut self,
        task: &TaskId,
        card: &CardActionId,
    ) -> Result<Acting, ServeError> {
        let rt = self.runtimes.get(task).ok_or(ServeError::UnknownSession)?;
        let card = rt.card(card).cloned().ok_or(ServeError::NoSuchCard)?;
        let (session, window, id) = (rt.session.clone(), rt.window.clone(), rt.next_call);
        let started = timed(
            &self.clock,
            self.waits.prompt,
            self.intents
                .perform_watched(card.call.clone(), Some(session), window),
        )
        .await;
        let watch = match started {
            Ok(watch) => watch.map_err(ServeError::Router)?,
            Err(_) => {
                return Err(ServeError::Router(ClientError::Transport(
                    TransportError::Bus("the router did not answer in time".to_owned()),
                )));
            }
        };
        if let Some(rt) = self.runtimes.get_mut(task) {
            rt.acts.begin(
                CallId(id),
                card.call.action.clone(),
                card.label,
                card.effect,
            );
            rt.phase = AnswerPhase::Streaming;
        }
        self.publish_answer(task);
        Ok(Acting {
            task: task.clone(),
            id: CallId(id),
            call: card.call,
            effect: card.effect,
            watch,
            patience: self.waits.person,
        })
    }

    /// A draft or a proposal is the answer now: its cards may be acted on.
    ///
    /// # Errors
    ///
    /// Returns a [`ServeError`] when the task is unknown or finished.
    pub fn propose(
        &mut self,
        task: &TaskId,
        body: companion_wire::AnswerBody,
    ) -> Result<(), ServeError> {
        let rt = self
            .runtimes
            .get_mut(task)
            .ok_or(ServeError::UnknownSession)?;
        rt.proposal = Some(body);
        rt.acts = crate::plan::Plan::default();
        rt.phase = AnswerPhase::Done;
        self.publish_answer(task);
        Ok(())
    }

    /// The router says how far a card's call is.
    pub fn act_progress(&mut self, acting: &Acting, progress: &action_core::CallProgress) {
        if let Some(rt) = self.runtimes.get_mut(&acting.task) {
            rt.acts.progress(acting.id, progress);
            rt.phase = phase_of(progress);
        }
        self.publish_answer(&acting.task);
    }

    /// A card's call ended.
    pub fn act_end(
        &mut self,
        acting: &Acting,
        result: Result<action_core::Outcome, action_core::CallRefusal>,
    ) {
        if let Some(rt) = self.runtimes.get_mut(&acting.task) {
            let end = rt.record_call(&acting.call, acting.effect, result);
            rt.acts.end(acting.id, &end);
            rt.phase = AnswerPhase::Done;
        }
        self.publish_answer(&acting.task);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pressed() -> (crate::task::TaskRuntime, TaskId) {
        use action_vocab::{ActionRef, LabelText};
        let mut rt = crate::task::TaskRuntime::new(
            provenance::SessionId::parse("s-1").expect("session"),
            provenance::SpaceId::desktop(),
            provenance::AgentRef::Companion,
            None,
            provenance::UnixSeconds(0),
        );
        rt.proposal = Some(companion_wire::AnswerBody::Text { lines: vec![] });
        rt.acts.begin(
            CallId(0),
            ActionRef {
                app: account_core::AppName::parse("org.bardo.Mail").expect("app"),
                name: provenance::ActionName::parse("mail.message.send").expect("name"),
            },
            LabelText::parse("Send").expect("label"),
            Effect::Outbound,
        );
        (rt, TaskId::parse("t-1").expect("task"))
    }

    #[test]
    fn while_the_sheet_is_up_the_pressed_card_shows_as_a_pending_step_and_needs_you() {
        use companion_wire::{AnswerBody, NeedsYou, StepWireState};
        let (mut rt, task) = pressed();
        let sheet = action_core::ConfirmId::parse("c-1").expect("id");
        rt.phase = AnswerPhase::NeedsYou(NeedsYou::Confirm(sheet));
        let answer = rt.answer(&task);
        assert!(matches!(answer.phase, AnswerPhase::NeedsYou(_)));
        let AnswerBody::Plan(plan) = answer.body else {
            panic!("the step, not the proposal")
        };
        assert_eq!(plan.steps[0].state, StepWireState::Pending);
    }

    #[test]
    fn a_failed_step_keeps_the_answer_done_and_shows_the_step() {
        use companion_wire::{AnswerBody, StepWireState};
        let (mut rt, task) = pressed();
        let end = rt.record_call(
            &action_vocab::CallRequest {
                action: action_vocab::ActionRef {
                    app: account_core::AppName::parse("org.bardo.Mail").expect("app"),
                    name: provenance::ActionName::parse("mail.message.send").expect("name"),
                },
                target: action_vocab::TargetValue::Nothing,
                args: action_vocab::Args::new(),
                origin: action_vocab::Origin::Companion,
            },
            Effect::Outbound,
            Err(action_core::CallRefusal::Timeout),
        );
        rt.acts.end(CallId(0), &end);
        rt.phase = AnswerPhase::Done;
        let AnswerBody::Plan(plan) = rt.answer(&task).body else {
            panic!("a failed card shows its step, not a refusal")
        };
        assert!(matches!(plan.steps[0].state, StepWireState::Failed(_)));
    }

    #[test]
    fn a_card_is_named_by_its_id_or_the_json_of_it() {
        assert_eq!(card_id("send").expect("bare").as_str(), "send");
        assert_eq!(card_id("\"send\"").expect("json").as_str(), "send");
        assert_eq!(card_id(""), Err(ServeError::NoSuchCard));
    }
}
