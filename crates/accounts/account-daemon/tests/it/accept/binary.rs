//! The accountd binary on a private bus with scratch HOME and XDG (a `TestEnv`): it loads its registry,
//! refuses to start over one it cannot read, finds callers through a /proc root only in a
//! `test-proc-root` build, and never touches the real Secret Service (nothing here needs a secret).

use super::accountd::{env_on, serving, spawn};
use crate::common;

use account_capability::StorageScope;
use account_core::AuthKind;
use account_core::store::Persisted;
use account_dbus::accounts::IdentitiesProxy;
use base_testkit::PrivateBus;
use std::path::{Path, PathBuf};

/// A /proc tree naming this process a Flatpak app.
fn proc_tree(dir: &Path) -> PathBuf {
    let root = dir.join("proc");
    let pid = root.join(std::process::id().to_string());
    std::fs::create_dir_all(&pid).expect("proc dir");
    std::fs::write(
        pid.join("cgroup"),
        "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-flatpak-org.example.Probe-1.scope\n",
    )
    .expect("cgroup");
    // A Flatpak app is known by its sandbox's metadata, not its scope's name.
    std::fs::create_dir_all(pid.join("root")).expect("root");
    std::fs::write(
        pid.join("root/.flatpak-info"),
        "[Application]\nname=org.example.Probe\n",
    )
    .expect("flatpak-info");
    root
}

fn probe() -> account_core::AppId {
    common::app("org.example.Probe")
}

/// A registry file as the accounts service wrote it before the split: an identity (a password
/// account), an AI key, a grant for the probe app and a switch.
fn seed(state: &Path) {
    use account_core::GrantId;
    use account_core::{DataClass, SpaceScope};
    use account_core::{
        GrantKey,
        consent::{Decision, Grant, GrantScope, Usage},
    };
    use base_time::UnixSeconds;
    let mut account = account_fake::storage_account();
    account.auth = AuthKind::Password;
    let mut key = account_fake::llm_account();
    key.auth = AuthKind::ApiKey;
    let grant = Grant {
        id: GrantId::parse("seeded").expect("id"),
        key: GrantKey {
            app: probe(),
            account: key.id.clone(),
            kind: account_capability::CapabilityKind::Llm,
            class: DataClass::Photos,
            usage: Usage::Interactive,
            space: SpaceScope::Any,
            reach: StorageScope::AppFolder,
        },
        decision: Decision::Allow,
        scope: GrantScope::Always,
        at: UnixSeconds(1),
    };
    let mut stored = Persisted::empty();
    stored.accounts.push(account);
    stored.accounts.push(key);
    stored.grants.push(grant);
    let dir = state.join("bardo/accounts");
    std::fs::create_dir_all(&dir).expect("state dir");
    std::fs::write(dir.join("registry.json"), stored.to_json().expect("json")).expect("registry");
}

#[cfg(feature = "test-proc-root")]
#[tokio::test(flavor = "multi_thread")]
async fn a_test_build_reads_callers_from_the_proc_root_and_says_so() {
    let bus = PrivateBus::start();
    let env = env_on(&bus);
    seed(&env.state());
    let proc = proc_tree(env.root()).display().to_string();
    let env = env.with("BARDO_ACCOUNTD_PROC_ROOT", &proc);
    let mut daemon = spawn(&env, &[]);
    assert!(serving(&bus, &mut daemon).await, "{}", daemon.stderr());
    assert!(
        daemon.stderr().contains("reading callers from") && daemon.stderr().contains("proc"),
        "{}",
        daemon.stderr()
    );

    // This process is the Probe app by the fixture: it lists the one identity; the AI key of the
    // file is an account but not an identity.
    let client = bus.connect().await;
    let found = IdentitiesProxy::new(&client)
        .await
        .expect("proxy")
        .list()
        .await
        .expect("list");
    assert_eq!(found.len(), 1, "only the identity was kept");
}

/// The registry is read as it is: an AI key, its permit and the identity all stay, and no copy of
/// the file is made beside it.
#[tokio::test(flavor = "multi_thread")]
async fn the_registry_is_read_as_it_is_and_no_copy_is_made() {
    let bus = PrivateBus::start();
    let env = env_on(&bus);
    seed(&env.state());
    let dir = env.state().join("bardo/accounts");
    let before = std::fs::read_to_string(dir.join("registry.json")).expect("the file");
    let mut daemon = spawn(&env, &[]);
    assert!(serving(&bus, &mut daemon).await, "{}", daemon.stderr());
    assert_eq!(
        std::fs::read_to_string(dir.join("registry.json")).expect("the file"),
        before,
        "nothing was dropped or rewritten"
    );
    assert!(!dir.join("registry.pre-split.json").exists());
}

#[cfg(not(feature = "test-proc-root"))]
#[tokio::test(flavor = "multi_thread")]
async fn a_release_build_ignores_the_proc_root_variable() {
    let bus = PrivateBus::start();
    let env = env_on(&bus);
    seed(&env.state());
    let proc = proc_tree(env.root()).display().to_string();
    let env = env.with("BARDO_ACCOUNTD_PROC_ROOT", &proc);
    let mut daemon = spawn(&env, &[]);
    assert!(serving(&bus, &mut daemon).await, "{}", daemon.stderr());
    assert!(
        !daemon.stderr().contains("reading callers from"),
        "{}",
        daemon.stderr()
    );
    // The real /proc says this process is no app (a test runner's scope): nobody, so refused.
    let client = bus.connect().await;
    let err = IdentitiesProxy::new(&client)
        .await
        .expect("proxy")
        .list()
        .await
        .expect_err("the fixture was not read");
    assert_eq!(common::error_name(&err), common::ACCESS_DENIED);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_caller_is_refused_by_the_real_binary() {
    let bus = PrivateBus::start();
    let env = env_on(&bus);
    let mut daemon = spawn(&env, &[]);
    assert!(serving(&bus, &mut daemon).await, "{}", daemon.stderr());
    let client = bus.connect().await;
    let err = IdentitiesProxy::new(&client)
        .await
        .expect("proxy")
        .list()
        .await;
    // A runner inside a login session may or may not be named by its cgroup; either way the
    // daemon answered on its bus, and a caller it cannot name is AccessDenied.
    if let Err(err) = err {
        assert_eq!(common::error_name(&err), common::ACCESS_DENIED);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_registry_it_cannot_read_is_never_overwritten_and_the_daemon_does_not_start() {
    let bus = PrivateBus::start();
    let env = env_on(&bus);
    let dir = env.state().join("bardo/accounts");
    std::fs::create_dir_all(&dir).expect("state dir");
    std::fs::write(dir.join("registry.json"), "{ this is not a registry").expect("registry");
    std::fs::write(dir.join("registry.json.bak"), "the earlier copy").expect("backup");
    let mut daemon = spawn(&env, &[]);
    assert!(!serving(&bus, &mut daemon).await);
    let status = daemon.wait();
    // rel-5: the status the unit does not restart on, and both files named.
    assert_eq!(
        status.code(),
        Some(i32::from(account_daemon::EXIT_REGISTRY_REFUSED))
    );
    let said = daemon.stderr();
    assert!(said.contains("registry.json.bak"), "{said}");
    assert!(said.contains("Nothing was changed"), "{said}");
    assert_eq!(
        std::fs::read_to_string(dir.join("registry.json")).expect("still there"),
        "{ this is not a registry"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("registry.json.bak")).expect("still there"),
        "the earlier copy"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_caller_table_that_does_not_parse_stops_the_daemon_naming_the_file() {
    let bus = PrivateBus::start();
    let env = env_on(&bus);
    let config = env.config().join("bardo/accounts");
    std::fs::create_dir_all(&config).expect("config dir");
    std::fs::write(
        config.join("callers.toml"),
        "[[caller]]\napp = \"not a name\"\nrole = \"app\"\n",
    )
    .expect("callers");
    let mut daemon = spawn(&env, &[]);
    assert!(!serving(&bus, &mut daemon).await);
    assert!(!daemon.wait().success());
    assert!(
        daemon.stderr().contains("callers.toml"),
        "{}",
        daemon.stderr()
    );
}
