//! The order a conversation must be in before it is encoded.
//!
//! Strict servers (vLLM chat templates, Anthropic later) reject or mis-render an orphaned tool
//! result; local templates sometimes fail silently. The check runs in the encode paths and where
//! the planner windows its history.

use std::collections::BTreeSet;

use model_types::{Message, Part, Role, ToolCallId};

/// What is wrong with a conversation's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
pub enum SequenceError {
    /// Two assistant messages in a row.
    #[error("two assistant messages follow each other")]
    ConsecutiveAssistant,
    /// A call is still open when the next non-tool message comes, or when the conversation ends.
    #[error("a tool call has no result before the next turn")]
    UnansweredCall,
    /// A result whose call is not open: unknown, or already answered.
    #[error("a tool result answers no call")]
    OrphanResult,
}

/// `Ok` when no assistant message follows another, every tool call is answered before the next
/// non-tool message (and before the conversation ends), and every tool result answers an earlier.
///
/// Still unanswered call. A tool call and its result are matched by id, so a second result for one
/// call is an orphan.
///
/// # Errors
///
/// Returns [`SequenceError`] when an assistant message follows another, a tool result answers no
/// earlier call, or a tool call is left unanswered.
pub fn check(messages: &[Message]) -> Result<(), SequenceError> {
    let mut pending: BTreeSet<&ToolCallId> = BTreeSet::new();
    let mut previous: Option<Role> = None;
    for message in messages {
        for part in &message.parts {
            if let Part::ToolResult(result) = part
                && !pending.remove(&result.id)
            {
                return Err(SequenceError::OrphanResult);
            }
        }
        if message.role == Role::Assistant && previous == Some(Role::Assistant) {
            return Err(SequenceError::ConsecutiveAssistant);
        }
        if message.role != Role::Tool && !pending.is_empty() {
            return Err(SequenceError::UnansweredCall);
        }
        pending.extend(message.parts.iter().filter_map(|part| match part {
            Part::ToolCall(call) => Some(&call.id),
            _ => None,
        }));
        previous = Some(message.role);
    }
    if pending.is_empty() {
        Ok(())
    } else {
        Err(SequenceError::UnansweredCall)
    }
}
