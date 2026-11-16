//! A router gate that answers from a table.

use action_core::CallRefusal;
use action_gate_wire::{CuaAsk, GateAnswer};
use action_wire::{GrantAnswer, GrantAsk};
use computer_use_run::{GateCheck, GateEvent, GateLink, LinkError};
use provenance::{ConfirmId, RunId, SessionId, SpaceId};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// What the fake router says to the next `Gate.Check`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Run it.
    Run,
    /// Refuse it.
    Refuse(CallRefusal),
    /// Ask the person (cuad hears `Confirming`, calls `Proceed`), then answer this.
    AskThen(GateAnswer),
    /// Ask the person and never answer: the check hangs after `Confirming` until it is closed
    /// (a run paused during a confirmation).
    AskAndHang,
}

/// A [`GateLink`] over a queue of grant answers and a queue of verdicts; when a queue is empty
/// it grants and runs.
#[derive(Debug, Default)]
pub struct FakePdp {
    grants: Mutex<VecDeque<GrantAnswer>>,
    verdicts: Mutex<VecDeque<Verdict>>,
    checks: Mutex<Vec<CuaAsk>>,
    grant_asks: Mutex<Vec<GrantAsk>>,
    /// Whether the person never answers the consent question: the call never returns.
    grant_hangs: bool,
    confirms: Mutex<u32>,
    session: Option<SessionId>,
    closes: Arc<Mutex<u32>>,
    opened: Mutex<Vec<(RunId, SpaceId)>>,
}

impl FakePdp {
    /// A gate that grants everything and runs every action.
    #[must_use]
    pub fn allow_all() -> Self {
        Self::default()
    }

    /// A gate that answers `grants` and `verdicts` in order, then grants and runs.
    #[must_use]
    pub fn scripted(grants: Vec<GrantAnswer>, verdicts: Vec<Verdict>) -> Self {
        Self {
            grants: Mutex::new(grants.into()),
            verdicts: Mutex::new(verdicts.into()),
            ..Self::default()
        }
    }

    fn confirming(&self) -> GateEvent {
        let mut confirms = self.confirms.lock().expect("fake gate lock");
        *confirms += 1;
        let id = ConfirmId::parse(&format!("c-{confirms}")).expect("a well-formed id");
        GateEvent::Confirming(id)
    }

    /// A gate whose consent question nobody answers: `Gate.Grant` never returns.
    #[must_use]
    pub fn grant_unanswered() -> Self {
        Self {
            grant_hangs: true,
            ..Self::default()
        }
    }

    /// Opens this session for every run (`s-1` when not set), as the router would.
    #[must_use]
    pub fn opening(mut self, session: SessionId) -> Self {
        self.session = Some(session);
        self
    }

    /// How many checks were taken back with `close`.
    ///
    /// # Panics
    ///
    /// Panics if the lock was poisoned by a panic on another thread.
    pub fn closes(&self) -> u32 {
        *self.closes.lock().expect("fake gate lock")
    }

    /// The runs it opened a session for, in order, with their Space.
    ///
    /// # Panics
    ///
    /// Panics if the lock was poisoned by a panic on another thread.
    pub fn opened(&self) -> Vec<(RunId, SpaceId)> {
        self.opened.lock().expect("fake gate lock").clone()
    }

    /// Every action it was asked about, in order.
    ///
    /// # Panics
    ///
    /// Panics if the lock was poisoned by a panic on another thread.
    pub fn checks(&self) -> Vec<CuaAsk> {
        self.checks.lock().expect("fake gate lock").clone()
    }

    /// Every consent it was asked for.
    ///
    /// # Panics
    ///
    /// Panics if the lock was poisoned by a panic on another thread.
    pub fn grant_asks(&self) -> Vec<GrantAsk> {
        self.grant_asks.lock().expect("fake gate lock").clone()
    }
}

/// One check in flight.
#[derive(Debug)]
pub struct FakeCheck {
    events: VecDeque<GateEvent>,
    /// Whether, once it has said everything, the next read never returns (the router has not
    /// answered yet) rather than failing (the router is gone).
    hangs: bool,
    proceeded: Mutex<u32>,
    closed: Arc<Mutex<u32>>,
}

impl FakeCheck {
    /// How many times `close` was called.
    ///
    /// # Panics
    ///
    /// Panics if the lock was poisoned by a panic on another thread.
    pub fn closed(&self) -> u32 {
        *self.closed.lock().expect("fake gate lock")
    }

    /// How many times `proceed` was called.
    ///
    /// # Panics
    ///
    /// Panics if the lock was poisoned by a panic on another thread.
    pub fn proceeded(&self) -> u32 {
        *self.proceeded.lock().expect("fake gate lock")
    }
}

impl GateLink for FakePdp {
    type Check = FakeCheck;

    async fn grant(&self, ask: GrantAsk) -> Result<GrantAnswer, LinkError> {
        self.grant_asks.lock().expect("fake gate lock").push(ask);
        if self.grant_hangs {
            std::future::pending::<()>().await;
        }
        Ok(self
            .grants
            .lock()
            .expect("fake gate lock")
            .pop_front()
            .unwrap_or(GrantAnswer::Granted))
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "GateLink declares `open_session` async; the fake has nothing to await"
    )]
    async fn open_session(
        &self,
        run: &RunId,
        space: &SpaceId,
        _conversation: &SessionId,
    ) -> Result<SessionId, LinkError> {
        self.opened
            .lock()
            .expect("fake gate lock")
            .push((run.clone(), space.clone()));
        Ok(self
            .session
            .clone()
            .unwrap_or_else(|| SessionId::parse("s-1").expect("a session id")))
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "GateLink declares `check` async; the fake has nothing to await"
    )]
    async fn check(&self, ask: CuaAsk) -> Result<FakeCheck, LinkError> {
        self.checks.lock().expect("fake gate lock").push(ask);
        let verdict = self
            .verdicts
            .lock()
            .expect("fake gate lock")
            .pop_front()
            .unwrap_or(Verdict::Run);
        let (events, hangs) = match verdict {
            Verdict::Run => (vec![GateEvent::Answered(GateAnswer::Run)], false),
            Verdict::Refuse(why) => (vec![GateEvent::Answered(GateAnswer::Refused(why))], false),
            Verdict::AskThen(answer) => {
                (vec![self.confirming(), GateEvent::Answered(answer)], false)
            }
            Verdict::AskAndHang => (vec![self.confirming()], true),
        };
        Ok(FakeCheck {
            events: events.into(),
            hangs,
            proceeded: Mutex::new(0),
            closed: Arc::clone(&self.closes),
        })
    }
}

impl GateCheck for FakeCheck {
    async fn next(&mut self) -> Result<GateEvent, LinkError> {
        match self.events.pop_front() {
            Some(event) => Ok(event),
            None if self.hangs => std::future::pending().await,
            None => Err(LinkError::Unavailable),
        }
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "GateCheck declares `proceed` async; the fake has nothing to await"
    )]
    async fn proceed(&mut self) -> Result<(), LinkError> {
        *self.proceeded.lock().expect("fake gate lock") += 1;
        Ok(())
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "GateCheck declares `close` async; the fake has nothing to await"
    )]
    async fn close(&mut self) -> Result<(), LinkError> {
        *self.closed.lock().expect("fake gate lock") += 1;
        self.events.clear();
        Ok(())
    }
}
