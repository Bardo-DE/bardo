//! The words a refused `cua.run.start` gives the person (and the planner, as the app's own
//! reason): one plain sentence, what is wrong and where to put it right when there is a place.
//! No ids, no developer words.

use cua_bus::ControlRefusal;

/// Why a start was refused, in words. `enabled` is the setting now: `Disabled` means the whole
/// feature is off when it is, and that the app is not on the person's list when it is on.
pub(crate) fn words_of(why: ControlRefusal, enabled: bool) -> &'static str {
    match (why, enabled) {
        (ControlRefusal::Disabled, false) => {
            "On-screen control is off. You can turn it on in Settings."
        }
        (ControlRefusal::Disabled, true) => {
            "That app isn't allowed for on-screen control. You can allow it in Settings."
        }
        (ControlRefusal::NotPermitted, _) => "On-screen control wasn't allowed to start.",
        (ControlRefusal::WindowUnknown, _) => "That window isn't open any more.",
        (ControlRefusal::LeaseHeld, _) => "Another on-screen task is already using that window.",
        (ControlRefusal::ModeUnsupported, _) => {
            "This computer can't do on-screen control that way."
        }
        (ControlRefusal::BadTask, _) => "On-screen control needs a window or an app to work in.",
        (ControlRefusal::ModelUnavailable, _) => {
            "The on-screen model isn't available. You can choose one in Settings."
        }
        (ControlRefusal::WrongState | ControlRefusal::UnknownQuestion, _) => {
            "On-screen control isn't ready for that."
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_refusal_has_a_plain_sentence_and_disabled_says_which_setting() {
        for enabled in [false, true] {
            for why in ControlRefusal::ALL {
                let words = words_of(why, enabled);
                assert!(words.ends_with('.'), "{why:?}: {words}");
                for word in ["Disabled", "Lease", "BadTask", "cua", "D-Bus", "org."] {
                    assert!(!words.contains(word), "{why:?}: {words}");
                }
            }
        }
        assert!(words_of(ControlRefusal::Disabled, false).contains("turn it on in Settings"));
        assert!(words_of(ControlRefusal::Disabled, true).contains("allow it in Settings"));
        assert_eq!(
            words_of(ControlRefusal::ModelUnavailable, true),
            "The on-screen model isn't available. You can choose one in Settings."
        );
    }
}
