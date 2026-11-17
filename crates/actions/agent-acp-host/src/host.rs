//! `AgentHost`: the `SessionHost` of one external agent. A sibling of the native host, for the
//! same edges: it opens the agent's session at the router (which writes the session's log, as it
//! does for every session), records the person's turn there (the router derives the task policy
//! from the person's words, as for a launcher), and runs the turn on an `AcpBackend`.
//!
//! **Sheets.** The router asks the person through its confirmer, like any call: the shell's sheet, by
//! default, and the host sees nothing of it. In the development fallback (`true`,
//! `bardo-agent --tty`) the host tells the router that it shows this session's sheets itself,
//! and each one comes out of `next_event` as `BackendEvent::Confirm` and goes back through
//! `answer_sheet`, as an editor's do. Without the flag the host asks for no such route and holds
//! no desk, so a sheet can only go to the desktop.
//!
//! One host runs one agent session at a time; resuming or forking one is not served (ACP v1 has
//! no fork, and a resumed agent session is always a new one, which the router restores itself).

use super::backend::{AcpBackend, Seams};
use super::running::RunningTurn;
use action_core::{ConfirmId, Rewind, SheetSurface, TurnEnd as Ended, UserTurn};
use action_vocab::TurnId;
use agent_acp_core::cause::Step;
use agent_acp_core::court::{Court, OpenAgent};
use agent_session::{
    AgentHostError, BackendError, BackendEvent, BackendKind, EndCause, Opening, ResumePlan,
    SessionBackend, SessionExport, SessionHost, SheetChoice, SheetDesk, StartSession, TurnEnd,
};
use companion_model::{RunsIn, WatchedAgent};
use futures_util::future::{Either, select};
use provenance::SessionId;
use std::pin::{Pin, pin};

type Recording =
    Pin<Box<dyn Future<Output = Result<TurnId, agent_acp_core::court::CourtError>> + Send>>;

/// A turn the router is recording and the agent has not yet been given.
struct Opening2 {
    turn: UserTurn,
    recording: Recording,
}

/// The host of one agent session.
pub struct AgentHost<X: Seams, D: SheetDesk> {
    backend: AcpBackend<X>,
    court: X::Court,
    desk: D,
    /// Whether the person answers the router's sheets on this host, on the terminal (a development
    /// fallback, never the default) rather than on the desktop's sheet through the router's
    /// confirmer.
    terminal_fallback: bool,
    rewind: Rewind,
    helper: Option<provenance::HelperId>,
    session: Option<SessionId>,
    pending: Option<Opening2>,
    /// The turn the agent is working on; the router hears when it ends, whichever way.
    running: Option<RunningTurn<X::Court>>,
}

impl<X: Seams, D: SheetDesk> std::fmt::Debug for AgentHost<X, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AgentHost")
    }
}

impl<X: Seams, D: SheetDesk> AgentHost<X, D> {
    /// A host over `backend` and the `court` it calls the router through (a clone of the
    /// backend's), with `desk` holding the sheets the router puts to this host when `fallback`
    /// asks for them.
    pub fn new(backend: AcpBackend<X>, court: X::Court, desk: D, terminal_fallback: bool) -> Self {
        Self {
            backend,
            court,
            desk,
            terminal_fallback,
            rewind: Rewind::default(),
            helper: None,
            session: None,
            pending: None,
            running: None,
        }
    }

    /// Says who keeps the history of the agent's file changes (its `checkpoints` in
    /// `agents.toml`); the router is told when the session opens. Without it the default over-saves
    /// rather than under-saves.
    #[must_use]
    pub fn with_rewind(mut self, rewind: Rewind) -> Self {
        self.rewind = rewind;
        self
    }

    /// Says that the agent is one helper's: the router keeps its grants apart from every other
    /// helper's, even one that runs the same program.
    #[must_use]
    pub fn for_helper(mut self, helper: provenance::HelperId) -> Self {
        self.helper = Some(helper);
        self
    }

    /// Ends the agent's process after a start that failed (the agent wanted signing in, say): the
    /// session never opened, so [`SessionHost::close`] has nothing to close, but the process is
    /// running and must not be left so.
    pub async fn abandon(&mut self) {
        self.backend.close().await;
    }

    /// Asks the agent to sign out (`logout`), when it advertised the method. `Ok(false)` when it
    /// did not, or would not. The agent clears its own login.
    ///
    /// # Errors
    /// Returns a [`BackendError`] when no agent is running or the connection to it is gone.
    pub async fn log_out(&mut self) -> Result<bool, BackendError> {
        self.backend.log_out().await
    }

    /// Leaves the open session at the router as a watch session that a terminal marks, and ends
    /// the agent's process: the conversation goes on in the app's own terminal program. `agent` is
    /// the app and its own id for the conversation. The router refuses while a turn is running.
    ///
    /// # Errors
    /// Returns a [`AgentHostError`] when `session` is not this host's or the router will not hand the
    /// session to a terminal.
    pub async fn hand_to_terminal(
        &mut self,
        session: &SessionId,
        agent: WatchedAgent,
    ) -> Result<(), AgentHostError> {
        self.mine(session)?;
        self.court
            .hand(session, RunsIn::Terminal, Some(agent))
            .await
            .map_err(|error| {
                tracing::debug!(%error, "the router would not hand the session over");
                BackendError::Unavailable
            })?;
        self.pending = None;
        self.running = None;
        self.backend.close().await;
        self.session = None;
        Ok(())
    }

    /// Takes a session back that the router holds as a watch session, and starts the agent on it
    /// (`opening` says where; the backend was made to resume the agent's own session). Answers
    /// the app and its own id for the conversation. A start that fails leaves the session a watch
    /// session, as it was.
    ///
    /// # Errors
    /// Returns a [`AgentHostError`] when this host already has a session, the router does not hold
    /// `session` as a watch session, or the agent fails to start on it.
    pub async fn take_back(
        &mut self,
        session: SessionId,
        opening: Opening,
    ) -> Result<WatchedAgent, AgentHostError> {
        if self.session.is_some() {
            return Err(BackendError::Busy.into());
        }
        let agent = self
            .court
            .hand(&session, RunsIn::Desktop, None)
            .await
            .ok()
            .flatten()
            .ok_or(BackendError::Unavailable)?;
        let started = self
            .backend
            .start(StartSession {
                session: session.clone(),
                opening,
            })
            .await;
        if let Err(fault) = started {
            self.backend.close().await;
            let _ = self
                .court
                .hand(&session, RunsIn::Terminal, Some(agent))
                .await;
            return Err(fault.into());
        }
        self.session = Some(session);
        Ok(agent)
    }

    /// The backend, for what only it knows (the taint's cause).
    pub fn backend(&self) -> &AcpBackend<X> {
        &self.backend
    }

    fn mine(&self, session: &SessionId) -> Result<(), AgentHostError> {
        (self.session.as_ref() == Some(session))
            .then_some(())
            .ok_or(AgentHostError::NoSuchSession)
    }

    fn is_terminal(&self) -> bool {
        self.terminal_fallback
    }
}

/// Tells the router the turn ended, if `event` is the end of one (or the backend has nothing
/// more to say, which ends it too). Any other event leaves the turn running.
async fn settle<C: Court>(running: &mut Option<RunningTurn<C>>, event: Option<&BackendEvent>) {
    let how = match event {
        Some(BackendEvent::TurnEnd(end)) => Ended::from(*end),
        None => Ended::Failed,
        Some(_) => return,
    };
    if let Some(turn) = running.take() {
        turn.end(how).await;
    }
}

fn program_of(opening: &Opening) -> Option<permission_core::ProgramName> {
    match &opening.backend {
        BackendKind::Acp(program) => Some(program.clone()),
        BackendKind::Native | BackendKind::Fake | BackendKind::Watch(_) => None,
    }
}

impl<X: Seams, D: SheetDesk> SessionHost for AgentHost<X, D> {
    async fn open(&mut self, opening: Opening) -> Result<SessionId, AgentHostError> {
        if self.session.is_some() {
            return Err(BackendError::Busy.into());
        }
        let program = program_of(&opening)
            .ok_or_else(|| Step::Opening.unavailable(&"it is not for an agent program"))?;
        let cwd = opening
            .cwd
            .clone()
            .ok_or_else(|| Step::Opening.unavailable(&"it names no folder"))?;
        let session = self
            .court
            .open(OpenAgent {
                program,
                cwd,
                sheets: if self.terminal_fallback {
                    SheetSurface::Host
                } else {
                    SheetSurface::Desktop
                },
                space: opening.space.clone(),
                label: opening.label.clone(),
                rewind: self.rewind,
                helper: self.helper.clone(),
            })
            .await
            .map_err(|e| Step::Router.unavailable(&e))?;
        let started = self
            .backend
            .start(StartSession {
                session: session.clone(),
                opening,
            })
            .await;
        if let Err(fault) = started {
            self.court.close(&session).await;
            return Err(fault.into());
        }
        self.session = Some(session.clone());
        Ok(session)
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "`SessionHost::resume` is async; this host has no resume and answers at once"
    )]
    async fn resume(&mut self, _session: &SessionId) -> Result<ResumePlan, AgentHostError> {
        Err(AgentHostError::NoSuchSession)
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "`SessionHost::turn` is async; this host only stages the turn for `next_event`"
    )]
    async fn turn(&mut self, session: &SessionId, turn: UserTurn) -> Result<(), AgentHostError> {
        self.mine(session)?;
        if self.pending.is_some() {
            return Err(BackendError::Busy.into());
        }
        // The router records what the person said, and may ask the person about it (a turn that
        // widens the task): that wait happens in `next_event`, where a sheet can be shown.
        let (mut court, id, text) = (self.court.clone(), session.clone(), turn.text.clone());
        let recording: Recording = Box::pin(async move { court.turn(&id, &text).await });
        self.pending = Some(Opening2 { turn, recording });
        Ok(())
    }

    async fn next_event(
        &mut self,
        session: &SessionId,
    ) -> Result<Option<BackendEvent>, AgentHostError> {
        self.mine(session)?;
        loop {
            if let Some(pending) = self.pending.as_mut() {
                let sheet = pin!(self.desk.next_sheet(session));
                let recorded = match (
                    self.terminal_fallback,
                    select(pending.recording.as_mut(), sheet).await,
                ) {
                    (_, Either::Left((recorded, _))) => recorded,
                    (true, Either::Right((Some(request), _))) => {
                        return Ok(Some(BackendEvent::Confirm(Box::new(request))));
                    }
                    // No desk is asked for: nothing can be on it.
                    (_, Either::Right(_)) => continue,
                };
                let Some(Opening2 { turn, .. }) = self.pending.take() else {
                    continue;
                };
                let Ok(recorded) = recorded else {
                    return Ok(Some(BackendEvent::TurnEnd(TurnEnd::Failed)));
                };
                // From here the router counts the turn as running; the guard says when it is not.
                let guard = RunningTurn::begin(self.court.clone(), session.clone(), recorded);
                self.running = Some(guard);
                if let Err(fault) = self.backend.turn(turn).await {
                    if let Some(turn) = self.running.take() {
                        turn.end(Ended::Failed).await;
                    }
                    return Err(fault.into());
                }
                continue;
            }
            if !self.is_terminal() {
                let event = self.backend.next_event().await;
                settle(&mut self.running, event.as_ref()).await;
                return Ok(event);
            }
            let event = pin!(self.backend.next_event());
            let sheet = pin!(self.desk.next_sheet(session));
            return Ok(match select(event, sheet).await {
                Either::Left((event, _)) => {
                    settle(&mut self.running, event.as_ref()).await;
                    event
                }
                Either::Right((Some(request), _)) => Some(BackendEvent::Confirm(Box::new(request))),
                // A desk with nothing to hand out ends the race: the backend's event is what is
                // left, and the next pull asks for it again.
                Either::Right((None, _)) => continue,
            });
        }
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "`SessionHost::answer_sheet` is async; the desk takes the answer without waiting"
    )]
    async fn answer_sheet(
        &mut self,
        session: &SessionId,
        id: &ConfirmId,
        choice: SheetChoice,
    ) -> Result<(), AgentHostError> {
        self.mine(session)?;
        self.desk.answer(id, choice)?;
        Ok(())
    }

    async fn cancel(&mut self, session: &SessionId) -> Result<(), AgentHostError> {
        self.mine(session)?;
        self.backend.cancel().await;
        Ok(())
    }

    async fn close(&mut self, session: &SessionId, _cause: EndCause) -> Result<(), AgentHostError> {
        self.mine(session)?;
        self.pending = None;
        if let Some(turn) = self.running.take() {
            turn.end(Ended::Cancelled).await;
        }
        self.backend.close().await;
        self.court.close(session).await;
        self.session = None;
        Ok(())
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "`SessionHost::fork` is async; this host has no fork and answers at once"
    )]
    async fn fork(
        &mut self,
        _session: &SessionId,
        _at: agent_session::Seq,
    ) -> Result<SessionId, AgentHostError> {
        Err(AgentHostError::NoSuchSession)
    }

    fn export(
        &self,
        _session: &SessionId,
    ) -> impl Future<Output = Result<SessionExport, AgentHostError>> + Send {
        std::future::ready(Err(AgentHostError::NoSuchSession))
    }
}
