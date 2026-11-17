//! What the bridge to the router adds: the agent's grants live in the actions service's store (created from
//! the sheet's "Always", used and audited, listed and revoked over Control), a reviewer's no
//! refuses, the router's sheets reach the person and come back (the desktop's by default, the
//! host's own only with the development flag), and the person's prompt is what derives the task.

use super::agent::{Act, call};
use super::rig::{
    Policy, Rig, Setup, abs, always, host_caller, no, once, program, read, run_turn,
    run_turn_answering, started, write,
};
use account_core::{AppId, AppName, Isolation};
use action_core::AuditRecord;
use action_fake::{ReviewMode, ScriptedReviewer};
use action_review::{ReviewReason, ReviewVerdict};
use action_vocab::{Caller, CallerRole};
use action_wire::{CallerId, IntentsReply, IntentsRequest};
use agent_session::{BackendEvent, SheetChoice};
use permission_core::{GrantCaller, ReasonCode, ReasonText, StandingScope};
use std::collections::BTreeSet;

fn control() -> CallerId {
    CallerId {
        app: AppId {
            name: AppName::parse("org.bardo.Shell").expect("app"),
            isolation: Isolation::Unsandboxed,
        },
        roles: BTreeSet::from([CallerRole::Control]),
    }
}

async fn over_control(rig: &Rig<super::rig::Fakes>, request: IntentsRequest) -> IntentsReply {
    rig.router.handle(&control(), request).await
}

fn used(rig: &Rig<super::rig::Fakes>) -> usize {
    rig.audit()
        .iter()
        .filter(|r| matches!(r, AuditRecord::StandingUsed { .. }))
        .count()
}

/// A tainted session asks about every write; these are the agent's calls for the tests below.
fn taint_then_writes(paths: &[&'static str]) -> Vec<Act> {
    let mut acts = vec![call("r", "fs/read_text_file", read("/work/app/a.txt"))];
    for (n, path) in paths.iter().enumerate() {
        let tag: &'static str = Box::leak(format!("w{n}").into_boxed_str());
        acts.push(call(tag, "fs/write_text_file", write(path, "x")));
    }
    acts.push(Act::Stop("end_turn"));
    acts
}

#[tokio::test]
async fn an_always_from_the_sheet_is_a_grant_in_actions_service_store_used_audited_listed_and_revoked()
 {
    let (mut rig, files) = Box::pin(started(Setup {
        turns: vec![
            taint_then_writes(&[
                "/work/app/src/a.rs",
                "/work/app/src/b.rs",
                "/work/app/lib/c.rs",
            ]),
            taint_then_writes(&[]),
        ],
        answers: vec![always(), no()],
        ..Setup::default()
    }))
    .await;
    files.put(&abs("/work/app/a.txt"), "a");
    Box::pin(run_turn(&mut rig, "go")).await;
    // The first write asked and the person said always; the second, in the same directory, ran on
    // the grant; the third, in a sibling directory, asked again (and was refused).
    rig.agent.reply("w0").unwrap();
    rig.agent.reply("w1").unwrap();
    rig.agent.reply("w2").unwrap_err();
    assert_eq!(rig.sheets().len(), 2);
    assert_eq!(used(&rig), 1);
    let held = rig.grants();
    assert_eq!(held.len(), 1);
    assert_eq!(held[0].caller, GrantCaller::AcpAgent(program()));
    assert!(
        matches!(&held[0].scope, StandingScope::Files { under, .. } if under.as_str() == "/work/app/src")
    );

    // Settings lists it over Control, and revoking it makes the next call ask again.
    let listed = over_control(&rig, IntentsRequest::ControlStandingGrants).await;
    assert_eq!(listed, IntentsReply::StandingGrants(held.clone()));
    let gone = over_control(
        &rig,
        IntentsRequest::ControlStandingRevoke(held[0].id.clone()),
    )
    .await;
    assert_eq!(gone, IntentsReply::Done);
    assert!(rig.grants().is_empty());
}

#[tokio::test]
async fn a_grant_is_the_programs_and_not_another_hosts_or_programs() {
    let (mut rig, files) = Box::pin(started(Setup {
        turns: vec![taint_then_writes(&["/work/app/src/a.rs"])],
        answers: vec![always()],
        ..Setup::default()
    }))
    .await;
    files.put(&abs("/work/app/a.txt"), "a");
    Box::pin(run_turn(&mut rig, "go")).await;
    let held = rig.grants();
    assert_eq!(held.len(), 1);
    // Another program (or an editor of the same app name) holds none of it.
    assert_ne!(
        held[0].caller,
        GrantCaller::AcpAgent(permission_core::ProgramName::parse("gemini-cli").expect("program"))
    );
    assert_ne!(
        held[0].caller,
        GrantCaller::Editor(provenance::ClientName::parse(super::rig::HOST).expect("client"))
    );
    assert_ne!(
        host_caller().app.name.as_str(),
        "",
        "the host's identity is its verified app"
    );
}

#[tokio::test]
async fn a_reviewers_no_refuses_a_write_a_grant_let_through() {
    let deny = Ok(ReviewVerdict::Deny {
        why: ReviewReason {
            code: ReasonCode::OutsideRequest,
            text: ReasonText("no".into()),
        },
    });
    let (mut rig, files) = Box::pin(started(Setup {
        turns: vec![taint_then_writes(&[
            "/work/app/src/a.rs",
            "/work/app/src/b.rs",
        ])],
        answers: vec![always()],
        // The first write is asked of the person (no review); the second runs on the grant and
        // meets the reviewer, whose first verdict is no.
        reviewer: ScriptedReviewer::queued(vec![deny], ReviewMode::AlwaysAllow),
        ..Setup::default()
    }))
    .await;
    files.put(&abs("/work/app/a.txt"), "a");
    Box::pin(run_turn(&mut rig, "go")).await;
    rig.agent.reply("w0").unwrap();
    assert!(rig.agent.reply("w1").is_err(), "the reviewer said no");
    assert_eq!(files.text(&abs("/work/app/src/b.rs")), None);
    assert_eq!(rig.sheets().len(), 1);
}

#[tokio::test]
async fn the_sheet_goes_to_the_desktops_confirmer_and_its_answer_comes_back() {
    let (mut rig, files) = Box::pin(started(Setup {
        turns: vec![taint_then_writes(&["/work/app/a.rs", "/work/app/b.rs"])],
        answers: vec![once(), no()],
        ..Setup::default()
    }))
    .await;
    files.put(&abs("/work/app/a.txt"), "a");
    let events = Box::pin(run_turn(&mut rig, "go")).await;
    rig.agent.reply("w0").unwrap();
    rig.agent.reply("w1").unwrap_err();
    let sheets = rig.sheets();
    assert_eq!(sheets.len(), 2);
    assert_eq!(sheets[0].app.as_str(), "org.bardo.AcpAgent");
    assert_eq!(sheets[0].action.as_str(), "Write a file");
    assert_eq!(
        sheets[0].editor, None,
        "the desktop's sheet, not the host's"
    );
    assert!(
        !events.iter().any(|e| matches!(e, BackendEvent::Confirm(_))),
        "the host shows nothing"
    );
    assert_eq!(rig.desk.waiting(), 0);
}

#[tokio::test]
async fn the_hosts_own_sheet_exists_only_with_the_development_flag() {
    let (mut rig, files) = Box::pin(started(Setup {
        turns: vec![taint_then_writes(&["/work/app/a.rs", "/work/app/b.rs"])],
        terminal_fallback: true,
        ..Setup::default()
    }))
    .await;
    files.put(&abs("/work/app/a.txt"), "a");
    let mut shown = Vec::new();
    let events = Box::pin(run_turn_answering(&mut rig, 1, "go", &mut |request| {
        shown.push(request.clone());
        if shown.len() == 1 {
            SheetChoice::Once
        } else {
            SheetChoice::Refused
        }
    }))
    .await;
    assert_eq!(shown.len(), 2, "both sheets came out of the host");
    assert!(
        shown[0].editor.is_some(),
        "the router was told to route it to the host"
    );
    assert!(rig.sheets().is_empty(), "the desktop was not asked");
    rig.agent.reply("w0").unwrap();
    rig.agent.reply("w1").unwrap_err();
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, BackendEvent::Confirm(_)))
            .count(),
        2
    );
}

#[tokio::test]
async fn a_sheet_the_person_refuses_on_the_host_refuses_the_call() {
    let (mut rig, files) = Box::pin(started(Setup {
        turns: vec![taint_then_writes(&["/work/app/a.rs"])],
        terminal_fallback: true,
        ..Setup::default()
    }))
    .await;
    files.put(&abs("/work/app/a.txt"), "a");
    Box::pin(run_turn_answering(&mut rig, 1, "go", &mut |_| {
        SheetChoice::Refused
    }))
    .await;
    rig.agent.reply("w0").unwrap_err();
    assert_eq!(files.text(&abs("/work/app/a.rs")), None);
}

#[tokio::test]
async fn the_persons_prompt_is_what_derives_the_task_and_is_recorded_as_the_hosts() {
    let (mut rig, _files) = Box::pin(started(Setup {
        turns: vec![vec![Act::Stop("end_turn")]],
        policy: Policy::Wide,
        ..Setup::default()
    }))
    .await;
    Box::pin(run_turn(&mut rig, "update the changelog")).await;
    let st = rig.router.state.lock().expect("lock");
    let record = &st.sessions[&rig.session];
    assert_eq!(record.turns.len(), 1);
    assert_eq!(record.turns[0].text, "update the changelog");
    assert_eq!(
        record.turns[0].from,
        Caller::agent(AppName::parse(super::rig::HOST).expect("app"))
    );
    assert!(
        record.policy.is_some(),
        "the policy derived from the person's words"
    );
}

#[tokio::test]
async fn a_call_dropped_and_asked_again_with_its_number_is_one_call_at_the_router() {
    use agent_acp_core::call::AgentCall;
    use agent_acp_core::court::Court;
    use futures_util::FutureExt;
    let (rig, _files) = Box::pin(started(Setup::default())).await;
    let mut court = rig.court.clone();
    let call = AgentCall::Reported(permission_core::PermissionKind::Read);
    // The first pull gave up before the router answered; the second asks again.
    assert!(court.call(&rig.session, 7, &call).now_or_never().is_none());
    let ruled = court.call(&rig.session, 7, &call).await;
    assert!(
        matches!(ruled, agent_acp_core::court::Ruled::Done(_)),
        "{ruled:?}"
    );
    let routed = rig
        .audit()
        .iter()
        .filter(|r| matches!(r, AuditRecord::Call { action, .. } if action.name.as_str() == "acpagent.reported"))
        .count();
    assert_eq!(routed, 1, "one call, however many times it was asked for");
}

#[tokio::test]
async fn the_host_runs_one_agent_session_of_an_agent_program() {
    use super::rig::{Fakes, wired_over};
    use agent_acp_host::AgentHost;
    use agent_session::{BackendKind, HostFault, SessionHost};
    let wired = wired_over::<Fakes>(agent_acp_host::fake::FakeFiles::new(), Setup::default());
    let mut host = AgentHost::new(wired.backend, wired.court, wired.desk, false);
    let mut native = super::rig::opening("/work/app");
    native.backend = BackendKind::Native;
    assert!(
        Box::pin(host.open(native)).await.is_err(),
        "not an agent program"
    );
    let session = Box::pin(host.open(super::rig::opening("/work/app")))
        .await
        .expect("open");
    let again = Box::pin(host.open(super::rig::opening("/work/app"))).await;
    assert!(matches!(again, Err(HostFault::Backend(_))), "{again:?}");
    // Fork, export and resume are not served.
    host.resume(&session).await.unwrap_err();
    host.export(&session).await.unwrap_err();
}
