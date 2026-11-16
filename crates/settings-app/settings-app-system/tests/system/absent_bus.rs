//! A real link built over no bus answers with why there is nothing, and reaches nothing: the
//! window started without a bus shows each pane as unavailable.

use settings_app_model::{Absence, SpaceKey};
use settings_app_system::bluetooth::{BluetoothLink, OnBluetooth};
use settings_app_system::bus::BusSource;
use settings_app_system::computers::{ComputersLink, OnBus as OnComputers};
use settings_app_system::running::{OnBus as OnRunning, RunningLink};
use settings_app_system::screen_sharing::{OnBus as OnPortal, ScreenSharingLink};
use settings_app_system::spaces::{OnBus as OnSpaces, SpacesLink};
use settings_app_system::updates::{OnBus as OnUpdates, UpdatesLink};
use settings_app_system::wifi::{OnWifi, WifiLink};

#[tokio::test]
async fn the_system_bus_pages_find_no_service_without_a_bus() {
    assert_eq!(
        OnWifi(BusSource::Absent).read().await.err(),
        Some(Absence::NotRunning)
    );
    assert_eq!(
        OnBluetooth::new(BusSource::Absent).read().await.err(),
        Some(Absence::NotRunning)
    );
}

#[tokio::test]
async fn the_daemon_panes_find_no_bus() {
    let bus = || BusSource::Absent;
    assert_eq!(OnUpdates(bus()).read().await.err(), Some(Absence::NoBus));
    assert_eq!(OnComputers(bus()).read().await.err(), Some(Absence::NoBus));
    assert_eq!(OnRunning(bus()).read().await.err(), Some(Absence::NoBus));
    assert_eq!(OnPortal(bus()).list().await.err(), Some(Absence::NoBus));
    assert_eq!(OnSpaces(bus()).list().await.err(), Some(Absence::NoBus));
}

#[tokio::test]
async fn letting_an_app_in_or_taking_it_back_without_a_bus_says_it_was_not_done() {
    let app = || settings_app_model::ProgramSubject("rustdesk".to_owned());
    let portal = OnPortal(BusSource::Absent);
    assert!(portal.allow(app()).await.is_err());
    assert!(portal.revoke(app()).await.is_err());
}

#[tokio::test]
async fn a_removal_without_a_bus_says_it_was_not_made() {
    let removal = memory_core::Removal::KEEP_ALL;
    let ended = OnSpaces(BusSource::Absent)
        .remove(SpaceKey("work".to_owned()), removal)
        .await;
    assert!(ended.is_err());
}
