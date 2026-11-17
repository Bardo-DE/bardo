//! The connection `org.bardo.Compositor1.Connect("shell")` hands out, asked of casement on the
//! session bus. The probe's `shell-connect` and `compositor-shell-exec` both take theirs here.

use compositor_dbus::Compositor1Proxy;
use std::os::fd::OwnedFd;
use std::time::Duration;

/// How long the bus call may take.
const CALL_PATIENCE: Duration = Duration::from_secs(10);

/// The client end of a new `shell` connection.
///
/// # Errors
///
/// Returns a message when `DBUS_SESSION_BUS_ADDRESS` is not set, when the bus call fails, or when casement does not answer in time.
pub fn shell() -> Result<OwnedFd, String> {
    let address = std::env::var("DBUS_SESSION_BUS_ADDRESS")
        .map_err(|error| format!("DBUS_SESSION_BUS_ADDRESS is not set: {error}"))?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    let call = async {
        let bus = zbus::connection::Builder::address(address.as_str())?
            .build()
            .await?;
        let proxy = Compositor1Proxy::new(&bus).await?;
        Ok::<OwnedFd, zbus::Error>(proxy.connect("shell").await?.into())
    };
    // The timeout's timer is made inside the runtime: tokio's timers need its context.
    runtime
        .block_on(async { tokio::time::timeout(CALL_PATIENCE, call).await })
        .map_err(|elapsed| format!("Connect did not answer: {elapsed}"))?
        .map_err(|e| e.to_string())
}
