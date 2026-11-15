//! Whether the edge may serve at all: the setting `agent.acp.expose`, off unless the file says
//! `on`. The binary checks it before it builds a server, so one that was never switched on has
//! nothing to serve with.

use docket_settings::{AgentSettings, Locator, read};

/// Whether the text of a settings file says `agent.acp.expose` is on; anything but `on` is off.
pub fn exposed(text: &str) -> bool {
    read(text, AgentSettings::default()).value.acp
}

/// Whether the first settings file of the configuration directories says it is on; none is off.
pub fn exposed_in(env: &impl Fn(&str) -> Option<String>) -> bool {
    Locator::from_env(env).read(AgentSettings::default()).value.acp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_on_exposes() {
        let cases = [
            ("", false),
            ("[agent.acp]\nexpose = \"on\"\n", true),
            ("[agent.acp]\nexpose = \"off\"\n", false),
            ("[agent.acp]\nexpose = \"maybe\"\n", false),
            ("[agent.acp]\nexpose = true\n", false),
            ("[agent.mcp]\nexpose = \"on\"\n", false),
            ("[agent.acp\nexpose = \"on\"", false),
        ];
        for (text, want) in cases {
            assert_eq!(exposed(text), want, "{text:?}");
        }
    }
}
