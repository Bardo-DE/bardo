//! Ending a run: the one place every road to `Finished` goes through, so the effects of an end
//! are the same whichever road it was.

use crate::attempts::stall_words;
use crate::machine::acquire::AcquireFailure;
use crate::machine::effect::{ComputerUseSignal, RunEffect};
use crate::machine::phase::RunPhase;
use crate::machine::run::Run;
use crate::machine::track::SettleWatch;
use computer_use_action::{Extracted, Summary};
use computer_use_bus::{
    BlockedOn, CancelledBy, ComputerUseOutcome, ComputerUseRecord, FailReason, RunBudgetKind,
    report_draft,
};
use computer_use_desktop::GlowState;

/// A summary of `text`; the empty one (the report's fallback line says something) when the text
/// does not fit a summary.
pub(crate) fn summary(text: &str) -> Summary {
    Summary::new(text)
        .or_else(|_| Summary::new(""))
        .expect("the empty summary is valid")
}

/// What a report says about an outcome the model did not write itself. Machine text, short.
pub(crate) fn said_of(outcome: ComputerUseOutcome) -> &'static str {
    match outcome {
        ComputerUseOutcome::Done
        | ComputerUseOutcome::Cancelled(CancelledBy::User | CancelledBy::Planner) => "",
        ComputerUseOutcome::Failed(FailReason::WindowGone) => "The window went away.",
        ComputerUseOutcome::Failed(FailReason::Model) => {
            "The on-screen model isn't available. You can choose one in Settings."
        }
        ComputerUseOutcome::Failed(FailReason::Capture) => "The window could not be seen.",
        ComputerUseOutcome::Failed(FailReason::Input) => "The window could not be controlled.",
        ComputerUseOutcome::Failed(FailReason::CheckFailed) => "The result did not check out.",
        ComputerUseOutcome::Failed(FailReason::Infeasible) => "This cannot be done in this window.",
        ComputerUseOutcome::Failed(FailReason::Log) => "What was done could not be written down.",
        ComputerUseOutcome::Failed(FailReason::CannotSeeScreen) => {
            inference_core::NO_MODEL_SEES_SCREEN
        }
        ComputerUseOutcome::Blocked(BlockedOn::ConsentDenied) => {
            "Computer use was not allowed for this app."
        }
        ComputerUseOutcome::Blocked(BlockedOn::LeaseRefused) => "The window could not be taken.",
        ComputerUseOutcome::Blocked(BlockedOn::PolicyDenials) => "Paused after refused actions.",
        ComputerUseOutcome::Blocked(BlockedOn::RouterUnreachable) => {
            "Stopped: it could not ask whether its next step was allowed."
        }
        ComputerUseOutcome::Blocked(BlockedOn::ModeUnsupported(_)) => {
            "That mode is not supported on this desktop."
        }
        ComputerUseOutcome::Blocked(BlockedOn::NeedsCapability) => {
            "This needs something this desktop does not offer yet."
        }
        ComputerUseOutcome::Blocked(BlockedOn::ScreenSuspicious(_)) => {
            "Stopped: the screen showed text that looked like instructions for the assistant."
        }
        ComputerUseOutcome::Cancelled(CancelledBy::KillSwitch) => "Stopped by the kill switch.",
        ComputerUseOutcome::BudgetOut(RunBudgetKind::Steps) => "Ran out of steps.",
        ComputerUseOutcome::BudgetOut(RunBudgetKind::Active) => "Ran out of time.",
        ComputerUseOutcome::BudgetOut(RunBudgetKind::Stall) => "The screen stopped changing.",
        ComputerUseOutcome::BudgetOut(RunBudgetKind::Rate) => "Too many actions in a minute.",
        ComputerUseOutcome::BudgetOut(RunBudgetKind::Spend) => "Ran out of budget.",
    }
}

/// What the model said when it finished: its summary and what it read off the screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Said {
    /// The summary.
    pub summary: Summary,
    /// The values.
    pub extracted: Vec<Extracted>,
}

impl Said {
    /// What the machine says for `outcome`.
    pub fn of(outcome: ComputerUseOutcome) -> Said {
        Said {
            summary: summary(said_of(outcome)),
            extracted: Vec::new(),
        }
    }
}

/// Ends the run with `outcome`: stops whatever it waits on, lets the window go, and says so on
/// the bus, in the log and to the requester (the report goes out before the record, so the
/// record can name the message that carried it).
pub(crate) fn finish(
    run: &mut Run,
    fx: &mut Vec<RunEffect>,
    outcome: ComputerUseOutcome,
    said: &Said,
) {
    match run.phase {
        RunPhase::Thinking(_) => fx.push(RunEffect::CancelModel),
        RunPhase::Settling(..) => fx.push(RunEffect::CancelSettle),
        _ => {}
    }
    if let Some(lease) = run.lease.take() {
        fx.push(RunEffect::Glow(GlowState::Off));
        fx.push(RunEffect::ReleaseLease(lease));
    }
    fx.push(RunEffect::Report(Box::new(report_draft(
        &run.task.report,
        outcome.status(),
        &said.summary,
        &said.extracted,
    ))));
    fx.push(RunEffect::Record(Box::new(ComputerUseRecord::Finished {
        run: run.id.clone(),
        outcome,
        steps: run.used.steps,
        charges: run.used.spend,
        report: None,
    })));
    fx.push(RunEffect::Emit(ComputerUseSignal::StateChanged(
        computer_use_bus::RunStateSlug::Finished,
    )));
    fx.push(RunEffect::Emit(ComputerUseSignal::Finished(outcome)));
    run.phase = RunPhase::Finished(outcome);
    run.pending.actions.clear();
    run.working = None;
    run.track.settle = SettleWatch::Idle;
}

/// Ends the run because the screen stopped changing, saying what was tried.
pub(crate) fn finish_stalled(run: &mut Run, fx: &mut Vec<RunEffect>) {
    let said = Said {
        summary: summary(&stall_words(&run.track.tried)),
        extracted: Vec::new(),
    };
    finish(
        run,
        fx,
        ComputerUseOutcome::BudgetOut(RunBudgetKind::Stall),
        &said,
    );
}

/// Ends the run with `outcome` and the machine's own words.
pub(crate) fn finish_plain(run: &mut Run, fx: &mut Vec<RunEffect>, outcome: ComputerUseOutcome) {
    finish(run, fx, outcome, &Said::of(outcome));
}

/// Ends the run blocked because a step of taking the window failed; the words name the step.
pub(crate) fn finish_acquire(run: &mut Run, fx: &mut Vec<RunEffect>, failure: &AcquireFailure) {
    let said = Said {
        summary: summary(failure.words()),
        extracted: Vec::new(),
    };
    finish(
        run,
        fx,
        ComputerUseOutcome::Blocked(BlockedOn::LeaseRefused),
        &said,
    );
}

/// Ends the run blocked because the question about the app was never answered.
pub(crate) fn finish_unanswered(run: &mut Run, fx: &mut Vec<RunEffect>) {
    let said = Said {
        summary: summary("Nobody answered the question about using this app, so the run stopped."),
        extracted: Vec::new(),
    };
    finish(
        run,
        fx,
        ComputerUseOutcome::Blocked(BlockedOn::ConsentDenied),
        &said,
    );
}

/// Ends the run blocked because its task names an app and no window: opening an app is not
/// available yet, and the words say so.
pub(crate) fn finish_no_window(run: &mut Run, fx: &mut Vec<RunEffect>) {
    let outcome = ComputerUseOutcome::Blocked(BlockedOn::NeedsCapability);
    let said = Said {
        summary: summary("That app has no open window, and opening apps is not available yet."),
        extracted: Vec::new(),
    };
    finish(run, fx, outcome, &said);
}
