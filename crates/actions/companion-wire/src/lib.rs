//! The wire of `org.bardo.Companion1`: what an app or the launcher asks, the answer object
//! they watch (`AnswerWire`: a phase, a body, a footer), the cards, plans and forms inside it,
//! the records companiond stores about its sessions, and the front pointer (the task the
//! launcher returns to). Pure and serde-only; the shell and the ds adapter read it, and Bardo
//! never names it (each pair of wire and view types has a `*_total` conversion test in the shell).
//!
//! Session records are stored as `EventBody::Area { area: Companion }` in this serde form.

mod answer;
mod ask;
mod handoff;
mod helpers;
mod record;
mod route;
mod route_log;
mod routines;

pub use answer::{
    AnswerBody, AnswerPhase, AnswerWire, CardWire, FooterWire, FormWire, NeedsYou, PlanStepWire,
    PlanWire, RefusalWire, StepWireState,
};
pub use ask::{AskWire, FrontTask};
pub use companion_model::RunsIn;
pub use handoff::{BroughtBack, Handover, TerminalSession};
pub use helpers::{
    HelperCardView, HelperForm, HelperLine, HelperNeeds, HelperReply, HelpersView, JobFrom,
    JobLine, JobShown, JobWords, LoginShown, SayReply, SignInChoice, ToldWire, app_name_of,
    card_of, line_of,
};
pub use record::{SESSION_KIND_PREFIX, SessionRecord};
pub use route::{
    Reached, RouteNote, WhySays, WhyWord, declined_text, footer_line, model_name, provider_name,
    why_says,
};
pub use route_log::RouteLog;
pub use routines::{
    EventForm, ProposalId, ProposalIdFault, RoutineCardView, RoutineForm, RoutineLine,
    RoutineProposal, RoutineReply, RoutineShown, RoutinesView, RunLine, RunShown, WhenForm,
    routine_card, routine_line,
};
