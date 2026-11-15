use airwaves_bluez::{AgentError, Capability};
use zbus::DBusError;

#[test]
fn capabilities_are_bluez_words() {
    assert_eq!(Capability::KeyboardDisplay.to_wire(), "KeyboardDisplay");
    assert_eq!(Capability::NoInputNoOutput.to_wire(), "NoInputNoOutput");
}

#[test]
fn a_refusal_is_bluez_rejected() {
    let refused = AgentError::Rejected("no".to_owned());
    assert_eq!(refused.name().as_str(), "org.bluez.Error.Rejected");
    let withdrawn = AgentError::Canceled("gone".to_owned());
    assert_eq!(withdrawn.name().as_str(), "org.bluez.Error.Canceled");
}
