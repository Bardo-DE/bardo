//! The computer-use model over inferd.

use crate::frame_fd::frame_fd;
use account_capability::ComputerUseEnv;
use account_capability::Tier;
use account_core::DataClass;
use account_match::ComputerUseNeed;
use account_match::Need;
use base_ids::Tokens;
use computer_use_run::{
    ComputerUseModel, ComputerUseModelError, FrameAttachment, ModelReach, ModelSight, StepEvent,
    StepSink, ThoughtText,
};
use inference_client::{OpenOptions, Transport, TransportError};
use inference_core::{
    AttachIndex, ClientFrame, CuaBegin, CuaStepFailure, CuaStepReply, CuaStepRequest, Declined,
    DeclinedBecause, FrameLayout, ImageSource, InferEvent, InferRefusal, InferReply, InferRequest,
    InferSession, NO_MODEL_SEES_SCREEN, PlaceRow, PlaceState, Readiness, ServedBy, SessionError,
    Sight, TurnUsage, cannot_see_screen,
};
use std::collections::BTreeSet;

/// A [`ComputerUseModel`] over one inferd session (`Need::ComputerUse`, class `Screen`).
///
/// It sends `ComputerUseBegin` once, then a `ComputerUseStep` per turn with the frame as a memfd attached to the
/// session's fd, never base64 on the bus. The route is chosen once, so the session is pinned to one
/// model.
///
/// `begin` reads to the empty `ComputerUseStepReply` that acknowledges it (the protocol has no other
/// reply for a begin; one with actions in it is a protocol fault). `step` forwards each
/// `ThoughtDelta` and `ActionProposed` to the sink as it arrives and ends on `Finished`. A model
/// failure ends the step and leaves the session usable (its history is intact); a session that
/// closes or breaks is dropped, and every later step is `Fatal`: the run cannot continue on
/// another model.
#[derive(Debug)]
pub struct InferdComputerUseModel<T: Transport> {
    transport: T,
    tier: Tier,
    session: Option<T::Session>,
    served: Option<ServedBy>,
    usage: Option<TurnUsage>,
    /// What inferd said about the screen when it refused to serve: the model the person named
    /// cannot read images, or no model can. It is the sentence the person reads.
    blind: Option<String>,
}

impl<T: Transport> InferdComputerUseModel<T> {
    /// A model that has not opened its session yet; `begin` opens it, on the `Balanced` tier.
    pub fn over(transport: T) -> Self {
        Self {
            transport,
            tier: Tier::Balanced,
            session: None,
            served: None,
            usage: None,
            blind: None,
        }
    }

    /// The tier the session is opened on (`ComputerUseDaemonConfig::tier`).
    #[must_use]
    pub fn on_tier(self, tier: Tier) -> Self {
        Self { tier, ..self }
    }
}

/// The need a run of `env` opens its session with.
fn need_for(env: ComputerUseEnv) -> Need {
    Need::ComputerUse(ComputerUseNeed::new(BTreeSet::from([env])))
}

/// An unreachable inferd is worth retrying; a hung-up, confused or refusing one is not (a refused
/// caller stays refused until inferd's caller table names the computer-use daemon).
#[expect(
    clippy::needless_pass_by_value,
    reason = "used as the function of `map_err`, which hands the error over"
)]
fn transport_failure(error: TransportError) -> ComputerUseModelError {
    match error {
        TransportError::Unreachable => ComputerUseModelError::Transient,
        _ => ComputerUseModelError::Fatal,
    }
}

/// Why a turn ended without the reply it was waiting for. `blind` is the sentence inferd's
/// notice that the chosen model cannot read images left: a refusal that follows it is that
/// refusal, not a model that is merely not there.
fn failed_turn(reply: &InferReply, blind: Option<&str>) -> ComputerUseModelError {
    match reply {
        InferReply::Failed(error) => ComputerUseStepFailure::ModelFailed(*error).into(),
        InferReply::Refused(InferRefusal::NoModelSeesScreen) => {
            ComputerUseModelError::CannotSeeScreen
        }
        InferReply::Refused(InferRefusal::Unavailable) if blind.is_some() => {
            ComputerUseModelError::CannotSeeScreen
        }
        InferReply::Refused(InferRefusal::Unavailable) | InferReply::Cancelled => {
            ComputerUseModelError::Transient
        }
        InferReply::Refused(_)
        | InferReply::ComputerUseStep(_)
        | InferReply::Chat(_)
        | InferReply::Embed(_)
        | InferReply::Transcribed(_)
        | InferReply::Spoke(_) => ComputerUseModelError::Fatal,
    }
}

fn zero() -> TurnUsage {
    TurnUsage {
        input: Tokens(0),
        output: Tokens(0),
        cached: Tokens(0),
        charge: None,
        ..TurnUsage::default()
    }
}

impl<T: Transport> ComputerUseModel for InferdComputerUseModel<T> {
    /// Asks inferd (`Prepare`, which also warms the model) whether the engine for a desktop
    /// run can be had. Only `Unavailable` is a no: loading, downloading, an inferd that does
    /// not answer and a refusal are left to the run, which reports why if it cannot go on.
    async fn reach(&mut self) -> ModelReach {
        let need = need_for(ComputerUseEnv::Desktop);
        let asked = self
            .transport
            .prepare(&need, DataClass::Screen, self.tier, &OpenOptions::default())
            .await;
        match asked {
            Ok(Readiness::Unavailable) => ModelReach::Missing,
            _ => ModelReach::Serves,
        }
    }

    async fn begin(&mut self, begin: ComputerUseBegin) -> Result<(), ComputerUseModelError> {
        let need = need_for(begin.env);
        let mut session = self
            .transport
            .open(&need, DataClass::Screen, self.tier)
            .await
            .map_err(transport_failure)?;
        session
            .send(ClientFrame::Request(InferRequest::ComputerUseBegin(begin)))
            .await
            .map_err(|error| {
                tracing::warn!(%error, "could not send the begin frame to the model");
                ComputerUseModelError::Fatal
            })?;
        loop {
            let event = session.next().await.map_err(|error| {
                tracing::warn!(%error, "the model session broke before it began");
                ComputerUseModelError::Fatal
            })?;
            match event {
                InferEvent::Routed(served) => self.served = Some(served),
                InferEvent::Declined(declined) => self.note_declined(&declined),
                InferEvent::Finished(InferReply::ComputerUseStep(ack)) if is_ack(&ack) => break,
                InferEvent::Finished(other) => return Err(self.ended(&other)),
                _ => {}
            }
        }
        self.session = Some(session);
        Ok(())
    }

    async fn step(
        &mut self,
        mut request: ComputerUseStepRequest,
        frame: FrameAttachment,
        sink: &mut impl StepSink,
    ) -> Result<ComputerUseStepReply, ComputerUseModelError> {
        let session = self.session.as_mut().ok_or(ComputerUseModelError::Fatal)?;
        // The bytes ride as the first descriptor, whatever the request says: the layout is the
        // buffer's own.
        request.frame.source = ImageSource::Attached(AttachIndex(0));
        request.frame.layout = FrameLayout::Raw {
            format: frame.0.format,
            size: frame.0.size,
            stride: frame.0.stride,
        };
        let fd = frame_fd(&frame).map_err(|error| {
            tracing::warn!(%error, "could not make a descriptor of the frame");
            ComputerUseModelError::Fatal
        })?;
        self.usage = None;
        let sent = session
            .send_attached(
                ClientFrame::Request(InferRequest::ComputerUseStep(request)),
                vec![fd],
            )
            .await;
        if let Err(error) = sent {
            return Err(self.lost(&error));
        }
        loop {
            let event = match self.session.as_mut() {
                Some(session) => session.next().await,
                None => return Err(ComputerUseModelError::Fatal),
            };
            match event {
                Ok(InferEvent::ThoughtDelta(text)) => {
                    sink.event(StepEvent::ThoughtDelta(ThoughtText(text)));
                }
                Ok(InferEvent::ActionProposed(action)) => {
                    sink.event(StepEvent::ActionProposed(action));
                }
                Ok(InferEvent::Routed(served)) => self.served = Some(served),
                Ok(InferEvent::Declined(declined)) => self.note_declined(&declined),
                Ok(InferEvent::Usage(usage)) => self.usage = Some(usage),
                Ok(InferEvent::Finished(InferReply::ComputerUseStep(reply))) => return Ok(reply),
                Ok(InferEvent::Finished(other)) => return Err(self.ended(&other)),
                Ok(_) => {}
                Err(error) => return Err(self.lost(&error)),
            }
        }
    }

    fn usage(&self) -> TurnUsage {
        self.usage.unwrap_or_else(zero)
    }

    fn served(&self) -> Option<ServedBy> {
        self.served.clone()
    }

    /// Asks for the places (`Places`) and sorts what their models take in: a model that reads
    /// images means computer use can work. A model that says nothing counts as one that may
    /// see, and so does a list that cannot be had: the warning is only for a list whose every
    /// model is known not to.
    async fn sight(&mut self) -> ModelSight {
        match self.transport.places().await {
            Ok(places) => sight_of(&places),
            Err(_) => ModelSight::MaySee,
        }
    }

    fn cannot_see_words(&self) -> Option<String> {
        self.blind.clone()
    }
}

/// Whether any model among `places` may read images. Only a list whose models are all known
/// not to is [`ModelSight::Blind`]; an empty list says nothing.
fn sight_of(places: &[PlaceRow]) -> ModelSight {
    let sights: Vec<Sight> = places
        .iter()
        .filter(|place| place.state == PlaceState::Ready)
        .flat_map(|place| place.models.iter().map(|model| model.takes.sight()))
        .collect();
    if sights.is_empty() || sights.iter().any(|sight| *sight != Sight::Blind) {
        ModelSight::MaySee
    } else {
        ModelSight::Blind
    }
}

impl<T: Transport> InferdComputerUseModel<T> {
    /// A turn ended without its reply: sorted, and the sentence kept when it is about the screen.
    fn ended(&mut self, reply: &InferReply) -> ComputerUseModelError {
        if matches!(reply, InferReply::Refused(InferRefusal::NoModelSeesScreen)) {
            self.blind = Some(NO_MODEL_SEES_SCREEN.to_owned());
        }
        failed_turn(reply, self.blind.as_deref())
    }

    /// inferd named the model the person chose and said why it cannot serve; for one that cannot
    /// read images the sentence is kept, with the model's name.
    fn note_declined(&mut self, declined: &Declined) {
        if declined.because == DeclinedBecause::CannotSeeScreen {
            self.blind = Some(cannot_see_screen(declined.model.model.as_str()));
        }
    }

    /// The session broke: it is gone, and so is the history the model was pinned to.
    fn lost(&mut self, error: &SessionError) -> ComputerUseModelError {
        tracing::warn!(%error, "the model session broke");
        self.session = None;
        ComputerUseModelError::Fatal
    }
}

/// The empty reply that acknowledges a begin.
fn is_ack(reply: &ComputerUseStepReply) -> bool {
    reply.thought.is_none()
        && reply.actions.is_empty()
        && reply.dropped.is_empty()
        && reply.safety.is_empty()
}
