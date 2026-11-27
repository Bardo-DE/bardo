//! `account-rig-secrets` as a process: a fake `org.freedesktop.secrets` on a private bus, and
//! accountd's own `Oo7Secrets` (the real client, session negotiation and all) storing,
//! refusing to overwrite, reading back and deleting through it, from a child process whose
//! only bus is the private one (this test executable run again, `accept::secrets::oo7_child`). The
//! service ends cleanly on SIGTERM sent to its PID.

use account_core::AccountId;
use account_core::{Credential, SecretKey, SecretPurpose, SecretText};
use account_secrets::{Oo7Secrets, Secrets, SecretsError};
use base_testkit::PrivateBus;
use base_testkit::TestEnv;
use base_time::UnixSeconds;
use futures_util::StreamExt;
use std::process::Stdio;

const CHILD: &str = "RIG_OO7_CHILD";

fn key(account: &str) -> SecretKey {
    SecretKey {
        account: AccountId::parse(account).expect("id"),
        purpose: SecretPurpose::Password,
    }
}

fn credential(refresh: &str) -> Credential {
    Credential::OAuth {
        access: SecretText::new("an-access-token"),
        refresh: SecretText::new(refresh),
        expires_at: UnixSeconds(5),
        granted: account_core::GrantedScopes::default(),
    }
}

/// The client half: runs only in the child process the other test starts, where the session
/// bus is the private one. Anywhere else it is a test of nothing and passes.
#[tokio::test(flavor = "multi_thread")]
async fn oo7_child() {
    if std::env::var_os(CHILD).is_none() {
        return;
    }
    let store = Oo7Secrets;
    let one = key("rig-account");
    assert_eq!(store.get(&one).await, Err(SecretsError::Missing));
    store
        .put(&one, &credential("RIG-REFRESH-1"))
        .await
        .expect("put");
    assert_eq!(
        store.get(&one).await.expect("get"),
        credential("RIG-REFRESH-1")
    );
    assert_eq!(
        store
            .put_if_absent(&one, &credential("RIG-REFRESH-2"))
            .await,
        Ok(false)
    );
    assert_eq!(
        store.get(&one).await.expect("get"),
        credential("RIG-REFRESH-1")
    );
    store
        .put(&one, &credential("RIG-REFRESH-3"))
        .await
        .expect("replace");
    assert_eq!(
        store.get(&one).await.expect("get"),
        credential("RIG-REFRESH-3")
    );
    let two = key("other-account");
    assert_eq!(
        store
            .put_if_absent(&two, &credential("RIG-REFRESH-4"))
            .await,
        Ok(true)
    );
    store.delete(&one).await.expect("delete");
    assert_eq!(store.get(&one).await, Err(SecretsError::Missing));
    store
        .delete_account(&AccountId::parse("other-account").expect("id"))
        .await
        .expect("wipe");
    assert_eq!(store.get(&two).await, Err(SecretsError::Missing));
}

#[tokio::test(flavor = "multi_thread")]
async fn accountds_oo7_secrets_stores_and_reads_back_through_the_fake_and_it_ends_on_sigterm() {
    let bus = PrivateBus::start();
    let env = TestEnv::new("rig-secrets").with_bus(&bus);
    let mut service = env
        .spawn(env!("CARGO_BIN_EXE_account-rig-secrets"), [] as [&str; 0])
        .expect("the rig binary starts");
    let probe = bus.connect().await;
    let dbus = zbus::fdo::DBusProxy::new(&probe).await.expect("proxy");
    let name: zbus::names::BusName<'_> = "org.freedesktop.secrets".try_into().expect("name");
    // Subscribe before looking, so an owner that appears in between is not missed.
    let mut owners = dbus
        .receive_name_owner_changed_with_args(&[(0, "org.freedesktop.secrets")])
        .await
        .expect("owner changes");
    tokio::time::timeout(base_testkit::GENEROUS, async {
        while !dbus.name_has_owner(name.clone()).await.unwrap_or(false) {
            owners.next().await.expect("the bus stays up");
        }
    })
    .await
    .expect("the service takes org.freedesktop.secrets");

    let exe = std::env::current_exe().expect("this test binary");
    let mut child = env.command(exe);
    let output = tokio::task::spawn_blocking({
        move || {
            child
                .env(CHILD, "1")
                .args([
                    "--exact",
                    "accept::secrets::oo7_child",
                    "--nocapture",
                    "--test-threads=1",
                ])
                .stdin(Stdio::null())
                .output()
                .expect("the child test runs")
        }
    })
    .await
    .expect("join");
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success(), "{said}");
    assert!(said.contains("1 passed"), "the child test ran: {said}");

    let pid =
        rustix::process::Pid::from_raw(i32::try_from(service.pid()).expect("pid")).expect("a pid");
    rustix::process::kill_process(pid, rustix::process::Signal::TERM).expect("SIGTERM");
    let status = service.child_mut().wait().expect("the process ends");
    assert!(status.success(), "{status:?}");
}
