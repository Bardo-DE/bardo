//! The person's settings file reaches a router that is already serving: a change in the file
//! applies to the next decision without a restart. The wait is the watch's own event, never a sleep.

use action_core::AgentConfig;
use action_daemon::{apply, apply_next, watch_settings};
use action_fake::fake_router;
use action_settings::AgentSettings;
use permission_core::{AskOrRun, Mode};
use settings_store_fs::WatchState;
use std::path::Path;

/// What the Settings app does: a temp file, then a rename over the settings file.
fn write_atomically(home: &Path, text: &str) {
    let dir = home.join("bardo/actions");
    std::fs::create_dir_all(&dir).unwrap();
    let temp = dir.join("settings.toml.tmp");
    std::fs::write(&temp, text).unwrap();
    std::fs::rename(&temp, dir.join("settings.toml")).unwrap();
}

#[tokio::test]
async fn a_change_in_the_file_applies_to_the_router_without_a_restart() {
    let home = tempfile::tempdir().unwrap();
    let nothing = tempfile::tempdir().unwrap();
    let env = |key: &str| match key {
        "XDG_CONFIG_HOME" => Some(home.path().display().to_string()),
        "XDG_CONFIG_DIRS" => Some(nothing.path().display().to_string()),
        _ => None,
    };
    let router = fake_router(AgentConfig::default()).expect("router");
    let mut watch = watch_settings(
        &action_settings_fs::locator(&env),
        AgentSettings::over(AgentConfig::default()),
    );
    assert_eq!(*watch.state(), WatchState::Live);
    apply(&router, &watch.current());
    assert_eq!(router.agent_config(), AgentConfig::default());

    write_atomically(
        home.path(),
        "[agent]\nstart_mode = \"manual\"\n[agent.auto]\ninside = \"asks\"\n[agent.budget]\nspend_microusd = 7\n",
    );
    while router.agent_config().start_mode == Mode::Auto {
        let loaded = apply_next(&router, &mut watch)
            .await
            .expect("the watch is live");
        assert_eq!(loaded.invalid, vec![]);
    }
    assert_eq!(router.agent_config().start_mode, Mode::Manual);
    assert_eq!(router.agent_config().rows.auto.inside, AskOrRun::Asks);
    assert_eq!(
        router.agent_config().budget.spend,
        Some(model_types::Money::usd(7, 6))
    );

    // A bad value falls back to the base for that key; the good key beside it still applies.
    write_atomically(
        home.path(),
        "[agent]\nstart_mode = \"auto\"\n[agent.budget]\nspend_microusd = -1\n",
    );
    while router.agent_config().start_mode != Mode::Auto {
        apply_next(&router, &mut watch)
            .await
            .expect("the watch is live");
    }
    assert_eq!(
        router.agent_config().budget.spend,
        AgentConfig::default().budget.spend,
        "the refused value is the base's, not the previous file's"
    );
}
