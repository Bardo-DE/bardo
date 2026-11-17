//! Signing in: the way `agents.toml` names is sent with `authenticate` after `initialize`, its
//! reply is awaited before `session/new`, and an agent that wants signing in gets its own fault.

use super::agent::Auth;
use super::rig::{CWD, Fakes, Setup, opening, started, wired_over};
use agent_acp_host::fake::FakeFiles;
use agent_acp_host::{AgentHost, SignIn};
use agent_session::{AgentHostError, BackendError, SessionHost};
use serde_json::json;

fn method(id: &str) -> SignIn {
    SignIn::parse(id).expect("a sign-in id")
}

async fn open_with(
    auth: Auth,
    sign_in: Option<SignIn>,
) -> (Result<(), AgentHostError>, super::agent::View) {
    let wired = wired_over::<Fakes>(
        FakeFiles::new(),
        Setup {
            auth,
            sign_in,
            ..Setup::default()
        },
    );
    let mut host = AgentHost::new(
        wired.backend,
        wired.court.clone(),
        wired.desk.clone(),
        wired.terminal_fallback,
    );
    let opened = Box::pin(host.open(opening(CWD))).await.map(|_| ());
    (opened, wired.agent)
}

#[tokio::test]
async fn with_the_method_named_the_session_opens_after_authenticate() {
    let (opened, agent) = Box::pin(open_with(Auth::Needed, Some(method("oauth-personal")))).await;
    assert_eq!(opened, Ok(()));
    assert_eq!(
        agent.authenticated(),
        vec![json!({"methodId": "oauth-personal"})]
    );
    let order: Vec<String> = agent
        .lines()
        .iter()
        .filter_map(|l| l["method"].as_str().map(str::to_owned))
        .collect();
    assert_eq!(order, ["initialize", "authenticate", "session/new"]);
}

#[tokio::test]
async fn the_authenticate_reply_is_awaited_before_session_new_goes_out() {
    let (opened, agent) = Box::pin(open_with(Auth::Needed, Some(method("oauth-personal")))).await;
    assert_eq!(opened, Ok(()));
    // The fake refuses a `session/new` that was already in the pipe when it answered.
    assert_eq!(agent.raced(), 0);
}

#[tokio::test]
async fn without_a_method_an_agent_that_wants_signing_in_is_sign_in_needed_not_unavailable() {
    let (opened, agent) = Box::pin(open_with(Auth::Needed, None)).await;
    // The agent lists the ways it signs itself in by their names; the terminal one is not offered.
    let Err(AgentHostError::Backend(BackendError::SignInChoose(ways))) = opened else {
        panic!("expected the list of ways to sign in, got {opened:?}");
    };
    let names: Vec<_> = ways.0.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["Personal sign-in", "API key"]);
    assert!(agent.authenticated().is_empty(), "nothing was sent for it");
}

#[tokio::test]
async fn a_method_the_agent_did_not_advertise_is_refused_before_anything_is_sent() {
    for id in ["oauth-business", "login-terminal"] {
        let (opened, agent) = Box::pin(open_with(Auth::Needed, Some(method(id)))).await;
        assert_eq!(
            opened,
            Err(AgentHostError::Backend(BackendError::SignInUnsupported)),
            "{id}"
        );
        assert!(agent.authenticated().is_empty(), "{id}");
    }
}

#[tokio::test]
async fn an_agent_that_advertises_nothing_refuses_a_configured_method() {
    let (opened, _agent) = Box::pin(open_with(Auth::Open, Some(method("oauth-personal")))).await;
    assert_eq!(
        opened,
        Err(AgentHostError::Backend(BackendError::SignInUnsupported))
    );
}

#[tokio::test]
async fn an_agent_that_needs_no_sign_in_is_not_sent_one() {
    let (rig, _files) = Box::pin(started(Setup::default())).await;
    assert!(rig.agent.authenticated().is_empty());
}

#[test]
fn a_sign_in_id_is_a_short_plain_word() {
    SignIn::parse("oauth-personal").unwrap();
    SignIn::parse("a.b_c-9").unwrap();
    for bad in ["", "two words", "a/b", "tab\t", &"x".repeat(65)] {
        assert!(SignIn::parse(bad).is_err(), "{bad:?}");
    }
}
