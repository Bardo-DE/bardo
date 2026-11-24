//! companiond's planner reaches inferd over inference-client's D-Bus transport on the daemon's own
//! connection. The planner is filled and tested over a scripted model (`tests/it/planner.rs`); this one
//! holds only that the constructor is built on the bus and calls nothing until the first session.

use base_testkit::PrivateBus;
use companion_daemon::PlannerModel;

#[tokio::test]
async fn the_planner_is_built_on_the_bus() {
    let scratch = tempfile::tempdir().expect("scratch");
    let bus = PrivateBus::start_in(scratch.path());
    let connection = bus.connect().await;
    let planner = PlannerModel::new(action_dbus::inferd_transport(
        &connection,
        action_dbus::InferWatch::off(),
    ));
    assert!(format!("{planner:?}").contains("Dbus"), "{planner:?}");
}
