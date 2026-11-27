//! A launch in a real systemd user scope: the person's running user manager.

use std::path::PathBuf;
use std::time::Duration;

use shell_launcher::{AppId, DesktopAction};
use shell_model::apps::{App, AppIcon, AppIndex, AppsEvent, DbusActivation, Listing, Terminal};
use shell_model::launch::{LaunchCommand, LaunchRequest, LaunchService};
use shell_model::notifications::NotificationsService;
use shell_services::env::Env;
use shell_services::launch::LaunchLinks;
use shell_services::notifications::Answers;
use shell_services::service::spawn;

fn index_with(id: &str, exec: &str) -> AppIndex {
    let app = App {
        id: AppId(id.to_owned()),
        name: id.to_owned(),
        generic_name: String::new(),
        comment: String::new(),
        exec: exec.to_owned(),
        icon: AppIcon::None,
        startup_wm_class: None,
        flatpak: None,
        bus_names: Vec::new(),
        intents_app: None,
        renamed_from: Vec::new(),
        terminal: Terminal::No,
        activation: DbusActivation::Exec,
        actions: Vec::new(),
        mime_types: Vec::new(),
        categories: Vec::new(),
        keyboard_profile: None,
        keywords: Vec::new(),
        listing: Listing::Shown,
        path: PathBuf::from(format!("/usr/share/applications/{id}.desktop")),
    };
    shell_model::apps::step(AppIndex::default(), AppsEvent::Scanned(vec![app]))
}

/// Exercises the real `systemd-run --user --scope` path on this machine: `/bin/true` starts,
/// `Launches` reports `Started` with a well-formed unit, and once the process exits the same
/// unit is reported `Exited`. Needs a user systemd (`systemctl --user status`).
#[tokio::test]
async fn a_launch_runs_in_a_real_systemd_scope_and_exits() {
    let (_apps_tx, apps_rx) =
        tokio::sync::watch::channel(index_with("shell-launch-test", "/bin/true"));
    let rt = tokio::runtime::Handle::current();
    let scratch = base_testkit::Scratch::new("launch-scope");
    let mut env = Env::isolated(scratch.path());
    env.dirs.runtime = std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from);
    let notifications = spawn::<NotificationsService>(&rt, &env, Answers::default()).control;
    let launch = spawn::<LaunchService>(
        &rt,
        &env,
        LaunchLinks {
            apps: apps_rx,
            notifications,
        },
    );
    let (mut launches, control) = (launch.state, launch.control);

    control
        .send(LaunchCommand::Launch(LaunchRequest {
            app: AppId("shell-launch-test".to_owned()),
            action: DesktopAction::Default,
            files: Vec::new(),
            uris: Vec::new(),
            token: None,
        }))
        .unwrap_or_else(|e| panic!("{e}"));

    let started = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            launches.changed().await.unwrap_or_else(|e| panic!("{e}"));
            if let Some(scope) = launches.borrow().scopes.first() {
                return scope.unit.clone();
            }
        }
    })
    .await
    .unwrap_or_else(|_| panic!("no Started outcome within 5s"));
    eprintln!("bardo-shell: live launch started unit {}", started.0);
    assert!(started.0.starts_with("app-bardo-"));
    assert!(
        std::path::Path::new(&started.0)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("scope"))
    );

    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if launches.borrow().scopes.is_empty() {
                return;
            }
            launches.changed().await.unwrap_or_else(|e| panic!("{e}"));
        }
    })
    .await
    .unwrap_or_else(|_| panic!("the scope never reported Exited within 10s"));
    eprintln!("bardo-shell: live launch unit {} exited", started.0);
}
