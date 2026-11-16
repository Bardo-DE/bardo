//! The shell (`SheetHost`) and accounts that need signing in again: it lists them, reads any
//! account, is told when one is refused or well again, and signs one in again without a grant;
//! no other role does any of it.

use crate::common;

use account_callers::CallerRole;
use account_core::AccountState;
use account_dbus::Details;
use account_dbus::accounts::{AccountProxy, ManagerProxy};
use common::*;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread")]
async fn the_shell_is_told_of_a_refusal_and_a_recovery_and_reads_the_account() {
    let (_dir, options) = poking_options();
    let rig = Rig::start_with(options, SheetHost::quiet()).await;
    let mail = mail_account();
    let shell = rig
        .client_as(caller("org.bardo.Shell", CallerRole::SheetHost))
        .await;
    let manager = ManagerProxy::new(&shell).await.expect("proxy");
    assert!(manager.needing_reauth().await.expect("list").is_empty());
    // An app that has called accountd does not hear.
    let other = rig.client("org.bardo.Mail").await;
    account_dbus::accounts::IdentitiesProxy::new(&other)
        .await
        .expect("proxy")
        .list()
        .await
        .expect("list");
    let mut other_hears = listen(&other).await;
    let mut shell_changes = property_changes(&shell).await;

    assert!(
        rig.service
            .set_state(&mail.id, AccountState::NeedsReauth)
            .await
    );
    // A publish tells whoever is to be told.
    publish(&rig).await;
    // An absence, proved in a short window by design: a slow machine can only make it pass.
    assert!(
        heard(&mut other_hears, Duration::from_millis(300))
            .await
            .is_empty()
    );
    assert_eq!(
        heard_names(&mut shell_changes, &["PropertiesChanged"]).await,
        ["PropertiesChanged"],
        "the shell is told `State` changed"
    );

    let listed = manager.needing_reauth().await.expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].1, mail.label.0);
    let account = AccountProxy::builder(&shell)
        .path(listed[0].0.clone())
        .expect("path")
        .build()
        .await
        .expect("account");
    assert_eq!(account.state().await.expect("state"), "needs_reauth");
    assert_eq!(account.label().await.expect("label"), mail.label.0);
    // The same object does not exist for an app.
    let hidden = AccountProxy::builder(&other)
        .path(listed[0].0.clone())
        .expect("path")
        .build()
        .await
        .expect("account");
    assert!(hidden.state().await.is_err());
    // No grant is asked of the shell to sign the account in again.
    account
        .reauthenticate("", &Details::new())
        .await
        .expect("a request");

    // The sign-in the shell just started (it needs no grant) may finish first and make the
    // account well itself; either way it is well once this returns.
    let _ = rig.service.set_state(&mail.id, AccountState::Ok).await;
    eventually("the account is well", || {
        rig.service
            .registry()
            .accounts
            .iter()
            .any(|a| a.id == mail.id && a.state == AccountState::Ok)
    })
    .await;
    publish(&rig).await;
    assert_eq!(
        heard_names(&mut shell_changes, &["PropertiesChanged"]).await,
        ["PropertiesChanged"],
        "the shell is told the account is well again"
    );
}

/// The `PropertiesChanged` signals a connection receives.
async fn property_changes(connection: &zbus::Connection) -> zbus::MessageStream {
    let rule = zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface("org.freedesktop.DBus.Properties")
        .expect("interface")
        .member("PropertiesChanged")
        .expect("member")
        .build();
    zbus::MessageStream::for_match_rule(rule, connection, None)
        .await
        .expect("stream")
}
