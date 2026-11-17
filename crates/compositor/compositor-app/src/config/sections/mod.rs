//! The document's sections: one file per `CompositorSettings` part. Each key has a
//! `#[settings(...)]` row (ui-settings' `SettingsSchema` derive), which is the schema the Settings app and
//! The Settings app read; each section's `Default` is the model's own default through
//! `convert`, so no default is written twice.

mod bindings;
mod gestures;
mod gpu;
mod keyboard;
mod night_light;
mod outputs;
mod pointer;
mod spaces;
mod touchpad;
mod windows;
mod xwayland;

pub use bindings::BindingsSection;
pub use gestures::GesturesSection;
pub use gpu::GpuSection;
pub use keyboard::KeyboardSection;
pub use night_light::{NightLightMode, NightLightSection};
pub use outputs::{FirstSight, OutputsSection};
pub use pointer::PointerSection;
pub use spaces::SpacesSection;
pub use touchpad::TouchpadSection;
pub use windows::WindowsSection;
pub use xwayland::{XwaylandSection, XwaylandUse};
