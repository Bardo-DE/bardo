//! Custom-widget textures that outlive a frame. A `blitz_dom::Widget` registers its wgpu
//! texture through the painter's `RenderContext` (`try_register_custom_resource`), and gets the
//! device from `renderer_specific_context()` (a `wgpu_context::DeviceHandle`).
//! `VelloGpuScenePainter` borrows the texture set for one paint; this holds it per device
//! between paints, next to the images the renderer uploaded into the same set.
//!
//! See FINDINGS F77.

use anyrender_vello_gpu::ExternalTextures;
use blitz_dom::BaseDocument;

/// The registered textures of one device: the widgets' and the uploaded images'.
#[derive(Debug, Default)]
pub(crate) struct ContextProxy {
    pub(crate) textures: ExternalTextures,
}

impl ContextProxy {
    /// Forget the textures of the widgets `doc` has dropped since the last call (a node that left
    /// the page takes its widget with it). Called before a paint, so the set holds what the pages
    /// still show and not what they once showed.
    pub(crate) fn forget_dropped(&mut self, doc: &mut BaseDocument) {
        for id in doc.take_pending_resource_deallocations() {
            self.textures.remove(id);
        }
    }

    /// Forget every texture `doc`'s widgets hold, for a document that is going away for good.
    pub(crate) fn forget_document(&mut self, doc: &mut BaseDocument) {
        for node in doc.custom_widget_node_ids() {
            doc.remove_custom_widget(node);
        }
        self.forget_dropped(doc);
    }
}
