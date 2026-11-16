//! The Wi-Fi page over a network service that answers from memory: the page without a service, the
//! networks listed, and joining a new secured one. The page's states are tested in `detent-model`.

use crate::support::Sandbox;
use crate::support::drive::{page_of, showing, sidebar_page, title_of};
use crate::support::wifi::Wifi;
use settings_app_model::{
    AccessPoint, NetworkLink, SignalStrength, Ssid, WifiRadio, WifiSecurity, WifiStatus, WifiWire,
};
use settings_app_ui::Opening;
use ui_harness::Harness;

mod joining;

const PAGE: &str = "[data-system=\"wifi\"]";
const JOINED: &str = "#wifi-joined";
const PASSWORD: &str = "[data-system=\"password\"]";
const SECURED: &str = "[data-lock=\"secured\"]";
const OPEN: &str = "[data-lock=\"open\"]";

fn wifi() -> Opening {
    Opening {
        page: Some(settings_app_model::PageSlug("wifi".to_owned())),
        query: String::new(),
    }
}

fn ssid(name: &str) -> Ssid {
    Ssid(name.to_owned())
}

fn point(name: &str, strength: u8, security: WifiSecurity, link: NetworkLink) -> AccessPoint {
    AccessPoint {
        ssid: ssid(name),
        strength: SignalStrength(strength),
        security,
        link,
    }
}

/// On Office, remembering Home, and seeing an open Cafe and a secured Neighbour.
fn status(radio: WifiRadio) -> WifiStatus {
    WifiStatus::of_wire(WifiWire {
        radio,
        access_points: vec![
            point("Office", 90, WifiSecurity::Psk, NetworkLink::Joined),
            point("Home", 70, WifiSecurity::Psk, NetworkLink::Idle),
            point("Cafe", 80, WifiSecurity::Open, NetworkLink::Idle),
            point("Neighbour", 60, WifiSecurity::Psk, NetworkLink::Idle),
        ],
        saved: vec![ssid("Office"), ssid("Home")],
    })
}

fn world(name: &str, service: &Wifi) -> (Sandbox, Harness) {
    let sandbox = Sandbox::new(name).with_wifi(service.clone());
    let harness = sandbox.harness(wifi());
    (sandbox, harness)
}

fn listed(harness: &mut Harness) {
    showing(harness, "the page", PAGE);
    showing(harness, "the joined network", JOINED);
}

fn join_button(name: &str) -> String {
    format!("[aria-label=\"Join {name}\"]")
}

#[test]
fn the_page_is_first_in_the_sidebar_and_says_plainly_when_nothing_serves_wifi() {
    let sandbox = Sandbox::new("wifi-absent");
    let mut harness = sandbox.harness(wifi());
    showing(
        &mut harness,
        "the unavailable row",
        "[data-system=\"unavailable\"]",
    );
    assert_eq!(page_of(&harness).as_deref(), Some("wifi"));
    assert_eq!(title_of(&harness).as_deref(), Some("Wi-Fi"));
    assert_eq!(
        harness.text_of(&sidebar_page("wifi")).as_deref(),
        Some("Wi-Fi")
    );
    assert_eq!(
        harness.text_of(&sidebar_page("bluetooth")).as_deref(),
        Some("Bluetooth")
    );
    assert_eq!(
        harness.text_of(&sidebar_page("displays")).as_deref(),
        Some("Displays")
    );
    let row = harness
        .text_of("[data-system=\"unavailable\"]")
        .unwrap_or_default();
    assert!(
        row.contains("Wi-Fi isn\u{2019}t available right now."),
        "{row}"
    );
}

#[test]
fn the_network_the_computer_is_on_comes_first_with_its_details_and_the_others_follow() {
    let service = Wifi::answering(status(WifiRadio::On));
    let (_sandbox, mut harness) = world("wifi-list", &service);
    listed(&mut harness);
    let joined = harness.text_of(JOINED).unwrap_or_default();
    assert!(
        joined.contains("Office") && joined.contains("Connected"),
        "{joined}"
    );
    assert_eq!(harness.count("[aria-label=\"Details\u{2026}\"]"), 1);
    for name in ["Home", "Cafe", "Neighbour"] {
        assert_eq!(harness.count(&join_button(name)), 1, "{name}");
    }
    assert_eq!(
        harness.count(&join_button("Office")),
        0,
        "the joined one has no Join"
    );
    let titles = harness.html();
    assert!(titles.contains("Known Networks") && titles.contains("Other Networks"));
    assert_eq!(
        harness.count(SECURED),
        3,
        "Office, Home and Neighbour have a lock"
    );
    assert_eq!(harness.count(OPEN), 1, "Cafe has none");
}
