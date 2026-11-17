//! The person's settings file reaches a running supervisor: a change in the file applies to the
//! next start without a restart. The wait is the watch's own event, never a sleep.

use crate::support::supervisor;
use computer_use_bus::{ComputerUseCaller, ControlRefusal};
use computer_use_daemon::{Apps, ComputerUseSettings, locator, watch_settings};
use computer_use_desktop::BackendKind;
use computer_use_fake::FakePdp;
use computer_use_fake::scene::{click, finish, reply, task};
use settings_store_fs::WatchState;
use std::path::Path;

/// What the Settings app does: a temp file, then a rename over the settings file.
fn write_atomically(home: &Path, text: &str) {
    let dir = home.join("bardo/computer-use");
    std::fs::create_dir_all(&dir).unwrap();
    let temp = dir.join("settings.toml.tmp");
    std::fs::write(&temp, text).unwrap();
    std::fs::rename(&temp, dir.join("settings.toml")).unwrap();
}

#[tokio::test]
async fn a_change_in_the_file_applies_to_the_next_start_without_a_restart() {
    let home = tempfile::tempdir().unwrap();
    let nothing = tempfile::tempdir().unwrap();
    let env = |key: &str| match key {
        "XDG_CONFIG_HOME" => Some(home.path().display().to_string()),
        "XDG_CONFIG_DIRS" => Some(nothing.path().display().to_string()),
        _ => None,
    };
    let sup = supervisor(
        BackendKind::KwinStock,
        vec![reply(click()), reply(finish())],
        FakePdp::allow_all(),
    );
    // What the daemon does at start: the file over the configuration, no app on until it says.
    let base = ComputerUseSettings {
        config: sup.settings(),
        apps: Apps::Listed(Vec::new()),
    };
    let mut watch = watch_settings(&locator(&env), base);
    assert_eq!(*watch.state(), WatchState::Live);
    sup.apply_settings(&watch.current().value);
    assert_eq!(
        sup.start(ComputerUseCaller::Companion, task()).await,
        Err(ControlRefusal::Disabled),
        "no app is on until the person lists one"
    );

    let app = task().app;
    write_atomically(
        home.path(),
        &format!(
            "[computer use]\nenabled = true\n[[computer_use.apps]]\napp = \"{}\"\nallow = \"ask\"\n",
            app.as_str()
        ),
    );
    while sup.apps().rule_for(&app).is_none() {
        let loaded = watch.changed().await.expect("the watch is live");
        sup.adopt(&loaded);
        assert_eq!(loaded.invalid, vec![]);
    }
    let started = sup.start(ComputerUseCaller::Companion, task()).await;
    assert!(started.is_ok(), "{started:?}");
    assert_eq!(
        computer_use_daemon::reach_json(sup.settings().enabled),
        r#"{"computeruse.run.start":"offered"}"#,
        "the live reach follows the file"
    );

    // Off again, in the file: the next start is refused, the daemon never restarted.
    write_atomically(home.path(), "[computer use]\nenabled = false\n");
    while sup.settings().enabled {
        let loaded = watch.changed().await.expect("the watch is live");
        sup.adopt(&loaded);
    }
    assert_eq!(
        sup.start(ComputerUseCaller::Companion, task()).await,
        Err(ControlRefusal::Disabled)
    );
    assert_eq!(
        computer_use_daemon::reach_json(sup.settings().enabled),
        r#"{"computeruse.run.start":"hidden"}"#
    );
}

#[tokio::test]
async fn a_bad_value_in_the_file_leaves_the_running_value_and_is_reported() {
    let home = tempfile::tempdir().unwrap();
    let nothing = tempfile::tempdir().unwrap();
    let env = |key: &str| match key {
        "XDG_CONFIG_HOME" => Some(home.path().display().to_string()),
        "XDG_CONFIG_DIRS" => Some(nothing.path().display().to_string()),
        _ => None,
    };
    let sup = supervisor(BackendKind::KwinStock, vec![], FakePdp::allow_all());
    let base = ComputerUseSettings {
        config: sup.settings(),
        apps: Apps::Unrestricted,
    };
    let mut watch = watch_settings(&locator(&env), base);
    write_atomically(home.path(), "[computer_use.budget]\nspend_microusd = -1\n");
    let loaded = watch.changed().await.expect("the watch is live");
    sup.adopt(&loaded);
    assert_eq!(loaded.invalid.len(), 1);
    assert_eq!(loaded.invalid[0].path, "computer_use.budget.spend_microusd");
    assert_eq!(
        sup.settings().budget.spend,
        model_provider::Money::usd(rust_decimal::Decimal::new(0, 6))
    );
}

#[tokio::test]
async fn a_settings_file_that_cannot_be_read_is_reported_and_the_next_directory_answers() {
    let blocked = tempfile::tempdir().unwrap();
    let etc = tempfile::tempdir().unwrap();
    // A directory stands in for a file that is there and cannot be read.
    std::fs::create_dir_all(blocked.path().join("bardo/computer-use/settings.toml")).unwrap();
    write_atomically(etc.path(), "[computer_use.budget]\nspend_microusd = 1900\n");
    let env = |key: &str| match key {
        "XDG_CONFIG_HOME" => Some(blocked.path().display().to_string()),
        "XDG_CONFIG_DIRS" => Some(etc.path().display().to_string()),
        _ => None,
    };
    let sup = supervisor(BackendKind::KwinStock, vec![], FakePdp::allow_all());
    let base = ComputerUseSettings {
        config: sup.settings(),
        apps: Apps::Unrestricted,
    };
    let loaded = watch_settings(&locator(&env), base).current();
    assert_eq!(
        loaded.value.config.budget.spend,
        model_provider::Money::usd(rust_decimal::Decimal::new(1900, 6))
    );
    assert_eq!(loaded.invalid.len(), 1);
    assert!(loaded.invalid[0].reason.starts_with("could not be read"));
}
