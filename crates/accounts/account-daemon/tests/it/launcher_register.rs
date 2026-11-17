//! The agent launcher's registration and caller rule
//! (`org.bardo.Accounts1.Launcher`, from accountd's `agent_launcher` tests): one launcher per
//! program, only the launcher calls, and a program has its launcher only while the connection that registered it is on the bus.

use crate::launcher_rig::*;

use account_daemon::launcher::LoginTiming;
use account_dbus::accounts::ServiceRoot;
use account_dbus::accounts::launcher_fault_error_name_in;
use account_wire::LauncherWireError;
use caller_role_types::CallerRole;

const DENIED: &str = "org.freedesktop.DBus.Error.AccessDenied";
const INVALID: &str = "org.freedesktop.DBus.Error.InvalidArgs";

/// The error name of a failed call.
pub fn error_name(error: zbus::Error) -> String {
    match error {
        zbus::Error::MethodError(name, ..) => name.to_string(),
        other => panic!("not a method error: {other}"),
    }
}

fn fault_name(fault: LauncherWireError) -> String {
    launcher_fault_error_name_in(fault, ServiceRoot::Accounts)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_program_has_one_launcher_first_wins_and_a_departed_connection_frees_it() {
    let rig = Rig::start(vec![], vec![], LoginTiming::new()).await;
    let first = rig.launcher("org.example.First").await;
    first
        .proxy
        .register_launcher(&[PROGRAM])
        .await
        .expect("registered");
    // Again by the same connection is a no-op; another program is added.
    first
        .proxy
        .register_launcher(&[PROGRAM, "codex"])
        .await
        .expect("again");

    let second = rig.launcher("org.example.Second").await;
    let refused = second
        .proxy
        .register_launcher(&[PROGRAM])
        .await
        .expect_err("held");
    assert_eq!(
        error_name(refused),
        fault_name(LauncherWireError::AlreadyRegistered)
    );
    // All or none: the free program was not taken either.
    let none_taken = second
        .proxy
        .register_launcher(&["gemini", PROGRAM])
        .await
        .expect_err("all or none");
    assert_eq!(
        error_name(none_taken),
        fault_name(LauncherWireError::AlreadyRegistered)
    );
    second
        .proxy
        .register_launcher(&["gemini"])
        .await
        .expect("the free one is still free");

    // The first connection leaves: its programs are free, once the bus says so.
    drop(first);
    base_testkit::eventually_some_async("the departed launcher's program to be free", || {
        let proxy = second.proxy.clone();
        async move {
            match proxy.register_launcher(&[PROGRAM]).await {
                Ok(()) => Some(()),
                Err(error) => {
                    assert_eq!(
                        error_name(error),
                        fault_name(LauncherWireError::AlreadyRegistered)
                    );
                    None
                }
            }
        }
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn only_the_launcher_calls_and_bad_words_are_invalid() {
    let rig = Rig::start(vec![], vec![], LoginTiming::new()).await;
    let app = rig.client_as("org.example.App", CallerRole::App).await;
    let stranger = rig.bus.connect().await;
    for connection in [&app, &stranger] {
        let proxy = account_dbus::accounts::LauncherProxy::new(connection)
            .await
            .expect("proxy");
        assert_eq!(
            error_name(
                proxy
                    .register_launcher(&[PROGRAM])
                    .await
                    .expect_err("refused")
            ),
            DENIED
        );
        assert_eq!(
            error_name(proxy.begin_session("s1").await.expect_err("refused")),
            DENIED
        );
        assert_eq!(
            error_name(
                proxy
                    .report_agent_logout("login-1", "ready", "")
                    .await
                    .expect_err("refused")
            ),
            DENIED
        );
    }

    let launcher = rig.launcher("org.example.Launcher").await;
    assert_eq!(
        error_name(
            launcher
                .proxy
                .register_launcher(&[])
                .await
                .expect_err("none")
        ),
        INVALID
    );
    assert_eq!(
        error_name(
            launcher
                .proxy
                .register_launcher(&["Not A Program!"])
                .await
                .expect_err("bad")
        ),
        INVALID
    );
    assert_eq!(
        error_name(
            launcher
                .proxy
                .report_agent_login("login-1", "ready", "")
                .await
                .expect_err("no such request")
        ),
        fault_name(LauncherWireError::UnknownRequest)
    );
}
