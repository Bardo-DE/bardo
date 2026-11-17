//! The portal backend's place on the session bus: its well-known name, the object every portal
//! interface is served at, and `org.bardo.Portal1`.
//!
//! The portal (`portal-daemon`) serves its interfaces behind a caller check, so the objects are
//! written by hand there; Settings calls `org.bardo.Portal1` through [`PortalProxy`].

/// The well-known name the portal frontend activates. The name is the portal backend's own; the
/// interface Settings calls on it is `org.bardo.Portal1`.
pub const BUS_NAME: &str = "org.freedesktop.impl.portal.desktop.bardo";

/// The object path every portal backend interface is served at.
pub const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";

/// What the portal remembers (`org.bardo.Portal1`): it lists apps and dates, never a token. A
/// caller that has confirmed the owner of the name addresses it by its unique name.
#[zbus::proxy(
    interface = "org.bardo.Portal1",
    default_path = "/org/freedesktop/portal/desktop"
)]
pub trait Portal {
    /// The apps that may share the screen without asking again, one entry each: app id, the
    /// portal's name for it, the mode (1 while the app runs, 2 until taken back) and when it was
    /// allowed, in seconds since 1970.
    fn list_grants(&self) -> zbus::Result<Vec<(String, String, u32, u64)>>;

    /// Takes back everything remembered for the app: it is asked next time. Answers how many
    /// grants went.
    fn revoke(&self, app_id: &str) -> zbus::Result<u32>;

    /// Lets the app share the screen and take over the keyboard and pointer without asking, until
    /// it is taken back. `screens` is `none`, `every` or `output:<name>`; `input` is the set of
    /// devices (1 keyboard, 2 pointer, 4 touchscreen; 0 none). It replaces what the app had.
    fn allow(&self, app_id: &str, screens: &str, input: u32) -> zbus::Result<()>;
}
