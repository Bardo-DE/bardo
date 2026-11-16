//! Builders the machine tests share.
#![allow(dead_code, reason = "shared test helpers: each test file uses only some of them")]

use action_core::*;
use action_vocab::{Caller, TurnId};
use agent_loop::*;
use companion_wire::SessionRecord;
use provenance::{AgentRef, SpaceId, TaskId, UnixSeconds};

pub fn space(s: &str) -> SpaceId {
    SpaceId::parse(s).expect("space")
}
pub fn task(s: &str) -> TaskId {
    TaskId::parse(s).expect("task")
}
pub fn at(t: i64) -> UnixSeconds {
    UnixSeconds(t)
}
pub fn worker(s: &str) -> AgentRef {
    AgentRef::Worker { task: task(s) }
}
pub fn turn(id: u64, text: &str, t: i64) -> UserTurn {
    UserTurn {
        id: TurnId(id),
        text: text.into(),
        at: at(t),
        from: Caller::launcher(),
        via: TurnVia::Typed,
        mode: None,
    }
}
pub fn session(record: SessionRecord) -> ReplayWhat {
    ReplayWhat::Session(Box::new(record))
}
pub fn rules() -> IdleRules {
    AgentConfig::default().idle
}
