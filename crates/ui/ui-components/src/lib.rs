//! quire's renderer-free design system: the host seams, the overlay stack, the root, the generic
//! components, and the assembly of the stylesheet and `Ds`. It sits over three crates (`ui-core`,
//! `ui-style`, `ui-motion`); its tests lint with `ui-lint`. `ui-shell` builds the shell's own parts
//! on top of this crate. Inside the crate a layer names only the layers below it
//! (`scripts/check-boundary.sh`). A consumer writes `use ui_components::prelude::*` for the names it draws
//! with and names everything else by its home path; the root holds only `prelude` and the
//! stylesheet assembly (`kits`, `stylesheet`, `component_sheets`, `KIT`, `selectors`). DESIGN.md
//! maps each module to the design doc section it implements.

pub mod assembly;
pub mod components;
pub mod edit;
pub mod file_drop;
pub mod focus;
pub mod host;
pub mod keys;
pub mod machine;
pub mod root;
pub mod spell;
pub mod stack;
pub mod window;

pub mod prelude;

// The three crates under `ds`, as facades: a consumer names `ui_components::base::..`, `ui_components::style::..` and
// `ui_components::motion::..` and depends on `ds` alone.
pub use ui_core as base;
pub use ui_motion as motion;
pub use ui_style as style;

// The curated roots: the stylesheet assembly a host installs. Every other name is in
// `prelude` or at its home path.
pub use crate::assembly::selectors;
pub use crate::assembly::{
    kit::{KIT, kits},
    stylesheet::{component_sheets, stylesheet},
};
