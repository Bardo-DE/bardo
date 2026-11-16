//! Stoker's answers as porter's: `TurnEvent` to `InferEvent`, a turn's end to a `ChatReply`,
//! `ProviderError` to `ModelError`, `StopReason` both ways. The only place the two vocabularies
//! meet on the way out.

use base_ids::Tokens;
use inference_core as pi;
use model_provider as sp;

use crate::scores::{NoScores, option_scores};

/// What a turn has produced so far, folded into the reply it ends with.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Gathered {
    text: String,
    thought: String,
    calls: Vec<pi::ToolCallPart>,
    /// The options whose shares the turn asked for; none when it did not ask.
    options: Option<Vec<String>>,
}

/// The event a stoker event is, if the client sees it: deltas, finished tool calls and usage
/// (usage is metering that rides on `Finished`, so the client sees it only there).
pub fn event(event: &sp::TurnEvent) -> Option<pi::InferEvent> {
    match event {
        sp::TurnEvent::TextDelta(text) => Some(pi::InferEvent::TextDelta(text.clone())),
        sp::TurnEvent::ThoughtDelta(text) => Some(pi::InferEvent::ThoughtDelta(text.clone())),
        sp::TurnEvent::ToolCallDone(call) => Some(pi::InferEvent::ToolCall(call_part(call))),
        sp::TurnEvent::ThoughtSealed(_)
        | sp::TurnEvent::ToolCallStarted { .. }
        | sp::TurnEvent::ToolCallDelta { .. }
        | sp::TurnEvent::Safety(_)
        | sp::TurnEvent::Usage(_) => None,
    }
}

/// A finished call as the wire has it: the same checked id, name and arguments.
fn call_part(call: &sp::ToolCall) -> pi::ToolCallPart {
    pi::ToolCallPart {
        id: call.id.clone(),
        name: call.name.clone(),
        args: call.input.clone(),
    }
}

impl Gathered {
    /// Takes one event into the reply being built.
    pub fn take(&mut self, event: &sp::TurnEvent) {
        match event {
            sp::TurnEvent::TextDelta(text) => self.text.push_str(text),
            sp::TurnEvent::ThoughtDelta(text) => self.thought.push_str(text),
            sp::TurnEvent::ToolCallDone(call) => self.calls.push(call_part(call)),
            _ => {}
        }
    }

    /// What gathers the reply to `turn`: the same as `default()`, and it remembers the options
    /// of a `Choice` whose shares the turn asked for, so the reply can carry them.
    pub fn for_turn(turn: &sp::TurnRequest) -> Self {
        Self {
            options: crate::scores::asked_options(turn),
            ..Self::default()
        }
    }

    /// The reply the turn ended with. A turn that asked for option shares gets them when the
    /// engine's first-token probabilities allow, and `scores: None` otherwise, never an error.
    pub fn chat_reply(self, end: &sp::TurnEnd, served: pi::ServedBy) -> pi::ChatReply {
        self.chat_reply_noted(end, served).0
    }

    /// As [`Gathered::chat_reply`], and why there are no scores when the turn asked for them
    /// (for the caller to note; the reply is the same reply either way).
    pub fn chat_reply_noted(
        self,
        end: &sp::TurnEnd,
        served: pi::ServedBy,
    ) -> (pi::ChatReply, Option<NoScores>) {
        let (scores, why) = match &self.options {
            None => (None, None),
            Some(options) => match option_scores(options, end.first_token.as_ref()) {
                Ok(scores) => (Some(scores), None),
                Err(why) => (None, Some(why)),
            },
        };
        let mut reply = pi::ChatReply::new(self.text, end.stop, usage(end.usage), served)
            .with_tool_calls(self.calls);
        if !self.thought.is_empty() {
            reply = reply.with_thought(self.thought);
        }
        if let Some(scores) = scores {
            reply = reply.with_scores(scores);
        }
        (reply, why)
    }
}

/// What a turn used, in the units the wire has, with the charge the provider reported for it
/// (none when it reported none).
pub fn usage(usage: sp::TurnUsage) -> pi::TokenUsage {
    pi::TokenUsage {
        input: Tokens(usage.input.0),
        output: Tokens(usage.output.0),
        cached: Tokens(usage.cached.0),
        charge: usage.charge,
    }
}

/// What the client is told when the engine fails. A gateway 5xx is a transient reach failure
/// (ARCHITECTURE section 2); a timeout has no variant of its own and is the same; a request the
/// engine rejected is told as a refusal (the model could not be asked), which the app cannot
/// fix by retrying.
pub fn model_error(error: &sp::ProviderError) -> pi::ModelError {
    match error {
        sp::ProviderError::Unreachable
        | sp::ProviderError::Timeout
        | sp::ProviderError::Server(_) => pi::ModelError::Unreachable,
        sp::ProviderError::NotReady => pi::ModelError::NotReady,
        sp::ProviderError::RateLimited(seconds) => pi::ModelError::RateLimited(seconds.0),
        sp::ProviderError::Unauthorized => pi::ModelError::Unauthorized,
        sp::ProviderError::PaymentRequired(_) => pi::ModelError::PaymentRequired,
        // A 401 with a message is still the key being refused: the person signs in again, as for a
        // bare 401. Only a refusal of an accepted key (a 403: the account or organisation is
        // turned off) is the company refusing the sign-in.
        sp::ProviderError::AuthRejected(detail) if detail.status.0 == 401 => {
            pi::ModelError::Unauthorized
        }
        sp::ProviderError::AuthRejected(_) => pi::ModelError::SignInRefused,
        sp::ProviderError::ContextOverflow { .. } => pi::ModelError::ContextOverflow,
        sp::ProviderError::BadRequest(_) | sp::ProviderError::Refused(_) => pi::ModelError::Refused,
        sp::ProviderError::Unreadable(_) => pi::ModelError::Unreadable,
    }
}

#[cfg(test)]
mod tests;
