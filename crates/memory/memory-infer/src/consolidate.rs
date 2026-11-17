//! `InferdConsolidator`: a consolidation draft from an inferd task session.

use crate::deadline::{Deadlines, turn, within};
use account_capability::LlmFeature;
use account_capability::Tier;
use account_core::consent::Usage;
use account_match::LlmNeed;
use account_match::Need;
use base_ids::Tokens;
use inference_client::Transport;
use inference_core::{InferRefusal, InferReply, InferRequest, ModelError, Task, TaskRequest};
use memory_store::class_of;
use memory_tidy::{
    ConsolidateError, ConsolidationInput, Consolidator, Draft, parse_draft, render_prompt,
};
use std::collections::BTreeSet;
use std::sync::Arc;

/// The context window the prompt may need: the active facts and the events of a day.
const CONTEXT: Tokens = Tokens(16_384);

/// Drafts a consolidation through an inferd session (`Need::Llm`, `Task::Extract`,
/// `Usage::Background`).
///
/// The input is rendered as the prompt (`render_prompt`), the answer parsed into hunks
/// (`parse_draft`, `Unparseable` on anything else), and the request carries the most sensitive data
/// class among the labels of what it reads (`class_of`), so mail-derived facts stay on the device
/// unless the person's grant says otherwise.
#[derive(Debug)]
pub struct InferdConsolidator<T: Transport> {
    transport: Arc<T>,
    deadlines: Deadlines,
}

impl<T: Transport> InferdConsolidator<T> {
    /// Drafts over `transport`.
    pub fn new(transport: Arc<T>) -> Self {
        Self {
            transport,
            deadlines: Deadlines::DRAFT,
        }
    }

    /// The same consolidator with these deadlines instead of [`Deadlines::DRAFT`].
    #[must_use]
    pub fn with_deadlines(mut self, deadlines: Deadlines) -> Self {
        self.deadlines = deadlines;
        self
    }
}

fn need() -> Need {
    Need::Llm(LlmNeed::new(BTreeSet::from([LlmFeature::Chat]), CONTEXT))
}

#[expect(
    clippy::match_same_arms,
    reason = "the wildcard stands for reasons the accounts service adds later (the enum is non-exhaustive); the named arms are the known ones"
)]
fn refusal(why: InferRefusal) -> ConsolidateError {
    match why {
        InferRefusal::Unavailable
        | InferRefusal::RequiresCloud(_)
        | InferRefusal::NeedsGrant
        | InferRefusal::Denied
        | InferRefusal::Unsupported => ConsolidateError::Unavailable,
        InferRefusal::OverBudget => ConsolidateError::Busy,
        // The accounts service may add reasons (`InferRefusal` is non-exhaustive); one not named means no run.
        _ => ConsolidateError::Unavailable,
    }
}

#[expect(
    clippy::match_same_arms,
    reason = "the wildcard stands for failures the accounts service adds later (the enum is non-exhaustive); the named arms are the known ones"
)]
fn failure(why: ModelError) -> ConsolidateError {
    match why {
        ModelError::RateLimited(_) => ConsolidateError::Busy,
        // A reply that was all thinking left no draft to read.
        ModelError::Unreadable
        | ModelError::Unparseable
        | ModelError::ContextOverflow
        | ModelError::OnlyThought { .. } => ConsolidateError::Unparseable,
        // Out of credit or a refused sign-in: nothing this daemon can retry its way past.
        ModelError::Unreachable
        | ModelError::Unauthorized
        | ModelError::PaymentRequired
        | ModelError::SignInRefused
        | ModelError::Refused
        | ModelError::NotReady => ConsolidateError::Unavailable,
        // The accounts service may add failures (`ModelError` is non-exhaustive); one not named left no draft.
        _ => ConsolidateError::Unparseable,
    }
}

impl<T: Transport> Consolidator for InferdConsolidator<T> {
    async fn draft(&self, input: ConsolidationInput) -> Result<Draft, ConsolidateError> {
        let labels = input
            .facts
            .iter()
            .map(|f| &f.label)
            .chain(input.events.iter().map(|e| &e.label));
        let class = class_of(labels);
        let mut session = within(
            self.deadlines.open,
            self.transport.open(&need(), class, Tier::Balanced),
        )
        .await
        .map_err(ConsolidateError::unreachable)?
        .map_err(ConsolidateError::unreachable)?;
        let request = InferRequest::Task(TaskRequest::new(
            Task::Extract,
            render_prompt(&input),
            class,
            Usage::Background,
        ));
        match turn(&mut session, request, self.deadlines.turn)
            .await
            .map_err(ConsolidateError::unreachable)?
        {
            InferReply::Chat(reply) => parse_draft(&reply.text, &input),
            InferReply::Refused(why) => Err(refusal(why)),
            InferReply::Failed(why) => Err(failure(why)),
            InferReply::Cancelled => Err(ConsolidateError::Busy),
            InferReply::Embed(_)
            | InferReply::ComputerUseStep(_)
            | InferReply::Transcribed(_)
            | InferReply::Spoke(_) => Err(ConsolidateError::Unparseable),
        }
    }
}
