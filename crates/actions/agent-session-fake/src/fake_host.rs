//! For tests only: a `SessionHost` over `FakeBackend`s and a shared `MemoryLog`.
//!
//! It writes the log the way a real host does (opening, turn, call, step, close, each before it
//! answers) and hands each session the next scripted backend. Fork is not scripted.

use crate::fake::{FakeBackend, MemoryLog};
use action_core::{ConfirmId, UserTurn};
use agent_session::ResumePlan;
use agent_session::resume_plan;
use agent_session::{
    AgentHostError, BackendEvent, CallEvent, SessionBackend, SessionHost, SheetChoice, StartSession,
};
use agent_session::{EndCause, Opening, Seq, SessionEntry};
use agent_session::{SessionExport, export};
use agent_session::{SessionLog, read_all};
use provenance::SessionId;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

/// A host whose sessions are scripted: one list of per-turn event lists for each session it opens
/// or resumes, in that order.
#[derive(Debug)]
pub struct FakeHost {
    log: Arc<MemoryLog>,
    scripts: VecDeque<Vec<Vec<BackendEvent>>>,
    backends: BTreeMap<SessionId, FakeBackend>,
    next_seq: BTreeMap<SessionId, Seq>,
    closed: BTreeSet<SessionId>,
    minted: u32,
    /// The sessions it was asked to cancel, in order.
    pub cancelled: Vec<SessionId>,
    /// The choices it was given on sheets, in order.
    pub sheets: Vec<(ConfirmId, SheetChoice)>,
    /// The turns it was given, by session, in order.
    pub turns: Vec<(SessionId, UserTurn)>,
}

impl FakeHost {
    /// A host writing to `log`; each session it opens takes the next script.
    pub fn new(log: Arc<MemoryLog>, scripts: Vec<Vec<Vec<BackendEvent>>>) -> Self {
        Self {
            log,
            scripts: scripts.into(),
            backends: BTreeMap::new(),
            next_seq: BTreeMap::new(),
            closed: BTreeSet::new(),
            minted: 0,
            cancelled: Vec::new(),
            sheets: Vec::new(),
            turns: Vec::new(),
        }
    }

    /// The same host numbering the sessions it opens from `n`, so two hosts over one log do not
    /// mint the same id.
    #[must_use]
    pub fn numbered_from(mut self, n: u32) -> Self {
        self.minted = n.saturating_sub(1);
        self
    }

    async fn append(
        &mut self,
        session: &SessionId,
        entry: SessionEntry,
    ) -> Result<(), AgentHostError> {
        let seq = match self.next_seq.get(session) {
            Some(seq) => *seq,
            None => Seq(read_all(&*self.log, session).await?.len() as u64),
        };
        self.log.append(session, seq, &entry).await?;
        self.next_seq.insert(session.clone(), seq.next());
        Ok(())
    }

    fn backend(&mut self, session: &SessionId) -> Result<&mut FakeBackend, AgentHostError> {
        self.backends
            .get_mut(session)
            .ok_or(AgentHostError::NoSuchSession)
    }
}

#[expect(
    clippy::unused_async_trait_impl,
    reason = "the trait method is async and this fake never waits"
)]
impl SessionHost for FakeHost {
    async fn open(&mut self, opening: Opening) -> Result<SessionId, AgentHostError> {
        self.minted += 1;
        let session = SessionId::parse(&format!("s-{}", self.minted))
            .expect("`s-` and a number is a session id");
        self.append(&session, SessionEntry::Opened(opening.clone()))
            .await?;
        let mut backend = FakeBackend::new(self.scripts.pop_front().unwrap_or_default());
        backend
            .start(StartSession {
                session: session.clone(),
                opening,
            })
            .await?;
        self.backends.insert(session.clone(), backend);
        Ok(session)
    }

    async fn resume(&mut self, session: &SessionId) -> Result<ResumePlan, AgentHostError> {
        let rows = read_all(&*self.log, session).await?;
        let plan = resume_plan(&rows)?;
        if !self.backends.contains_key(session) {
            let script = self.scripts.pop_front().unwrap_or_default();
            self.backends
                .insert(session.clone(), FakeBackend::new(script));
        }
        self.backend(session)?.resume(&plan).await?;
        Ok(plan)
    }

    async fn turn(&mut self, session: &SessionId, turn: UserTurn) -> Result<(), AgentHostError> {
        if self.closed.contains(session) {
            return Err(AgentHostError::NotOpen);
        }
        self.backend(session)?;
        self.append(session, SessionEntry::Turn(turn.clone()))
            .await?;
        self.turns.push((session.clone(), turn.clone()));
        self.backend(session)?.turn(turn).await?;
        Ok(())
    }

    async fn next_event(
        &mut self,
        session: &SessionId,
    ) -> Result<Option<BackendEvent>, AgentHostError> {
        let event = self.backend(session)?.next_event().await;
        match &event {
            Some(BackendEvent::Call(CallEvent::Started(open))) => {
                self.append(session, SessionEntry::Call(open.clone()))
                    .await?;
            }
            Some(BackendEvent::Call(CallEvent::Ended(step))) => {
                self.append(session, SessionEntry::Step(step.clone()))
                    .await?;
            }
            _ => {}
        }
        Ok(event)
    }

    async fn answer_sheet(
        &mut self,
        session: &SessionId,
        id: &ConfirmId,
        choice: SheetChoice,
    ) -> Result<(), AgentHostError> {
        self.backend(session)?;
        self.sheets.push((id.clone(), choice));
        Ok(())
    }

    async fn cancel(&mut self, session: &SessionId) -> Result<(), AgentHostError> {
        self.backend(session)?.cancel().await;
        self.cancelled.push(session.clone());
        Ok(())
    }

    async fn close(&mut self, session: &SessionId, cause: EndCause) -> Result<(), AgentHostError> {
        self.append(session, SessionEntry::Closed(cause)).await?;
        self.backend(session)?.close().await;
        self.closed.insert(session.clone());
        Ok(())
    }

    async fn fork(&mut self, _session: &SessionId, _at: Seq) -> Result<SessionId, AgentHostError> {
        Err(AgentHostError::NoSuchSession)
    }

    async fn export(&self, session: &SessionId) -> Result<SessionExport, AgentHostError> {
        let rows = read_all(&*self.log, session).await?;
        Ok(export(session, &rows)?)
    }
}
