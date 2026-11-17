//! A stoker [`Provider`] whose turns are inferd chat turns.
//!
//! A stoker [`Provider`] whose turns are inferd chat turns: one session per turn, opened as
//! `Need::Llm` (chat, vision, and tools when the turn has some) on data class `Screen`, the
//! request written with [`crate::request::chat`], the events read back into stoker's.
//!
//! inferd decides where the turn goes (the tier's row in its config), fetches the key from
//! accountd for that one turn and meters the reply against `[ai.spend]`. A turn refused on a
//! spend cap is counted in the [`Watch`] so the run can stop.

use crate::request::{RequestFault, SamplingFrom, chat};
use crate::watch::Watch;
use account_capability::LlmFeature;
use account_capability::Tier;
use account_core::DataClass;
use account_core::consent::Usage;
use account_match::LlmNeed;
use account_match::Need;
use base_ids::Tokens;
use inference_client::{OpenOptions, Transport, TransportError};
use inference_core::{
    ClientFrame, InferEvent, InferRefusal, InferReply, InferRequest, InferSession, ModelError,
};
use model_provider as sp;
use model_provider::{Provider, ProviderError, TurnEvent, TurnSink};
use std::collections::BTreeSet;
use std::sync::Arc;

/// A hosted model reached through inferd.
#[derive(Debug)]
pub struct InferdProvider<T> {
    transport: T,
    tier: Tier,
    sampling: SamplingFrom,
    watch: Arc<Watch>,
}

impl<T: Transport> InferdProvider<T> {
    /// A provider that asks inferd over `transport` at `tier`.
    pub fn new(transport: T, tier: Tier, sampling: SamplingFrom, watch: Arc<Watch>) -> Self {
        Self {
            transport,
            tier,
            sampling,
            watch,
        }
    }
}

/// What a chat turn with a screenshot (and tools, when there are some) needs of the model.
fn need(has_tools: bool) -> Need {
    let mut features = BTreeSet::from([LlmFeature::Chat, LlmFeature::Vision]);
    if has_tools {
        features.insert(LlmFeature::Tools);
    }
    Need::Llm(LlmNeed::new(features, Tokens(16_000)))
}

fn transport_fault(error: TransportError) -> ProviderError {
    match error {
        TransportError::Unreachable => ProviderError::Unreachable,
        TransportError::Closed | TransportError::Malformed(_) => {
            ProviderError::Unreadable("inferd closed or garbled the session".to_owned())
        }
        TransportError::Denied(why) => ProviderError::Refused(format!("inferd denied: {why}")),
        other => ProviderError::Unreadable(other.to_string()),
    }
}

fn model_fault(error: ModelError) -> ProviderError {
    match error {
        ModelError::Unreachable => ProviderError::Unreachable,
        ModelError::RateLimited(seconds) => ProviderError::RateLimited(sp::RetrySeconds(seconds)),
        ModelError::Unauthorized => ProviderError::Unauthorized,
        // Retrying cannot help either: the person tops up or picks another model, or the
        // provider has to accept the sign-in. The reason stays in inferd's logs, never here.
        ModelError::PaymentRequired => {
            ProviderError::PaymentRequired(sp::ProviderDetail::new(402, ""))
        }
        ModelError::SignInRefused => ProviderError::AuthRejected(sp::ProviderDetail::new(403, "")),
        ModelError::Refused => ProviderError::Refused(
            "the provider refused the request (OpenRouter answers 402 when max_tokens times the price exceeds the credit)"
                .to_owned(),
        ),
        ModelError::NotReady => ProviderError::NotReady,
        ModelError::ContextOverflow => ProviderError::ContextOverflow {
            limit: base_ids::Tokens(0),
        },
        other => ProviderError::Unreadable(other.to_string()),
    }
}

fn fault(error: &RequestFault) -> ProviderError {
    ProviderError::BadRequest(error.to_string())
}

/// Whether the turn's text and calls already arrived as events.
#[derive(Debug, Default)]
struct Seen {
    text: bool,
    calls: bool,
}

impl<T: Transport> Provider for InferdProvider<T> {
    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the trait method is async; this implementation has nothing to await"
    )]
    async fn describe(&self) -> Result<Vec<sp::ModelInfo>, ProviderError> {
        Ok(Vec::new())
    }

    async fn turn<K: TurnSink>(
        &self,
        request: &sp::TurnRequest,
        sink: &mut K,
    ) -> Result<sp::TurnEnd, ProviderError> {
        let asked = chat(request, self.tier, self.sampling).map_err(|e| fault(&e))?;
        let options = OpenOptions::default().with_usage(Usage::Interactive);
        let mut session = self
            .transport
            .open_with(
                &need(!asked.tools.is_empty()),
                DataClass::Screen,
                self.tier,
                &options,
            )
            .await
            .map_err(transport_fault)?;
        session
            .send(ClientFrame::Request(InferRequest::Chat(asked)))
            .await
            .map_err(|e| ProviderError::Unreadable(e.to_string()))?;
        let mut seen = Seen::default();
        loop {
            let event = session
                .next()
                .await
                .map_err(|e| ProviderError::Unreadable(e.to_string()))?;
            match event {
                InferEvent::TextDelta(text) => {
                    seen.text = true;
                    let _ = sink.event(TurnEvent::TextDelta(text));
                }
                InferEvent::ThoughtDelta(text) => {
                    let _ = sink.event(TurnEvent::ThoughtDelta(text));
                }
                InferEvent::ToolCall(part) => {
                    seen.calls = true;
                    let _ = sink.event(TurnEvent::ToolCallDone(part));
                }
                InferEvent::Finished(reply) => return self.finish(reply, &seen, sink),
                _ => {}
            }
        }
    }
}

impl<T> InferdProvider<T> {
    fn finish<K: TurnSink>(
        &self,
        reply: InferReply,
        seen: &Seen,
        sink: &mut K,
    ) -> Result<sp::TurnEnd, ProviderError> {
        match reply {
            InferReply::Chat(chat) => {
                if !seen.text && !chat.text.is_empty() {
                    let _ = sink.event(TurnEvent::TextDelta(chat.text.clone()));
                }
                if !seen.calls {
                    for part in &chat.tool_calls {
                        let _ = sink.event(TurnEvent::ToolCallDone(part.clone()));
                    }
                }
                self.watch.answered(
                    u64::from(chat.usage.input.0),
                    u64::from(chat.usage.output.0),
                    chat.usage.charge,
                );
                Ok(sp::TurnEnd {
                    stop: chat.stop,
                    usage: chat.usage,
                    served: sp::ModelName(chat.served.model.to_string()),
                    first_token: None,
                })
            }
            InferReply::Refused(InferRefusal::OverBudget) => {
                self.watch.refuse_spend();
                Err(ProviderError::Refused("a spend cap was reached".to_owned()))
            }
            InferReply::Refused(why) => Err(ProviderError::Refused(why.to_string())),
            InferReply::Failed(error) => Err(model_fault(error)),
            InferReply::Cancelled => Err(ProviderError::Unreachable),
            other => Err(ProviderError::Unreadable(format!(
                "an unexpected reply: {other:?}"
            ))),
        }
    }
}
