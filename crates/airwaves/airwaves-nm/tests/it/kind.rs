use airwaves_nm::{ActiveState, Connectivity, DeviceState, DeviceType};

#[test]
fn device_kinds_from_the_wire() {
    assert_eq!(DeviceType::from_wire(1), DeviceType::Ethernet);
    assert_eq!(DeviceType::from_wire(2), DeviceType::Wifi);
    assert_eq!(DeviceType::from_wire(29), DeviceType::WireGuard);
    assert_eq!(DeviceType::from_wire(7), DeviceType::Other(7));
}

#[test]
fn device_states_from_the_wire() {
    assert_eq!(DeviceState::from_wire(30), DeviceState::Disconnected);
    assert_eq!(DeviceState::from_wire(60), DeviceState::NeedAuth);
    assert_eq!(DeviceState::from_wire(100), DeviceState::Activated);
    assert_eq!(DeviceState::from_wire(120), DeviceState::Failed);
    assert_eq!(DeviceState::from_wire(55), DeviceState::Other(55));
}

#[test]
fn active_states_and_reach_from_the_wire() {
    assert_eq!(ActiveState::from_wire(2), ActiveState::Activated);
    assert_eq!(ActiveState::from_wire(9), ActiveState::Other(9));
    assert_eq!(Connectivity::from_wire(2), Connectivity::Portal);
    assert_eq!(Connectivity::from_wire(4), Connectivity::Full);
    assert_eq!(Connectivity::from_wire(5), Connectivity::Other(5));
}

#[test]
fn every_value_goes_back_to_the_number_it_came_from() {
    for value in 0..=200 {
        assert_eq!(DeviceType::from_wire(value).to_wire(), value);
        assert_eq!(DeviceState::from_wire(value).to_wire(), value);
        assert_eq!(ActiveState::from_wire(value).to_wire(), value);
        assert_eq!(Connectivity::from_wire(value).to_wire(), value);
    }
}
