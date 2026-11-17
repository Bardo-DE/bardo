//! The computer's system bus. A page looks at it and never starts anything: opening Settings must
//! not wake a service the person's session did not start. The services behind these pages are
//! system services whose names the bus's own policy reserves for the system, so the caller table
//! that vouches for the desktop's own daemons has nothing to add.

use super::io::because;
use base_time::deadline::within;
use dbus_kit::connect::BusTarget;
use settings_app_link::bus::BusSource;
use settings_app_link::{Deadline, peer};
use settings_app_model::Absence;
use zbus::Connection;
use zbus::fdo::DBusProxy;
use zbus::names::BusName;

/// A connection to the system bus. The session source means the real system bus; a test's private
/// bus is used as it is; no bus is no service.
///
/// # Errors
///
/// `Absence::NotRunning` when there is no bus, `Absence::Failed` when it does not answer in time.
pub async fn system_connection(bus: &BusSource) -> Result<Connection, Absence> {
    match within(Deadline::READ, peer::DESKTOP, dial(bus)).await {
        Ok(dialled) => dialled,
        Err(_) => Err(Absence::Failed),
    }
}

async fn dial(bus: &BusSource) -> Result<Connection, Absence> {
    match bus {
        BusSource::Absent => Err(Absence::NotRunning),
        BusSource::Session => BusTarget::System
            .connect()
            .await
            .map_err(because(Absence::NotRunning)),
        BusSource::Address(address, _) => BusTarget::Address(address.clone())
            .connect()
            .await
            .map_err(because(Absence::NotRunning)),
    }
}

/// Whether `name` has an owner on the bus now: a look, never a start.
pub async fn is_running(connection: &Connection, name: &'static str) -> bool {
    let Ok(name) = BusName::try_from(name) else {
        return false;
    };
    let Ok(bus) = DBusProxy::new(connection).await else {
        return false;
    };
    bus.name_has_owner(name).await.unwrap_or(false)
}
