//! Every git command has a deadline and a bound on what it prints, and a failure keeps its cause
//! for the operator's log while the store's own refusal stays plain.

use crate::scratch::{Scratch, root};
use action_checkpoint::{CheckpointStore, CheckpointStoreError, TakeAsk};
use action_checkpoint_git::{GitCauseError, GitError, GitRunError, Limits};
use action_core::CheckpointId;
use base_ids::Count;
use base_time::UnixSeconds;
use provenance::SessionId;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn ask(root: &action_checkpoint::WorkRoot, n: u32) -> TakeAsk {
    TakeAsk {
        root: root.clone(),
        session: SessionId::parse("s-limits").expect("session"),
        id: CheckpointId(n),
        at: UnixSeconds(5),
        max: Count(100),
    }
}

fn recorded() -> (Arc<Mutex<Vec<GitError>>>, impl Fn(&GitError) + Send + Sync) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    let tell = move |problem: &GitError| {
        if let Ok(mut all) = sink.lock() {
            all.push(problem.clone());
        }
    };
    (seen, tell)
}

#[tokio::test]
async fn a_command_past_its_deadline_is_killed_and_the_cause_is_a_timeout() {
    let Some(scratch) = Scratch::new() else {
        return;
    };
    let repo = scratch.repo("repo");
    let (seen, tell) = recorded();
    let limits = Limits {
        deadline: Duration::ZERO,
        stdout_max: 1024,
    };
    let store = scratch.store().with_limits(limits).with_problems(tell);
    assert_eq!(
        store.take(ask(&root(&repo), 1)).await,
        Err(CheckpointStoreError::Failed)
    );
    let all = seen.lock().expect("log");
    assert_eq!(
        all.first().map(|p| p.cause.clone()),
        Some(GitCauseError::Run(GitRunError::TimedOut))
    );
}

#[tokio::test]
async fn a_listing_over_the_output_limit_is_a_folder_too_large() {
    let Some(scratch) = Scratch::new() else {
        return;
    };
    let repo = scratch.repo("repo");
    std::fs::write(repo.join("a-long-enough-name.txt"), "x").expect("write");
    let limits = Limits {
        deadline: Duration::from_secs(60),
        stdout_max: 4,
    };
    let store = scratch.store().with_limits(limits);
    assert_eq!(
        store.take(ask(&root(&repo), 1)).await,
        Err(CheckpointStoreError::TooLarge)
    );
}

#[tokio::test]
async fn a_failed_command_keeps_the_tail_of_its_stderr_for_the_log_only() {
    let Some(scratch) = Scratch::new() else {
        return;
    };
    let repo = scratch.repo("repo");
    std::fs::write(repo.join("a.txt"), "x").expect("write");
    let (seen, tell) = recorded();
    let store = scratch.store().with_problems(tell);
    let there = root(&repo);
    store.take(ask(&there, 1)).await.expect("take");
    // The number is taken: the store says only that it failed.
    assert_eq!(
        store.take(ask(&there, 1)).await,
        Err(CheckpointStoreError::Failed)
    );
    let all = seen.lock().expect("log");
    let exit = all.iter().find_map(|p| match &p.cause {
        GitCauseError::Exit { stderr, .. } => Some((p.command.clone(), stderr.clone())),
        GitCauseError::Run(_) => None,
    });
    let (command, stderr) = exit.expect("an exit was logged");
    assert_eq!(command, "update-ref");
    assert!(!stderr.is_empty());
}
