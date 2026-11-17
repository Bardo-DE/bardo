//! GPU settings (`gpu.*`): which GPU renders (P1-T). The stored key and its conversion are in
//! `compositor/src/config/`.

use crate::gpu::GpuChoice;

/// `gpu.*`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuSettings {
    /// `gpu.render`: the GPU that renders every frame.
    pub render: GpuChoice,
}

impl GpuSettings {
    /// Settings that render on `render`.
    #[must_use]
    pub fn new(render: GpuChoice) -> GpuSettings {
        GpuSettings { render }
    }

    /// The default: the GPU that drives the outputs renders.
    ///
    /// ```
    /// use compositor_core::gpu::GpuChoice;
    /// use compositor_core::settings::GpuSettings;
    ///
    /// assert_eq!(GpuSettings::defaults().render, GpuChoice::Auto);
    /// ```
    #[must_use]
    pub fn defaults() -> GpuSettings {
        GpuSettings {
            render: GpuChoice::Auto,
        }
    }
}
