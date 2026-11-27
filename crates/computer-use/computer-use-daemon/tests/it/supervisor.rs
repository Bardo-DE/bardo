//! The supervisor over the fakes: a run end to end, who may start and control, the lease book,
//! and what the person's controls do to a run in flight.

use crate::support::*;
use action_gate_wire::GateAnswer;
use computer_use_action::{ComputerUseAction, Summary};
use computer_use_bus::{
    CancelledBy, ComputerUseCaller, ComputerUseMember, ComputerUseOutcome, ControlRefusal,
    QuestionId, ReportDelivery,
};
use computer_use_daemon::Supervisor;
use computer_use_desktop::{BackendKind, DesktopEvent, Suspend};
use computer_use_fake::scene::{click, finish, reply, task, user_goal, window};
use computer_use_fake::{DesktopCall, FakePdp, Fixture, SeatEvent, Verdict};
use computer_use_run::{ComputerUseSignal, FORCED_YES, RunPhase};
use computer_use_seat_input::Emulation;
use provenance::{Labelled, MessageKind, ReportStatus};

const STOCK: BackendKind = BackendKind::KwinStock;

fn ask() -> ComputerUseAction<computer_use_geometry::WindowSpace> {
    ComputerUseAction::Ask {
        question: Summary::new("Which account?").unwrap(),
        choices: vec![],
    }
}

#[tokio::test(start_paused = true)]
async fn end_to_end_scripted_run_done() {
    let sup = supervisor(
        STOCK,
        vec![reply(click()), reply(finish())],
        FakePdp::allow_all(),
    );
    let mut events = sup.subscribe();
    let id = sup
        .start(ComputerUseCaller::Companion, task())
        .await
        .expect("started");
    assert_eq!(id.as_str(), "r-1");
    // The fixture window is a sign-in form with a password field: the run asks before the
    // click whatever the gate says, and the person says yes.
    until(&sup, &id, |p| matches!(p, RunPhase::WaitingForUser(..))).await;
    sup.answer(
        ComputerUseCaller::Shell,
        &id,
        QuestionId(1),
        Labelled::new(FORCED_YES.to_owned(), user_goal().label),
    )
    .unwrap();
    let (end, signals) = ended_with(&mut events, &id).await;
    assert_eq!(end.outcome, ComputerUseOutcome::Done);
    assert_eq!(end.steps, base_ids::Count(2));
    assert!(
        matches!(end.report, ReportDelivery::Sent(_)),
        "the report went out"
    );
    assert!(matches!(
        signals.last(),
        Some(ComputerUseSignal::Finished(ComputerUseOutcome::Done))
    ));
    assert!(
        signals
            .iter()
            .any(|s| matches!(s, ComputerUseSignal::ActionProposed(..)))
    );
    assert!(
        signals
            .iter()
            .any(|s| matches!(s, ComputerUseSignal::ActionDone(..)))
    );

    let kinds: Vec<String> = sup
        .events()
        .records()
        .iter()
        .map(computer_use_bus::ComputerUseRecord::kind)
        .collect();
    assert_eq!(
        kinds,
        [
            "computer_use.run.started",
            "computer_use.asked",
            "computer_use.confirmed",
            "computer_use.step",
            "computer_use.step.seen",
            "computer_use.run.finished"
        ]
    );
    let performed = sup
        .seat()
        .events()
        .iter()
        .filter(|e| matches!(e, SeatEvent::Performed(..)))
        .count();
    assert_eq!(performed, 1, "one click reached the seat");
    assert!(matches!(
        sup.desktop().calls().last(),
        Some(DesktopCall::Released(_))
    ));
    let sent = sup.events().sent();
    assert_eq!(
        sent.len(),
        2,
        "a progress report for the question, then the final one"
    );
    assert_eq!(
        sent[1].1.kind,
        MessageKind::Report {
            status: ReportStatus::Done
        }
    );
    assert_eq!(sup.runs(), [] as [provenance::RunId; 0]);
    assert_eq!(sup.lease_book(), computer_use_run::LeaseBook::default());
    assert_eq!(sup.started(), 1);
}

#[tokio::test(start_paused = true)]
async fn start_refused_for_non_planner_caller_and_while_disabled_or_unaimed() {
    let sup = supervisor(STOCK, vec![], FakePdp::allow_all());
    assert_eq!(
        sup.start(ComputerUseCaller::Shell, task()).await,
        Err(ControlRefusal::NotPermitted)
    );
    assert_eq!(
        sup.start(ComputerUseCaller::Other, task()).await,
        Err(ControlRefusal::NotPermitted)
    );
    let mut missing = task();
    missing.target = computer_use_bus::TaskTarget::Window(
        computer_use_desktop::ToplevelIdent::parse("nope").unwrap(),
    );
    assert_eq!(
        sup.start(ComputerUseCaller::Router, missing).await,
        Err(ControlRefusal::WindowUnknown)
    );
    let mut hidden = task();
    hidden.mode = computer_use_action::RunMode::AgentWorkspace;
    assert_eq!(
        sup.start(ComputerUseCaller::Router, hidden).await,
        Err(ControlRefusal::ModeUnsupported),
        "a stock compositor cannot run the agent workspace"
    );
    assert_eq!(
        sup.started(),
        0,
        "a refused start uses no id and holds no window"
    );
    assert_eq!(sup.lease_book(), computer_use_run::LeaseBook::default());

    let off = Supervisor::new(
        computer_use_fake::scene::world(STOCK, vec![], FakePdp::allow_all()),
        computer_use_daemon::ComputerUseDaemonConfig::default(),
    );
    assert_eq!(
        off.start(ComputerUseCaller::Router, task()).await,
        Err(ControlRefusal::Disabled)
    );
}

#[tokio::test(start_paused = true)]
async fn a_start_the_model_cannot_serve_is_refused_before_it_takes_an_id_or_a_window() {
    let mut parts = computer_use_fake::scene::world(STOCK, vec![], FakePdp::allow_all());
    parts.model = computer_use_fake::ScriptedModel::new(vec![]).missing();
    let sup = Supervisor::new(parts, enabled());
    assert_eq!(
        sup.start(ComputerUseCaller::Router, task()).await,
        Err(ControlRefusal::ModelUnavailable)
    );
    assert_eq!(sup.started(), 0, "no id was used");
    assert_eq!(sup.lease_book(), computer_use_run::LeaseBook::default());
}

#[tokio::test(start_paused = true)]
async fn computer_use_on_with_no_model_that_reads_images_says_so_beside_the_setting() {
    let note = |blind: bool, on: bool| async move {
        let mut parts = computer_use_fake::scene::world(STOCK, vec![], FakePdp::allow_all());
        let model = computer_use_fake::ScriptedModel::new(vec![]);
        parts.model = if blind { model.blind() } else { model };
        let config = if on {
            enabled()
        } else {
            computer_use_daemon::ComputerUseDaemonConfig::default()
        };
        Supervisor::new(parts, config).sight_note().await
    };
    assert_eq!(
        note(true, true).await,
        Some(computer_use_run::NO_SIGHT_NOTE)
    );
    assert_eq!(
        note(false, true).await,
        None,
        "a model that may see is no warning"
    );
    assert_eq!(
        note(true, false).await,
        None,
        "computer use is off: nothing to warn of"
    );
}

#[tokio::test(start_paused = true)]
async fn a_window_has_one_run_at_a_time() {
    let sup = supervisor(
        STOCK,
        vec![reply(ask()), reply(finish())],
        FakePdp::allow_all(),
    );
    let mut events = sup.subscribe();
    let first = sup.start(ComputerUseCaller::Router, task()).await.unwrap();
    until(&sup, &first, |p| matches!(p, RunPhase::WaitingForUser(..))).await;
    assert_eq!(
        sup.start(ComputerUseCaller::Router, task()).await,
        Err(ControlRefusal::LeaseHeld)
    );
    assert_eq!(sup.runs(), vec![first.clone()]);
    let book = sup.lease_book();
    assert_eq!(
        book.holder(&computer_use_desktop::ToplevelIdent::parse("w1").unwrap()),
        Some(&first)
    );
    sup.control(ComputerUseCaller::Shell, &first, ComputerUseMember::Cancel)
        .unwrap();
    ended(&mut events, &first).await;
    let second = sup.start(ComputerUseCaller::Router, task()).await;
    assert_eq!(second.unwrap().as_str(), "r-2", "the window is free again");
}

#[tokio::test(start_paused = true)]
async fn the_person_answers_a_question_and_the_run_goes_on() {
    let sup = supervisor(
        STOCK,
        vec![reply(ask()), reply(finish())],
        FakePdp::allow_all(),
    );
    let mut events = sup.subscribe();
    let id = sup.start(ComputerUseCaller::Router, task()).await.unwrap();
    until(&sup, &id, |p| matches!(p, RunPhase::WaitingForUser(..))).await;
    let answer = || Labelled::new("the work one".to_owned(), user_goal().label);
    assert_eq!(
        sup.answer(ComputerUseCaller::Companion, &id, QuestionId(1), answer()),
        Err(ControlRefusal::NotPermitted),
        "only the shell answers"
    );
    assert_eq!(
        sup.answer(ComputerUseCaller::Shell, &id, QuestionId(7), answer()),
        Err(ControlRefusal::UnknownQuestion)
    );
    sup.answer(ComputerUseCaller::Shell, &id, QuestionId(1), answer())
        .unwrap();
    let end = ended(&mut events, &id).await;
    assert_eq!(end.outcome, ComputerUseOutcome::Done);
    let notes = &sup.events().sent();
    assert_eq!(
        notes.iter().map(|(_, d)| d.kind).collect::<Vec<_>>(),
        vec![
            MessageKind::Report {
                status: ReportStatus::Progress
            },
            MessageKind::Report {
                status: ReportStatus::Done
            }
        ],
        "the question was reported as progress before the end"
    );
}

#[tokio::test(start_paused = true)]
async fn pause_resume_take_over_and_hand_back_move_a_run_through_its_phases() {
    let sup = supervisor(
        STOCK,
        vec![reply(ask()), reply(ask()), reply(finish())],
        FakePdp::allow_all(),
    );
    let mut events = sup.subscribe();
    let id = sup.start(ComputerUseCaller::Router, task()).await.unwrap();
    until(&sup, &id, |p| matches!(p, RunPhase::WaitingForUser(..))).await;

    let wrong = |member| sup.control(ComputerUseCaller::Shell, &id, member);
    assert_eq!(
        wrong(ComputerUseMember::Resume),
        Err(ControlRefusal::WrongState)
    );
    assert_eq!(
        wrong(ComputerUseMember::HandBack),
        Err(ControlRefusal::WrongState)
    );
    assert_eq!(
        sup.control(ComputerUseCaller::Other, &id, ComputerUseMember::Pause),
        Err(ControlRefusal::NotPermitted)
    );

    sup.control(ComputerUseCaller::Shell, &id, ComputerUseMember::Pause)
        .unwrap();
    until(&sup, &id, |p| matches!(p, RunPhase::Paused(_))).await;
    assert_eq!(
        wrong(ComputerUseMember::Pause),
        Err(ControlRefusal::WrongState)
    );
    assert_eq!(
        sup.lease_book()
            .slots
            .values()
            .map(|slot| slot.state)
            .collect::<Vec<_>>(),
        vec![computer_use_run::SlotState::Suspended],
        "a paused run still holds its window, with input stopped"
    );
    assert!(
        sup.desktop()
            .calls()
            .iter()
            .any(|c| matches!(c, DesktopCall::Suspended(_, Suspend::Paused)))
    );

    sup.control(ComputerUseCaller::Shell, &id, ComputerUseMember::Resume)
        .unwrap();
    until(&sup, &id, |p| matches!(p, RunPhase::WaitingForUser(..))).await;
    sup.control(
        ComputerUseCaller::Companion,
        &id,
        ComputerUseMember::TakeOver,
    )
    .unwrap();
    until(&sup, &id, |p| matches!(p, RunPhase::TakenOver(..))).await;
    sup.control(ComputerUseCaller::Router, &id, ComputerUseMember::HandBack)
        .unwrap();
    let end = ended(&mut events, &id).await;
    assert_eq!(
        end.outcome,
        ComputerUseOutcome::Done,
        "the third reply finishes it"
    );
    let kinds: Vec<String> = sup
        .events()
        .records()
        .iter()
        .map(computer_use_bus::ComputerUseRecord::kind)
        .collect();
    assert!(
        kinds.contains(&"computer_use.taken_over".to_owned()),
        "{kinds:?}"
    );
    assert!(
        kinds.contains(&"computer_use.handed_back".to_owned()),
        "{kinds:?}"
    );
}

#[tokio::test(start_paused = true)]
async fn cancel_ends_a_run_whatever_it_is_waiting_on() {
    // A run waiting for the person.
    let sup = supervisor(STOCK, vec![reply(ask())], FakePdp::allow_all());
    let mut events = sup.subscribe();
    let id = sup.start(ComputerUseCaller::Router, task()).await.unwrap();
    until(&sup, &id, |p| matches!(p, RunPhase::WaitingForUser(..))).await;
    sup.control(ComputerUseCaller::Shell, &id, ComputerUseMember::Cancel)
        .unwrap();
    let end = ended(&mut events, &id).await;
    assert_eq!(
        end.outcome,
        ComputerUseOutcome::Cancelled(CancelledBy::User)
    );
    assert_eq!(
        sup.events().sent().last().unwrap().1.kind,
        MessageKind::Report {
            status: ReportStatus::Cancelled
        }
    );
    assert_eq!(
        sup.control(ComputerUseCaller::Shell, &id, ComputerUseMember::Cancel),
        Err(ControlRefusal::WrongState),
        "a run that is over is not there to cancel"
    );
}

#[tokio::test(start_paused = true)]
async fn kill_chord_finishes_all_runs() {
    let mut w = computer_use_fake::scene::world(
        STOCK,
        vec![reply(ask()), reply(ask())],
        FakePdp::allow_all(),
    );
    let mut second = window();
    second.ident = computer_use_desktop::ToplevelIdent::parse("w2").unwrap();
    w.desktop = computer_use_fake::FakeDesktop::new(STOCK, vec![window(), second]);
    let sup = Supervisor::new(w, enabled());
    let mut events = sup.subscribe();
    let first = sup.start(ComputerUseCaller::Router, task()).await.unwrap();
    until(&sup, &first, |p| matches!(p, RunPhase::WaitingForUser(..))).await;
    let mut other = task();
    other.target = computer_use_bus::TaskTarget::Window(
        computer_use_desktop::ToplevelIdent::parse("w2").unwrap(),
    );
    let second_id = sup.start(ComputerUseCaller::Router, other).await.unwrap();
    // The first run waits for the person and has let the model go, so the second gets it.
    until(&sup, &second_id, |p| {
        matches!(p, RunPhase::WaitingForUser(..))
    })
    .await;

    sup.desktop().raise(DesktopEvent::KillChord);
    for id in [&first, &second_id] {
        let end = ended(&mut events, id).await;
        assert_eq!(
            end.outcome,
            ComputerUseOutcome::Cancelled(CancelledBy::KillSwitch),
            "{id}"
        );
    }
    assert_eq!(sup.runs(), [] as [provenance::RunId; 0]);
    assert_eq!(sup.lease_book(), computer_use_run::LeaseBook::default());
}

#[tokio::test(start_paused = true)]
async fn stop_all_is_the_shells_kill_switch() {
    let sup = supervisor(STOCK, vec![reply(ask())], FakePdp::allow_all());
    let mut events = sup.subscribe();
    let id = sup.start(ComputerUseCaller::Router, task()).await.unwrap();
    until(&sup, &id, |p| matches!(p, RunPhase::WaitingForUser(..))).await;
    assert_eq!(
        sup.stop_all(ComputerUseCaller::Other),
        Err(ControlRefusal::NotPermitted)
    );
    sup.stop_all(ComputerUseCaller::Shell).unwrap();
    assert_eq!(
        ended(&mut events, &id).await.outcome,
        ComputerUseOutcome::Cancelled(CancelledBy::KillSwitch)
    );
}

#[tokio::test(start_paused = true)]
async fn stock_consent_guard_injects_nothing_while_prompt_shown() {
    let pdp = FakePdp::scripted(vec![], vec![Verdict::AskThen(GateAnswer::Run)]);
    let sup = supervisor(STOCK, vec![reply(click()), reply(finish())], pdp);
    let mut events = sup.subscribe();
    let id = sup.start(ComputerUseCaller::Router, task()).await.unwrap();
    assert_eq!(
        ended(&mut events, &id).await.outcome,
        ComputerUseOutcome::Done
    );
    let seat: Vec<&'static str> = sup
        .seat()
        .events()
        .iter()
        .map(|e| match e {
            SeatEvent::Attached(_) => "attach",
            SeatEvent::Emulation(_, Emulation::Stopped) => "stopped",
            SeatEvent::Emulation(_, Emulation::Running) => "running",
            SeatEvent::Performed(..) => "performed",
            SeatEvent::Detached(_) => "detach",
        })
        .collect();
    assert_eq!(
        seat,
        ["attach", "stopped", "running", "performed", "detach"]
    );
}

#[tokio::test(start_paused = true)]
async fn the_run_acts_in_the_session_it_opened_before_its_first_check() {
    let session = provenance::SessionId::parse("s-9").unwrap();
    let pdp = FakePdp::allow_all().opening(session.clone());
    // This test is about the session, not about sensitive windows: a dialog without a password
    // field, so that the run does not stop to ask before the click.
    let sup = supervisor_over(
        STOCK,
        vec![reply(click()), reply(finish())],
        pdp,
        Fixture::Gtk4Dialog,
    );
    let mut events = sup.subscribe();
    let mut planned = task();
    planned.session = provenance::SessionId::parse("s-1").unwrap();
    planned.conversation = provenance::SessionId::parse("s-1").unwrap();
    let id = sup.start(ComputerUseCaller::Router, planned).await.unwrap();
    assert_eq!(
        sup.view(&id).expect("live").task.session,
        session,
        "the planner's session is not the run's"
    );
    assert_eq!(
        ended(&mut events, &id).await.outcome,
        ComputerUseOutcome::Done
    );
    assert_eq!(sup.gate().opened(), vec![(id.clone(), task().space)]);
    assert_eq!(sup.gate().checks()[0].run, id);
    let sent = sup.events().sent();
    assert_eq!(
        sent[0].0, session,
        "the report goes out in the run's session"
    );
}

#[tokio::test(start_paused = true)]
async fn pausing_during_a_confirmation_takes_the_check_back() {
    let pdp = FakePdp::scripted(vec![], vec![Verdict::AskAndHang]);
    let sup = supervisor(STOCK, vec![reply(click()), reply(finish())], pdp);
    let id = sup.start(ComputerUseCaller::Router, task()).await.unwrap();
    until(&sup, &id, |p| matches!(p, RunPhase::Confirming(..))).await;
    assert_eq!(sup.gate().closes(), 0);
    sup.control(ComputerUseCaller::Shell, &id, ComputerUseMember::Pause)
        .unwrap();
    until(&sup, &id, |p| matches!(p, RunPhase::Paused(_))).await;
    assert_eq!(sup.gate().closes(), 1, "the sheet comes down");
}

#[tokio::test(start_paused = true)]
async fn the_callers_body_is_a_request_the_app_is_the_windows_and_limits_stay_in_the_settings() {
    let sup = supervisor(STOCK, vec![reply(finish())], FakePdp::allow_all());
    let mut elsewhere = task();
    elsewhere.app = account_core::AppName::parse("org.gnome.Terminal").unwrap();
    assert!(matches!(
        sup.start(ComputerUseCaller::Companion, elsewhere).await,
        Err(ControlRefusal::BadTask)
    ));
    let mut greedy = task();
    greedy.budget.steps = base_ids::Count(1_000_000);
    greedy.budget.stall_frames = base_ids::Count(0);
    let id = sup
        .start(ComputerUseCaller::Companion, greedy)
        .await
        .unwrap();
    let view = sup.view(&id).expect("live");
    assert_eq!(view.task.budget, computer_use_bus::RunBudget::PROPOSED);
    assert_eq!(
        view.task.goal.label.integrity,
        provenance::Integrity::Untrusted,
        "the planner's words are not the person's"
    );
    sup.stop_all(ComputerUseCaller::Shell).unwrap();
}

#[tokio::test(start_paused = true)]
async fn the_grant_carries_the_level_the_persons_row_for_the_app_gives() {
    use permission_core::Level;
    let rows = [
        (computer_use_daemon::AppAllow::Ask, Level::AlwaysAsk),
        (computer_use_daemon::AppAllow::Always, Level::AlwaysAllow),
    ];
    for (allow, level) in rows {
        let refused = action_wire::GrantAnswer::Refused(action_core::ConfirmEnd::Refused);
        let sup = supervisor(STOCK, vec![], FakePdp::scripted(vec![refused], vec![]));
        sup.apply_settings(&computer_use_daemon::ComputerUseSettings {
            config: sup.settings(),
            apps: computer_use_daemon::Apps::Listed(vec![computer_use_daemon::AppRule {
                app: computer_use_fake::scene::app(),
                allow,
            }]),
        });
        let mut events = sup.subscribe();
        let id = sup.start(ComputerUseCaller::Router, task()).await.unwrap();
        ended(&mut events, &id).await;
        let asks = sup.gate().grant_asks();
        assert_eq!(asks.len(), 1);
        assert_eq!(asks[0].level, level, "{allow:?}");
    }
}

#[tokio::test(start_paused = true)]
async fn a_desktop_event_the_run_ignores_leaves_the_consent_question_standing() {
    let mut parts = computer_use_fake::scene::world(STOCK, vec![], FakePdp::allow_all());
    parts.gate = FakePdp::grant_unanswered();
    let sup = Supervisor::new(parts, enabled());
    let mut events = sup.subscribe();
    let id = sup.start(ComputerUseCaller::Router, task()).await.unwrap();
    until(&sup, &id, |p| matches!(p, RunPhase::Granting)).await;
    sup.desktop().raise(DesktopEvent::ConsentShown);
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    assert!(
        matches!(sup.view(&id).map(|v| v.phase), Some(RunPhase::Granting)),
        "still waiting on the person"
    );
    // The question is still being asked: the run ends only when the wait does.
    tokio::time::sleep(std::time::Duration::from_secs(301)).await;
    let end = ended(&mut events, &id).await;
    assert_eq!(
        end.outcome,
        ComputerUseOutcome::Blocked(computer_use_bus::BlockedOn::ConsentDenied)
    );
}

#[tokio::test(start_paused = true)]
async fn a_consent_nobody_answers_ends_the_run_blocked_and_frees_the_window() {
    let mut parts = computer_use_fake::scene::world(STOCK, vec![], FakePdp::allow_all());
    parts.gate = FakePdp::grant_unanswered();
    let sup = Supervisor::new(parts, enabled());
    let mut events = sup.subscribe();
    let id = sup.start(ComputerUseCaller::Router, task()).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(301)).await;
    let end = ended(&mut events, &id).await;
    assert_eq!(
        end.outcome,
        ComputerUseOutcome::Blocked(computer_use_bus::BlockedOn::ConsentDenied)
    );
    assert!(matches!(end.report, ReportDelivery::Sent(_)));
    assert_eq!(sup.runs(), [] as [provenance::RunId; 0]);
    assert_eq!(sup.lease_book(), computer_use_run::LeaseBook::default());
}
