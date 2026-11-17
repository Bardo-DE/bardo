//! `gpu.*`.

use crate::config::convert;
use compositor_core::settings::CompositorSettings;
use serde::{Deserialize, Serialize};
use settings_types::schema::Page;

/// `gpu.*`: which GPU renders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, settings_types::SettingsSchema)]
#[serde(default)]
#[settings(file = "compositor/settings.toml", domain = "gpu", page = Page::Appearance)]
pub struct GpuSection {
    /// `gpu.render`: `auto`, or the PCI address of the GPU that renders.
    #[settings(
        label = "Rendering GPU",
        help = "auto lets the GPU that drives the displays render; or give a GPU by its PCI \
                address (0000:01:00.0, as under /dev/dri/by-path). Takes effect at the next login.",
        section = "Graphics",
        advanced
    )]
    pub render: String,
}

impl Default for GpuSection {
    fn default() -> Self {
        convert::gpu_doc(&CompositorSettings::defaults().gpu)
    }
}
