//! Completion notes reach only a live front task; restart rebuilds from a source.

use action_core::RosterState;
use agent_loop::*;
use companion_daemon::*;
use companion_wire::{AnswerPhase, NeedsYou, SessionRecord};
use memory_core::{
    BodyMode, EventRef, EventSummary, JsonText, KindTag, RecentEntry, RecentQuery, ReplicaId, Seq,
};
use provenance::{Actor, AgentRef, ReportStatus, RunId, SpaceId, TaskId};
use std::sync::Mutex;

fn note(status: ReportStatus, attention: Attention) -> CompletionNote {
    CompletionNote {
        agent: AgentRef::ComputerUse {
            run: RunId::parse("r-3").expect("run"),
        },
        status,
        steps: account_count(9),
        values: account_count(2),
        why: None,
        attention,
    }
}

fn working() -> LoopState {
    LoopState {
        phase: LoopPhase::Planning,
        turn: None,
        steps: 1,
        pending: vec![],
        guard: Guard::default(),
    }
}

#[test]
fn a_quiet_completion_is_one_line_in_a_live_front_task() {
    let effects = completion_effects(
        &note(ReportStatus::Done, Attention::Quiet),
        Some(&working()),
    );
    assert_eq!(
        effects,
        [LoopEffect::Note(
            "the on-screen run finished: Done, 9 steps, 2 values".into()
        )]
    );
}

#[test]
fn a_result_that_needs_the_person_also_makes_the_answer_wait_for_them() {
    let effects = completion_effects(
        &note(ReportStatus::Failed, Attention::NeedsYou),
        Some(&working()),
    );
    assert_eq!(effects.len(), 2);
    assert!(matches!(
        &effects[1],
        LoopEffect::Publish(AnswerPhase::NeedsYou(NeedsYou::Question { .. }))
    ));
}

#[test]
fn nothing_is_added_to_a_finished_or_absent_front_task_or_for_progress() {
    let finished = LoopState {
        phase: LoopPhase::Finished(FinishedAs::Done),
        turn: None,
        steps: 3,
        pending: vec![],
        guard: Guard::default(),
    };
    assert_eq!(
        completion_effects(&note(ReportStatus::Done, Attention::Quiet), Some(&finished)),
        [] as [agent_loop::LoopEffect; 0]
    );
    assert_eq!(
        completion_effects(&note(ReportStatus::Done, Attention::Quiet), None),
        [] as [agent_loop::LoopEffect; 0]
    );
    assert_eq!(
        completion_effects(
            &note(ReportStatus::Progress, Attention::Quiet),
            Some(&working())
        ),
        [] as [agent_loop::LoopEffect; 0]
    );
}

struct Scripted(
    Result<Vec<RecentEntry>, ReplayError>,
    Mutex<Vec<RecentQuery>>,
);

impl Scripted {
    fn new(entries: Result<Vec<RecentEntry>, ReplayError>) -> Self {
        Self(entries, Mutex::new(vec![]))
    }
}

impl RecentSource for Scripted {
    #[expect(
        clippy::unused_async_trait_impl,
        reason = "`RecentSource::recent` is an async trait method; the scripted source has nothing to await"
    )]
    async fn recent(&self, query: RecentQuery) -> Result<Vec<RecentEntry>, ReplayError> {
        self.1.lock().expect("log").push(query);
        self.0.clone()
    }
}

fn entry(seq: u64, kind: &str, at: i64, body: Option<String>) -> RecentEntry {
    RecentEntry {
        summary: EventSummary {
            event: EventRef {
                space: SpaceId::parse("work").expect("space"),
                replica: ReplicaId([1; 16]),
                seq: Seq(seq),
            },
            occurred: provenance::UnixSeconds(at),
            kind: KindTag::parse(kind).expect("kind"),
            actor: Actor::User {
                via: account_core::AppName::parse("org.bardo.Shell").expect("app"),
            },
            things: vec![],
        },
        effect: provenance::Effect::Read,
        label: provenance::Label::trusted_user(),
        text: None,
        body: body.map(|b| JsonText::parse(&b).expect("json")),
    }
}

fn opened(task: &TaskId) -> String {
    serde_json::to_string(&SessionRecord::Opened {
        task: task.clone(),
        space: SpaceId::parse("work").expect("space"),
        agent: AgentRef::Companion,
        parent: None,
    })
    .expect("json")
}

#[tokio::test]
async fn restart_rebuilds_the_front_and_the_roster_from_recent_with_bodies() {
    let task = TaskId::parse("t-1").expect("task");
    let source = Scripted::new(Ok(vec![
        // Newest first, as `Recent` answers; a kind the rebuild does not read is skipped, and so
        // is an event whose body is gone.
        entry(3, "actions.call", 3, Some("{}".into())),
        entry(2, "companion.session.closed", 2, None),
        entry(1, "companion.session.opened", 1, Some(opened(&task))),
    ]));
    let rebuilt = recover(&source, provenance::UnixSeconds(0))
        .await
        .expect("rebuilt");
    assert_eq!(rebuilt.front, Some(task));
    assert_eq!(rebuilt.roster().entries[0].state, RosterState::Working);
    let queries = source.1.lock().expect("log");
    assert_eq!(queries.len(), 1);
    assert_eq!(queries[0].bodies, BodyMode::Json);
    assert_eq!(queries[0].since, provenance::UnixSeconds(0));
    let kinds: Vec<&str> = queries[0]
        .kinds
        .iter()
        .map(memory_core::KindPattern::as_str)
        .collect();
    assert_eq!(
        kinds,
        vec![
            "companion.session.*",
            "companion.message",
            "companion.episode",
            "computer_use.run.started",
            "computer_use.asked",
            "computer_use.confirmed",
            "computer_use.taken_over",
            "computer_use.handed_back",
            "computer_use.run.finished",
        ]
    );
}

#[test]
fn replay_reads_oldest_first_skips_the_routers_log_and_refuses_a_body_it_cannot_parse() {
    let task = TaskId::parse("t-1").expect("task");
    let two = vec![
        entry(2, "companion.session.opened", 20, Some(opened(&task))),
        entry(1, "companion.session.opened", 10, Some(opened(&task))),
    ];
    let events = replay_of(&two).expect("events");
    assert_eq!(
        events.iter().map(|e| e.at.0).collect::<Vec<_>>(),
        vec![10, 20]
    );
    for kind in [
        "companion.session.opened",
        "companion.message",
        "companion.episode",
    ] {
        let bad = vec![entry(1, kind, 1, Some("{\"nope\":1}".into()))];
        assert_eq!(replay_of(&bad), Err(ReplayError::Malformed), "{kind}");
    }
    // The router's own session log shares the kind; its versioned entries are read elsewhere.
    let logged = r#"{"version":1,"seq":4,"kind":"closed","v":"closed"}"#;
    let routers = vec![entry(1, "companion.session.closed", 1, Some(logged.into()))];
    assert_eq!(replay_of(&routers), Ok(vec![]));
}

/// A record of a run as the computer-use daemon writes it (`computer-use-bus`'s `ComputerUseRecord`: tag `kind`, content `v`).
fn run_record(kind: &str, v: &serde_json::Value) -> String {
    serde_json::json!({ "kind": kind, "v": v }).to_string()
}

#[test]
fn a_computer_use_run_is_rebuilt_from_its_area_records() {
    let started = run_record(
        "computer_use.run.started",
        &serde_json::json!({ "run": "r-3", "session": "s-2", "space": "work", "app": "org.example.Browser" }),
    );
    let asked = run_record(
        "computer_use.asked",
        &serde_json::json!({ "run": "r-3", "n": 4 }),
    );
    let rows = |last: &str, body: String| {
        vec![
            entry(4, last, 40, Some(body)),
            entry(3, "computer_use.asked", 30, Some(asked.clone())),
            entry(2, "computer_use.step", 20, Some("{}".into())),
            entry(1, "computer_use.run.started", 10, Some(started.clone())),
        ]
    };
    let state_after = |last: &str, body: String| {
        let events = replay_of(&rows(last, body)).expect("events");
        assert!(
            events
                .iter()
                .all(|e| matches!(e.what, agent_loop::ReplayWhat::Run { .. })),
            "only the run's own records are read: {events:?}"
        );
        recover_events(&events)
    };
    let finished = |outcome: serde_json::Value| {
        run_record(
            "computer_use.run.finished",
            &serde_json::json!({ "run": "r-3", "outcome": outcome }),
        )
    };
    let rows_expected: [(&str, String, RosterState); 7] = [
        (
            "computer_use.confirmed",
            run_record(
                "computer_use.confirmed",
                &serde_json::json!({ "run": "r-3", "n": 5 }),
            ),
            RosterState::Working,
        ),
        (
            "computer_use.taken_over",
            run_record(
                "computer_use.taken_over",
                &serde_json::json!({ "run": "r-3", "n": 5 }),
            ),
            RosterState::Paused,
        ),
        (
            "computer_use.handed_back",
            run_record(
                "computer_use.handed_back",
                &serde_json::json!({ "run": "r-3", "n": 6 }),
            ),
            RosterState::Working,
        ),
        (
            "computer_use.run.finished",
            finished(serde_json::json!({ "kind": "done" })),
            RosterState::Done,
        ),
        (
            "computer_use.run.finished",
            finished(serde_json::json!({ "kind": "failed", "v": "no_progress" })),
            RosterState::Failed,
        ),
        (
            "computer_use.run.finished",
            finished(serde_json::json!({ "kind": "budget_out", "v": "steps" })),
            RosterState::Failed,
        ),
        (
            "computer_use.run.finished",
            finished(serde_json::json!({ "kind": "cancelled", "v": "user" })),
            RosterState::Cancelled,
        ),
    ];
    for (kind, body, want) in rows_expected {
        assert_eq!(state_after(kind, body.clone()), want, "{kind} {body}");
    }
    // With nothing after the question, the run is waiting for the person.
    let only_asked = replay_of(&rows("computer_use.asked", asked.clone())).expect("events");
    assert_eq!(recover_events(&only_asked), RosterState::NeedsYou);
}

fn recover_events(events: &[agent_loop::ReplayEvent]) -> RosterState {
    let rebuilt = agent_loop::rebuild(events);
    rebuilt
        .tasks
        .iter()
        .find(|t| matches!(t.agent, AgentRef::ComputerUse { .. }))
        .map(|t| t.state)
        .expect("the run is on the roster")
}

#[test]
fn a_run_whose_start_is_older_than_the_window_says_nothing_and_a_broken_record_is_a_fault() {
    let finished = run_record(
        "computer_use.run.finished",
        &serde_json::json!({ "run": "r-9", "outcome": { "kind": "done" } }),
    );
    assert_eq!(
        replay_of(&[entry(1, "computer_use.run.finished", 5, Some(finished))]).expect("events"),
        vec![]
    );
    for body in ["{\"nope\":1}", "{\"kind\":\"computer_use.asked\",\"v\":{}}"] {
        let bad = vec![entry(1, "computer_use.asked", 1, Some(body.into()))];
        assert_eq!(replay_of(&bad), Err(ReplayError::Malformed), "{body}");
    }
}

#[tokio::test]
async fn an_unreadable_record_is_a_fault_not_a_guess() {
    let since = provenance::UnixSeconds(0);
    assert_eq!(
        recover(&Scripted::new(Err(ReplayError::Malformed)), since).await,
        Err(ReplayError::Malformed)
    );
    assert_eq!(
        recover(&Scripted::new(Err(ReplayError::Unavailable)), since).await,
        Err(ReplayError::Unavailable)
    );
}

fn account_count(n: u32) -> base_ids::Count {
    base_ids::Count(n)
}
