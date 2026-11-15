use turnstile_client::fake::FakeSource;
use turnstile_client::{
    ClientError, Create, DeviceId, DeviceSource, Event, Made, Reason, Sent, Session,
};

#[test]
fn the_fake_records_and_applies_the_real_ceiling() {
    let source = FakeSource::new();
    let mut session = source.open().expect("opens");
    let pad = session.create(&Create::touchpad()).expect("created");
    assert!(matches!(pad, Made::Proxied(DeviceId(0))));
    // The ceiling says no keys outside the table, as the service would.
    let err = session
        .create(&Create::keys(vec![9999]))
        .expect_err("outside the ceiling");
    assert!(
        matches!(err, ClientError::Refused(Reason::BadCaps)),
        "{err:?}"
    );
    assert_eq!(
        session.send(DeviceId(0), &[Event::SYNC]).expect("sent"),
        Sent::Queued
    );
    session.destroy(DeviceId(0)).expect("destroyed");
    let state = source.state();
    assert_eq!(state.opened, 1);
    assert_eq!(state.created.len(), 2);
    assert_eq!(state.sent, [(DeviceId(0), vec![Event::SYNC])]);
    assert_eq!(state.destroyed, [DeviceId(0)]);
}
