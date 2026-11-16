//! The Bluetooth page over a `BlueZ` that answers from memory: the switch, My Devices and Nearby
//! Devices, connecting, pairing with a number to compare, forgetting after a question, and looking
//! for devices only while the page is open.

use crate::support::Sandbox;
use crate::support::drive::{click, page_of, settle, showing, sidebar_page, title_of};
use settings_app_model::{
    BtAddress, BtDevice, BtKind, BtLink, BtStatus, PairingAnswer, PairingAsk, PairingCode,
    PairingQuestion,
};
use settings_app_system::bluetooth::FakeBluetooth;
use settings_app_ui::Opening;
use ui_harness::Harness;

const PAGE: &str = "[data-system=\"bluetooth\"]";
const PAIRING: &str = "[data-system=\"pairing\"]";

fn bluetooth() -> Opening {
    Opening {
        page: Some(settings_app_model::PageSlug("bluetooth".to_owned())),
        query: String::new(),
    }
}

fn address(last: u8) -> BtAddress {
    BtAddress(format!("AA:BB:CC:DD:EE:{last:02X}"))
}

fn device(last: u8, name: &str, kind: BtKind, link: BtLink, paired: bool) -> BtDevice {
    BtDevice {
        address: address(last),
        name: name.to_owned(),
        kind,
        link,
        paired,
        battery: None,
    }
}

/// Paired with a connected mouse and idle headphones, and a nearby game pad it has not paired with.
fn status(on: bool) -> BtStatus {
    BtStatus::new(
        on,
        vec![
            device(1, "Quiet Mouse", BtKind::Mouse, BtLink::Connected, true),
            device(
                2,
                "Big Headphones",
                BtKind::Audio,
                BtLink::Disconnected,
                true,
            ),
            device(
                3,
                "Nearby Pad",
                BtKind::Gamepad,
                BtLink::Disconnected,
                false,
            ),
        ],
    )
}

fn world(name: &str, service: &FakeBluetooth) -> (Sandbox, Harness) {
    let sandbox = Sandbox::new(name).with_bluetooth(service.clone());
    let harness = sandbox.harness(bluetooth());
    (sandbox, harness)
}

fn button(verb: &str, name: &str) -> String {
    format!("[aria-label=\"{verb} {name}\"]")
}

#[test]
fn the_page_is_second_in_the_sidebar_and_says_plainly_when_nothing_serves_bluetooth() {
    let sandbox = Sandbox::new("bluetooth-absent");
    let mut harness = sandbox.harness(bluetooth());
    showing(
        &mut harness,
        "the unavailable row",
        "[data-system=\"unavailable\"]",
    );
    assert_eq!(page_of(&harness).as_deref(), Some("bluetooth"));
    assert_eq!(title_of(&harness).as_deref(), Some("Bluetooth"));
    assert_eq!(
        harness.text_of(&sidebar_page("bluetooth")).as_deref(),
        Some("Bluetooth")
    );
    let row = harness
        .text_of("[data-system=\"unavailable\"]")
        .unwrap_or_default();
    assert!(
        row.contains("Bluetooth isn\u{2019}t available right now."),
        "{row}"
    );
}

#[test]
fn devices_are_listed_as_mine_and_nearby_with_whether_they_are_connected() {
    let service = FakeBluetooth::answering(status(true));
    let (_sandbox, mut harness) = world("bluetooth-list", &service);
    showing(&mut harness, "the page", PAGE);
    showing(
        &mut harness,
        "the devices",
        "[aria-label=\"Connect Nearby Pad\"]",
    );
    let html = harness.html();
    assert!(html.contains("My Devices") && html.contains("Nearby Devices"));
    let mine = harness.text_of(PAGE).unwrap_or_default();
    assert!(
        mine.contains("Connected") && mine.contains("Not Connected"),
        "{mine}"
    );
    assert_eq!(harness.count(&button("Disconnect", "Quiet Mouse")), 1);
    assert_eq!(harness.count(&button("Connect", "Big Headphones")), 1);
    assert_eq!(harness.count(&button("Forget", "Quiet Mouse")), 1);
    assert_eq!(
        harness.count(&button("Forget", "Nearby Pad")),
        0,
        "only a paired device is forgotten"
    );
}

#[test]
fn a_nearby_device_pairs_after_the_person_confirms_the_number() {
    let service = FakeBluetooth::answering(status(true));
    let (_sandbox, mut harness) = world("bluetooth-pair", &service);
    showing(
        &mut harness,
        "the nearby device",
        "[aria-label=\"Connect Nearby Pad\"]",
    );
    settle(&mut harness, "a session", |_| service.looking() == 1);

    let mut answered = service
        .ask(PairingQuestion {
            address: address(3),
            name: "Nearby Pad".to_owned(),
            ask: PairingAsk::Compare(PairingCode(4_217)),
        })
        .expect("a session is open");
    showing(&mut harness, "the question", PAIRING);
    let sheet = harness.text_of(PAIRING).unwrap_or_default();
    assert!(
        sheet.contains("004217") && sheet.contains("\u{201c}Nearby Pad\u{201d}"),
        "{sheet}"
    );
    assert!(
        answered.try_recv().is_err(),
        "nothing is answered until the person does"
    );
    click(&mut harness, &format!("{PAIRING} [aria-label=\"Connect\"]"));
    assert_eq!(answered.try_recv().ok(), Some(PairingAnswer::Yes));
    settle(&mut harness, "the sheet closed", |h| h.count(PAIRING) == 0);
}
