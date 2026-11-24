//! A call goes to the program that was checked, not to whoever holds the service's name when the
//! call is made: the Privacy page over a fake portal that loses the name to a second one the
//! moment the window has asked who owns it. (The Spaces registry's own case is tested in
//! settings-app-system.)

use crate::support::Sandbox;
use base_testkit::PrivateBus;
use crate::support::drive::click;
use crate::support::drive::settle_on_threads;
use crate::support::handover::HandsOver;
use crate::support::portal;
use settings_app_link::bus::Vouch;
use settings_app_model::PageSlug;
use settings_app_ui::Opening;
use std::sync::atomic::Ordering;
use zbus::fdo::DBusProxy;
use zbus::names::BusName;

/// The unique name that owns `name` now.
fn owner(rt: &tokio::runtime::Runtime, via: &zbus::Connection, name: &str) -> String {
    rt.block_on(async {
        let bus = DBusProxy::new(via).await.expect("the bus");
        let name = BusName::try_from(name).expect("a name");
        bus.get_name_owner(name)
            .await
            .expect("an owner")
            .to_string()
    })
}

#[test]
fn an_app_is_taken_back_at_the_portal_that_was_checked() {
    const MEET: &str = "org.example.Meet";
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("a runtime for the fakes");
    let bus = PrivateBus::start();
    let apps = [portal::Remembered {
        app: MEET,
        label: "Meet",
        mode: 2,
        granted: 1,
    }];
    let original = rt.block_on(portal::serve(&bus, &apps));
    let successor = rt.block_on(portal::serve_unnamed(&bus, &apps));
    let (check, handover) = HandsOver::new(
        portal::NAME,
        original.connection.clone(),
        successor.connection.clone(),
    );
    let sandbox = Sandbox::new("owner-portal").on_bus_vouched(bus.address(), Vouch::of(check));
    sandbox.write_application(
        MEET,
        "[Desktop Entry]\nType=Application\nName=Video Meet\nExec=flatpak run org.example.Meet\nX-Flatpak=org.example.Meet\n",
    );
    let mut harness = sandbox.harness(Opening {
        page: Some(PageSlug("privacy".to_owned())),
        query: String::new(),
    });
    let remove = "[aria-label=\"Remove Video Meet\"]";
    settle_on_threads(&mut harness, "the list", |h| h.count(remove) == 1);
    handover.store(true, Ordering::SeqCst);
    click(&mut harness, remove);
    settle_on_threads(&mut harness, "the removal", |_| {
        !original.revoked().is_empty()
    });
    assert_eq!(
        owner(&rt, &original.connection, portal::NAME),
        successor
            .connection
            .unique_name()
            .expect("a unique name")
            .to_string(),
        "the name did change hands after the check"
    );
    assert_eq!(original.revoked(), [MEET], "the program checked was asked");
    assert!(
        successor.revoked().is_empty(),
        "the program that took the name after the check was asked nothing"
    );
}
