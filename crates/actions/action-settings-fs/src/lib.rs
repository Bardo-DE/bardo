//! docket's settings on disk: where the settings file is, the records beside the agents, and the
//! writes of `agents.toml`. What the settings mean, and every text edit, is `action-settings`.

mod agents;

pub use agents::{
    DiskRecords, read_agents_in, write_agent_add, write_agent_choice, write_agent_remove,
    write_agent_rewind,
};

use action_settings::SETTINGS_FILE;
use settings_store_fs::Locator;

/// Where the settings file is.
///
/// `$XDG_CONFIG_HOME/bardo/actions/settings.toml`, then the same under each of `$XDG_CONFIG_DIRS`.
/// Read it with
/// [`Locator::read`](settings_store_fs::Locator::read)`(base, action_settings::read)`.
pub fn locator(env: &impl Fn(&str) -> Option<String>) -> Locator {
    Locator::from_env(env, SETTINGS_FILE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use action_settings::{AgentSettings, read};
    use settings_store::Loaded;

    fn micro_usd(n: i64) -> model_provider::Money {
        model_provider::Money::usd(rust_decimal::Decimal::new(n, 6))
    }

    #[test]
    fn the_locator_reads_the_first_file_of_the_configuration_directories() {
        let home = tempfile::tempdir().unwrap();
        let etc = tempfile::tempdir().unwrap();
        for (dir, text) in [
            (&home, "[agent.budget]\nspend_microusd = 5\n"),
            (
                &etc,
                "[agent.budget]\nspend_microusd = 9\n[agent.mcp]\nexpose = true\n",
            ),
        ] {
            std::fs::create_dir_all(dir.path().join("bardo/actions")).unwrap();
            std::fs::write(dir.path().join("bardo").join(SETTINGS_FILE), text).unwrap();
        }
        let env = |k: &str| match k {
            "XDG_CONFIG_HOME" => Some(home.path().display().to_string()),
            "XDG_CONFIG_DIRS" => Some(etc.path().display().to_string()),
            _ => None,
        };
        let found = locator(&env);
        assert_eq!(found.watch_dir(), Some(home.path().join("bardo/actions")));
        let value = found.read(AgentSettings::default(), read).value;
        assert_eq!(value.agent.budget.spend, Some(micro_usd(5)));
        // The first file wins whole: the later directory's other keys are not merged in.
        assert!(!value.expose);
        let none = locator(&|_: &str| None::<String>);
        assert_eq!(
            none.read(AgentSettings::default(), read),
            Loaded::clean(AgentSettings::default())
        );
    }
}
