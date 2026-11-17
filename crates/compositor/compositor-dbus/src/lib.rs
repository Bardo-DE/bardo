//! `org.bardo.Compositor1`: the way the shell, the agent tier and the portal get their
//! privileged Wayland connections and input sockets from compositor.
//!
//! The name, the object and the proxy are written here once; the compositor's server (in
//! `compositor-ipc`) implements the same members and decides on the bus thread who may call
//! which. Each caller holds the role its method answers: a call from a program without it gets an
//! access-denied error.

use zbus::zvariant::OwnedFd;

/// The well-known name the compositor owns on the session bus.
pub const BUS_NAME: &str = "org.bardo.Compositor1";

/// The object's path.
pub const OBJECT_PATH: &str = "/org/bardo/Compositor1";

/// The interface's name.
pub const INTERFACE: &str = "org.bardo.Compositor1";

/// The compositor's door, as a caller sees it.
#[zbus::proxy(
    interface = "org.bardo.Compositor1",
    default_service = "org.bardo.Compositor1",
    default_path = "/org/bardo/Compositor1"
)]
pub trait Compositor1 {
    /// A new Wayland connection with the role `role` (`"shell"`, `"agent"` or `"portal"`): the
    /// client end of the socket, when the caller holds the role that stands for it.
    fn connect(&self, role: &str) -> zbus::Result<OwnedFd>;

    /// An EIS socket for the remote-desktop portal session `session`, offering the device kinds
    /// in `devices` (1 keyboard, 2 pointer, 4 touchscreen). Only the desktop portal may ask.
    fn connect_eis(&self, session: u64, devices: u32) -> zbus::Result<OwnedFd>;

    /// Ends every socket handed out for portal session `session`: its input stops and the
    /// compositor's end of each is closed. A session it does not know is not an error.
    fn end_eis(&self, session: u64) -> zbus::Result<()>;

    /// Leases `window` (its toplevel identifier) for the `devices` bit set: the lease number and
    /// the client end of an EIS socket whose input reaches that window and nothing else. Only the
    /// agent tier may ask.
    fn lease_window(&self, window: &str, devices: u32) -> zbus::Result<(u64, OwnedFd)>;

    /// Makes the window `lease` holds the key window (the run is about to send keys).
    fn focus_lease(&self, lease: u64) -> zbus::Result<()>;

    /// Ends a lease and closes its socket; a lease it does not know is not an error.
    fn end_lease(&self, lease: u64) -> zbus::Result<()>;
}
