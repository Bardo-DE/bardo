//! Bus names and object paths.

/// The bus name.
pub const VOICE_BUS: &str = "org.bardo.Voice1";
/// The root object.
pub const VOICE_PATH: &str = "/org/bardo/Voice1";
/// Where utterance objects live, `<prefix>/<n>`.
pub const UTTERANCE_PREFIX: &str = "/org/bardo/Voice1/utterance";
/// Where speech objects live, `<prefix>/<n>`.
pub const SPEECH_PREFIX: &str = "/org/bardo/Voice1/speech";

/// The object path of utterance `n`.
#[must_use]
pub fn utterance_path(n: u64) -> String {
    format!("{UTTERANCE_PREFIX}/{n}")
}

/// The object path of speech `n`.
#[must_use]
pub fn speech_path(n: u64) -> String {
    format!("{SPEECH_PREFIX}/{n}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_valid_object_paths() {
        for p in [utterance_path(3), speech_path(12)] {
            assert!(
                zbus::zvariant::ObjectPath::try_from(p.as_str()).is_ok(),
                "{p}"
            );
        }
    }
}
