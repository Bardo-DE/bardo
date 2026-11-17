//! What crosses the computer-use daemon's bus and its event log, and nothing that runs.
//!
//! The `ComputerUse1` bodies ([`ComputerUseTask`], [`WindowBody`], [`BudgetBody`], [`AskBody`], [`RunEnd`]), run
//! states and outcomes ([`RunStateSlug`], [`ComputerUseOutcome`], [`ModeSupport`]), the event-log record
//! ([`ComputerUseRecord`]), the control refusals and who may call which member ([`ControlRefusal`],
//! [`permits`]). Serde bodies over the vocabulary crates (memory-core, action-core, account-core,
//! inference-core, prov, computer-use-action) and no capability crate, so the shell's models and the D-Bus codec
//! read the same types.
//!
//! **One message model.** A run's progress and final result are `provenance::Message`s of kind
//! `Report` (`Done`, `Failed`, `Cancelled`, `Progress`), sent to the requester through the
//! router: [`report_draft`] builds the draft, [`ComputerUseOutcome::status`] picks the status. There is
//! no separate result type, and a report carries no authority: whatever it says is input to the
//! receiver, whose own task policy and the gate decide what follows.

mod action;
mod bodies;
mod budget;
mod control;
mod ids;
mod record;
mod report;
mod screen;
mod state;
mod task;

pub use action::{ActionOutcome, ActionResult, LocalRefusal, Refusal, TargetedAction};
pub use bodies::{AskBody, BudgetBody, WindowBody};
pub use budget::{BudgetUse, RunBudget, RunBudgetKind};
pub use control::{ComputerUseCaller, ComputerUseMember, ControlRefusal, permits};
pub use ids::{ComputerUseEnvelope, ComputerUseVocab, QuestionId, WindowRef, slug_of};
pub use record::{
    AuthorisedBy, ComputerUseRecord, ConfirmBy, FrameNote, ThoughtDigest, computer_use_actor,
};
pub use report::{ReportDelivery, RunEnd, report_draft};
pub use screen::ScreenKind;
pub use state::{
    BlockedOn, CancelledBy, ComputerUseOutcome, FailReason, ModeGap, ModeSupport, RunStateSlug,
    backend_wire, modes_wire,
};
pub use task::{ComputerUseTask, ReportRoute, RunOrigin, SuccessCheck, TaskTarget};
