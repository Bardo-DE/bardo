//! Joining a new secured network from the Wi-Fi page: the password sheet and the join it sends.

use super::{JOINED, PASSWORD, join_button, listed, ssid, status, world};
use crate::support::drive::{click, keys, settle, showing, type_into};
use crate::support::wifi::{Asked, Wifi};
use detent_model::{Passphrase, WifiRadio};

#[test]
fn a_new_secured_network_asks_for_its_password_and_joins_with_it() {
    let service = Wifi::answering(status(WifiRadio::On));
    let (_sandbox, mut harness) = world("wifi-join-secured", &service);
    listed(&mut harness);
    click(&mut harness, &join_button("Neighbour"));
    showing(&mut harness, "the password sheet", PASSWORD);
    assert_eq!(
        service.asked(),
        [],
        "nothing is asked until the password is given"
    );
    let sheet = harness.text_of(PASSWORD).unwrap_or_default();
    assert!(sheet.contains("\u{201c}Neighbour\u{201d}"), "{sheet}");
    let join = format!("{PASSWORD} [aria-label=\"Join\"]");
    assert_eq!(
        harness.attr(&join, "aria-disabled").as_deref(),
        Some("true")
    );

    type_into(&mut harness, &format!("{PASSWORD} .ds-input"), "short");
    assert_eq!(
        harness.attr(&join, "aria-disabled").as_deref(),
        Some("true"),
        "a WPA password has eight characters at least"
    );
    // More keys into the field that has focus: a second click would move the caret.
    keys(&mut harness, "er than that");
    assert_ne!(
        harness.attr(&join, "aria-disabled").as_deref(),
        Some("true")
    );
    click(&mut harness, &join);
    assert_eq!(
        service.asked(),
        [Asked::Join(
            ssid("Neighbour"),
            Some(Passphrase::new("shorter than that"))
        )]
    );
    settle(&mut harness, "the sheet closed", |h| h.count(PASSWORD) == 0);
    settle(&mut harness, "Neighbour joined", |h| {
        h.text_of(JOINED).unwrap_or_default().contains("Neighbour")
    });
}
