//! The durable session, portable and pure (design note `acp-sessions.md` sections 4 and 5).
//!
//! - `SessionEntry`: one thing that happened to a session. A session's log is its entries, in
//!   order, appended and never changed; `encode` and `decode` are the stored form, with an
//!   explicit version, under the kind tags `companion.session.<slug>`.
//! - `resume_plan`: entries in, a `ResumePlan` out. Taint never goes down, a call that was
//!   in flight ends interrupted and is never run again, the policy comes back as stored, handles
//!   come back as labels, and a gap or an unreadable entry fails closed.
//! - `checkpoint_rows`: the log's restore-point notes folded into a list's rows.
//! - `fork` and `export`: a child's first entries, and a stable JSON document.
//! - `may_restore`: who may bring a stored session back by naming it (the opener rule).
//! - `SessionLog`, `SessionBackend` and `SessionHost`: the seams. The native and ACP backends
//!   and the host are later lanes.
//!
//! Nothing here reaches a bus, a runtime or a clock: every time and every id is passed in.

mod backend;
mod checkpoints;
mod codec;
mod desk;
mod entry;
mod export;
mod fork;
mod log;
mod plan;
mod restore_rule;
mod resume;
mod thing;

pub use backend::{
    AgentHostError, BackendError, BackendEvent, CallEvent, Choice, Choices, Resumed,
    SessionBackend, SessionHost, SheetChoice, StartSession, TurnEnd, UsageNote,
};
pub use checkpoints::{checkpoint_rows, note_rows};
pub use codec::{
    CURRENT, EncodeError, Encoded, EntryVersion, Logged, Read, Unreadable, decode, encode, kind_tag,
};
pub use desk::{DeskError, NoDesk, SheetDesk};
pub use entry::{
    BackendKind, BreakerNote, CallOpen, Compacted, CompactedBy, EndCause, ForkPoint, HandleLabel,
    Opening, Seq, SessionEntry, SkillUse, TaintCause, TaintNote, UnknownEntry,
};
pub use export::{EXPORT_VERSION, ExportError, SessionExport, export, from_json, to_json};
pub use fork::{ForkError, child_names, fork, forks_of};
pub use log::{Appended, LogError, LogPage, PageSize, SessionLog, read_all};
pub use plan::{Blocker, Interrupted, PlanError, ResumeFault, ResumePlan, ResumedBudget, Standing};
pub use restore_rule::{Claimant, may_restore, may_restore_opening, may_restore_plan};
pub use resume::resume_plan;
pub use thing::{SESSION_KIND, session_thing};
