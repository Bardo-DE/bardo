//! Sending and receiving messages, the pure half: who receives a message, and the reply to a
//! refused send.

use action_context::session::SessionState;
use action_context::state::{RouterState, SessionRecord};
use action_core::SendRefusal;
use action_wire::{IntentsReply, WireRefusal};
use provenance::{Address, AgentRef, SessionId};

/// The sessions that receive a message to `to`: open ones of that agent in that Space.
pub fn receiving<'a>(
    st: &'a RouterState,
    to: &Address,
) -> impl Iterator<Item = (&'a SessionId, &'a SessionRecord)> {
    let to = to.clone();
    st.sessions.iter().filter(move |(_, r)| {
        r.space == to.space
            && !matches!(r.state, SessionState::Closed(_))
            && AgentRef::of(&r.actor) == Some(to.agent.clone())
    })
}

/// The reply to a send that was refused.
#[must_use]
pub fn send_refusal(why: SendRefusal) -> IntentsReply {
    IntentsReply::Refused(WireRefusal::Send(why))
}
