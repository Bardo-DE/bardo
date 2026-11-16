//! The companion's own members of `Intents1.Session`: what it reads from memory through the
//! router, the reader it asks, the episode it hands over, and the task policy it holds. They sit
//! apart from the rest of the caller side because only the companion's role may make them.

use crate::intents::{ClientError, Intents};
use crate::transport::Transport;
use action_core::{HandleCard, Reveal, UserTurn};
use action_vocab::Value;
use action_wire::{
    IntentsReply, IntentsRequest, NoteAsk, ReadAsk, RecallAsk, RecallView, StoredAsk, StoredView,
};
use permission_core::TaskPolicy;
use provenance::SessionId;

impl<T: Transport> Intents<T> {
    /// What memory knows for the session's Space, every untrusted text a handle
    /// (`Session.Recall`, the companion only).
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the request could not be made or the service's reply was not
    /// the one wanted.
    pub async fn session_recall(
        &self,
        session: SessionId,
        ask: RecallAsk,
    ) -> Result<RecallView, ClientError> {
        self.ask(
            IntentsRequest::SessionRecall { session, ask },
            |r| match r {
                IntentsReply::Recalled(view) => Some(view),
                _ => None,
            },
        )
        .await
    }

    /// The durable log of sessions (`Session.Stored`): the sessions the caller may bring back, or
    /// a page of one session's rows.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the request could not be made or the service's reply was not
    /// the one wanted.
    pub async fn session_stored(&self, ask: StoredAsk) -> Result<StoredView, ClientError> {
        self.ask(IntentsRequest::SessionStored { ask }, |r| match r {
            IntentsReply::Stored(view) => Some(view),
            _ => None,
        })
        .await
    }

    /// Asks the quarantined reader (`Session.Read`, the companion only): a closed-set value
    /// comes back plain, any text in it as a handle.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the request could not be made or the service's reply was not
    /// the one wanted.
    pub async fn session_read(
        &self,
        session: SessionId,
        ask: ReadAsk,
    ) -> Result<Reveal<Value>, ClientError> {
        self.ask(IntentsRequest::SessionRead { session, ask }, |r| match r {
            IntentsReply::Read(answer) => Some(answer),
            _ => None,
        })
        .await
    }

    /// Hands the router an episode to record (`Session.Note`, the companion only): the idle
    /// pass's narrative, or a side conversation's episode.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the request could not be made or the service's reply was not
    /// the one wanted.
    pub async fn session_note(&self, session: SessionId, note: NoteAsk) -> Result<(), ClientError> {
        self.ask(IntentsRequest::SessionNote { session, note }, |r| match r {
            IntentsReply::Done => Some(()),
            _ => None,
        })
        .await
    }

    /// The session's task policy, if the writer made one (`Session.TaskPolicy`).
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the request could not be made or the service's reply was not
    /// the one wanted.
    pub async fn session_task_policy(
        &self,
        session: SessionId,
    ) -> Result<Option<TaskPolicy>, ClientError> {
        self.ask(IntentsRequest::SessionTaskPolicy { session }, |r| match r {
            IntentsReply::TaskPolicy(policy) => Some(policy.map(|p| *p)),
            _ => None,
        })
        .await
    }

    /// Narrows a subagent's task policy from what the person said to it (`Session.Narrow`, the
    /// companion only): never wider than the policy it has.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the request could not be made or the service's reply was not
    /// the one wanted.
    pub async fn session_narrow(
        &self,
        session: SessionId,
        turn: UserTurn,
    ) -> Result<(), ClientError> {
        self.ask(
            IntentsRequest::SessionNarrow { session, turn },
            |r| match r {
                IntentsReply::Done => Some(()),
                _ => None,
            },
        )
        .await
    }

    /// What the session holds by handle: shape, source and size, never the content
    /// (`Session.Handles`, the companion only).
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the request could not be made or the service's reply was not
    /// the one wanted.
    pub async fn session_handles(
        &self,
        session: SessionId,
    ) -> Result<Vec<HandleCard>, ClientError> {
        self.ask(IntentsRequest::SessionHandles { session }, |r| match r {
            IntentsReply::Handles(cards) => Some(cards),
            _ => None,
        })
        .await
    }
}
