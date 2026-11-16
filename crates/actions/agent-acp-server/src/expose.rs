//! Whether the edge may serve at all: the setting `agent.acp.expose`, off unless the file says
//! `true`. The binary checks it before it builds a server, so one that was never switched on has
//! nothing to serve with.

use action_settings::{AgentSettings, Locator, read};

/// Whether the text of a settings file says `agent.acp.expose` is on; anything but `true` is off.
#[must_use]
pub fn exposed(text: &str) -> bool {
    read(text, AgentSettings::default()).value.acp
}

/// Whether the first settings file of the configuration directories says it is on; none is off.
pub fn exposed_in(env: &impl Fn(&str) -> Option<String>) -> bool {
    Locator::from_env(env)
        .read(AgentSettings::default())
        .value
        .acp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_true_exposes() {
        let cases = [
            ("", false),
            ("[agent.acp]\nexpose = true\n", true),
            ("[agent.acp]\nexpose = false\n", false),
            ("[agent.acp]\nexpose = \"maybe\"\n", false),
            ("[agent.acp]\nexpose = \"on\"\n", false),
            ("[agent.mcp]\nexpose = true\n", false),
            ("[agent.acp\nexpose = true", false),
        ];
        for (text, want) in cases {
            assert_eq!(exposed(text), want, "{text:?}");
        }
    }
}
