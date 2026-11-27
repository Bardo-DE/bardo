//! Role checks (PLAN §2.1): an `Agent` is refused what acts for the person, the agent launcher is
//! not accountd's to serve, an unknown sender is `AccessDenied`, and only the shell asks who needs
//! signing in again.

use crate::common;

use account_dbus::Details;
use account_dbus::accounts::account_path;
use account_dbus::accounts::{AccountProxy, ManagerProxy};
use account_wire::Refusal;
use caller_role_types::CallerRole;
use common::*;
use zbus::zvariant::ObjectPath;

#[derive(Clone, Copy)]
enum Call {
    Add,
    Reauthenticate,
    NeedingReauth,
}

fn storage_account_id() -> account_core::AccountId {
    account_fake::storage_account().id
}

/// The error name a caller gets for `call`.
async fn refused(connection: &zbus::Connection, call: Call) -> String {
    let manager = ManagerProxy::new(connection).await.expect("proxy");
    let error = match call {
        Call::Add => manager
            .add_account("", "", &Details::new())
            .await
            .map(|_| ())
            .expect_err("add"),
        Call::NeedingReauth => manager
            .needing_reauth()
            .await
            .map(|_| ())
            .expect_err("needing"),
        Call::Reauthenticate => {
            let account = AccountProxy::builder(connection)
                .path(ObjectPath::try_from(account_path(&storage_account_id())).expect("path"))
                .expect("path")
                .build()
                .await
                .expect("proxy");
            account
                .reauthenticate("", &Details::new())
                .await
                .expect_err("reauthenticate")
        }
    };
    error_name(&error)
}

#[tokio::test(flavor = "multi_thread")]
async fn each_role_is_refused_what_is_not_its_own() {
    let rig = Rig::start().await;
    let denied = refusal_name(Refusal::Denied);
    // `None` is an unknown sender.
    let rows = [
        (
            Some(("org.bardo.Companion", CallerRole::Agent)),
            Call::Add,
            denied.as_str(),
        ),
        (
            Some(("org.bardo.Companion", CallerRole::Agent)),
            Call::Reauthenticate,
            denied.as_str(),
        ),
        (
            Some(("org.bardo.Actions", CallerRole::AgentLauncher)),
            Call::Add,
            ACCESS_DENIED,
        ),
        (None, Call::Add, ACCESS_DENIED),
        (None, Call::NeedingReauth, ACCESS_DENIED),
        // Only the shell asks who needs signing in again.
        (
            Some(("org.example.App", CallerRole::App)),
            Call::NeedingReauth,
            denied.as_str(),
        ),
        (
            Some(("org.bardo.Settings", CallerRole::Settings)),
            Call::NeedingReauth,
            denied.as_str(),
        ),
        (
            Some(("org.bardo.Companion", CallerRole::Agent)),
            Call::NeedingReauth,
            denied.as_str(),
        ),
    ];
    for (who, call, expected) in rows {
        let connection = match who {
            Some((name, role)) => rig.client_as(caller(name, role)).await,
            None => rig.stranger().await,
        };
        assert_eq!(
            refused(&connection, call).await,
            expected,
            "{:?}",
            who.map(|w| w.1)
        );
    }
    // Nothing was shown to anyone.
    assert_eq!(
        rig.host_log.calls().opened,
        [] as [(
            std::string::String,
            std::string::String,
            std::string::String
        ); 0]
    );
}
