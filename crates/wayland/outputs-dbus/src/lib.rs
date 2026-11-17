//! The shell's work areas on the session bus (`org.bardo.Outputs1`).
//!
//! The shell serves them (`shell-services`); an app that sizes and places a window reads them
//! (`ui-desktop`). The names, the wire row and the proxy live here, below both, so a UI crate
//! never names a shell crate.

/// The well-known name `org.bardo.Outputs1` is served under.
pub const OUTPUTS_BUS: &str = "org.bardo.Outputs1";

/// Where `org.bardo.Outputs1` is exported.
pub const OUTPUTS_PATH: &str = "/org/bardo/Outputs1";

/// One output of `Outputs1.WorkAreas`, `(us(iiuu)(iiuu)u)`: id, name, frame, work area (each x, y,
/// width, height) and the scale in 120ths.
pub type WorkAreaWire = (u32, String, (i32, i32, u32, u32), (i32, i32, u32, u32), u32);

/// The shell's work areas (`org.bardo.Outputs1`), for an app that sizes and places a window.
#[zbus::proxy(
    interface = "org.bardo.Outputs1",
    default_service = "org.bardo.Outputs1",
    default_path = "/org/bardo/Outputs1",
    gen_blocking = false
)]
pub trait Outputs1 {
    /// Each output's frame, work area and scale.
    #[zbus(property)]
    fn work_areas(&self) -> zbus::Result<Vec<WorkAreaWire>>;
}
