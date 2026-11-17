//! The shell's place on the session bus.
//!
//! The shell serves these interfaces behind caller checks (`shell-services`), so the objects are
//! written by hand there. Its callers hold one proxy of each: the portal asks `Shell1`, Settings
//! reads and sets `Displays1`, and apps read `Outputs1`.

/// The well-known name the shell answers the portal's questions on, and the name whose owner a
/// caller must prove is the shell.
pub const SHELL_BUS: &str = "org.bardo.Shell";

/// Where `org.bardo.Shell1` is exported.
pub const SHELL_PATH: &str = "/org/bardo/Shell1";

/// The well-known name `org.bardo.Displays1` is served under.
pub const DISPLAYS_BUS: &str = "org.bardo.Displays1";

/// Where `org.bardo.Displays1` is exported.
pub const DISPLAYS_PATH: &str = "/org/bardo/Displays1";

/// The well-known name `org.bardo.Outputs1` is served under.
pub const OUTPUTS_BUS: &str = "org.bardo.Outputs1";

/// Where `org.bardo.Outputs1` is exported.
pub const OUTPUTS_PATH: &str = "/org/bardo/Outputs1";

/// One output of `Outputs1.WorkAreas`, `(us(iiuu)(iiuu)u)`: id, name, frame, work area (each x, y,
/// width, height) and the scale in 120ths.
pub type WorkAreaWire = (u32, String, (i32, i32, u32, u32), (i32, i32, u32, u32), u32);

/// The shell's pickers and sheets (`org.bardo.Shell1`), asked by the portal backend alone: the
/// shell answers no other name. The shell's answers are what the person decided.
#[zbus::proxy(
    interface = "org.bardo.Shell1",
    default_service = "org.bardo.Shell",
    default_path = "/org/bardo/Shell1",
    gen_blocking = false
)]
pub trait Shell1 {
    /// Shows the picker and waits for the answer.
    fn pick_capture_source(
        &self,
        app_label: &str,
        kinds: u32,
        multiple: u32,
        cursor_modes: Vec<u32>,
        remembered: u32,
    ) -> zbus::Result<(Vec<(String, String)>, u32)>;

    /// Shows the colour picker and waits for the answer: no tuple is a cancel, one tuple is the
    /// picked colour as red, green and blue, each 0 to 1.
    fn pick_color(&self, app_label: &str) -> zbus::Result<Vec<(f64, f64, f64)>>;

    /// Shows a yes-or-no sheet and waits for the answer: 0 granted, 1 denied, 2 dismissed.
    fn ask_permission(
        &self,
        app_label: &str,
        title: &str,
        subtitle: &str,
        body: &str,
        grant_label: &str,
        deny_label: &str,
    ) -> zbus::Result<u32>;
}

/// The shell's brightness list (`org.bardo.Displays1`). Reading is open; setting is Settings'
/// alone.
#[zbus::proxy(
    interface = "org.bardo.Displays1",
    default_path = "/org/bardo/Displays1"
)]
pub trait Displays1 {
    /// Each display whose brightness can be read: its connector and its percentage.
    fn brightness(&self) -> zbus::Result<Vec<(String, u32)>>;

    /// Sets one display's brightness (0 to 100); only Settings may.
    fn set_brightness(&self, output: &str, percent: u32) -> zbus::Result<()>;
}

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
