use airwaves_bluez::AgentError;
use zbus::DBusError;

#[test]
fn a_refusal_is_bluez_rejected() {
    let refused = AgentError::Rejected("no".to_owned());
    assert_eq!(refused.name().as_str(), "org.bluez.Error.Rejected");
    let withdrawn = AgentError::Canceled("gone".to_owned());
    assert_eq!(withdrawn.name().as_str(), "org.bluez.Error.Canceled");
}
