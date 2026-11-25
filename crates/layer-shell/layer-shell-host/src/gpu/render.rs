//! Painting a resolved document through the shared renderer: `paint_scene` into the
//! surface's `vello_gpu::Scene` via a `VelloGpuScenePainter` borrowing the device's image
//! cache and texture set, then `Renderer::render` into the swapchain texture. The frame
//! callback is requested before this, and `Queue::present` after.
//!
//! See FINDINGS F75.

use anyrender_vello_gpu::{ImageManager, VelloGpuScenePainter};
use blitz_dom::BaseDocument;
use blitz_paint::paint_scene;
use vello_gpu::{RenderOptions, RenderTarget};

use crate::dom::document::SurfaceDocument;
use crate::error::{HostError, Result};
use crate::gpu::context::GpuContext;
#[cfg(any(feature = "debug-inject", test))]
use crate::gpu::context::SURFACE_FORMAT;
use crate::gpu::presenter::{FrameTarget, Overlay};
#[cfg(any(feature = "debug-inject", test))]
use crate::gpu::shm::swap_red_blue;
use crate::gpu::swapchain::pixel_extent;

/// Paint the already resolved `doc` into `scene`, the host's overlay (the scroll thumbs) over
/// it, then render the scene into `view` with `encoder`. The caller resolved it (`SurfaceDocument::resolve`, which runs the pre-paint hook).
pub(crate) fn render_into(
    gpu: &mut GpuContext,
    doc: &mut SurfaceDocument,
    scene: &mut vello_gpu::Scene,
    target: FrameTarget,
    overlay: &Overlay,
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
) -> Result<()> {
    let mut base = doc.dioxus().inner.borrow_mut();
    paint_document(gpu, &mut base, scene, (target, overlay), encoder, view)
}

/// Paint an already resolved document into `scene` and render it into `view`: [`render_into`]
/// on a bare `BaseDocument`. Widgets in the document see the shared device and register their
/// textures into it, where they stay for later frames. Images are uploaded by the renderer's
/// own cache, which lets go of the ones no frame draws any more.
pub(crate) fn paint_document(
    gpu: &mut GpuContext,
    doc: &mut BaseDocument,
    scene: &mut vello_gpu::Scene,
    (target, overlay): (FrameTarget, &Overlay),
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
) -> Result<()> {
    let (width, height) = pixel_extent(target.buffer);
    let (paint_width, paint_height, x, y) = target.paint_area();
    fit_scene(scene, width, height)?;
    let GpuContext {
        device,
        renderer,
        resources,
        images,
        widgets,
        ..
    } = gpu;
    widgets.forget_dropped(doc);
    {
        let image_manager = ImageManager::new(
            &mut *resources,
            &device.device,
            &device.queue,
            &mut *images,
            &mut widgets.textures,
        );
        let mut painter = VelloGpuScenePainter::new(&mut *scene, image_manager, device);
        paint_scene(
            &mut painter,
            doc,
            target.scale.factor(),
            paint_width,
            paint_height,
            x,
            y,
        );
        crate::scroll::paint::paint(
            &mut painter,
            &overlay.thumbs,
            overlay.scheme,
            target.scale.factor(),
            (f64::from(x), f64::from(y)),
        );
    }
    let rendered = renderer.render(
        scene,
        resources,
        encoder,
        RenderTarget {
            view,
            depth_view: None,
        },
        widgets.textures.bindings(),
        RenderOptions::default(),
    );
    // Evict the images no recent frame drew, whether or not this one rendered.
    ImageManager::new(
        resources,
        &device.device,
        &device.queue,
        images,
        &mut widgets.textures,
    )
    .finish_frame();
    rendered.map_err(|e| HostError::Render(format!("{e:?}")))
}

/// An empty scene of exactly the render size (`vello_gpu` wants the two equal; a scene cannot
/// be resized, so a new size makes a new scene).
fn fit_scene(scene: &mut vello_gpu::Scene, width: u32, height: u32) -> Result<()> {
    // The error holds no more than "does not fit", which `too_large` says with the sizes.
    let (Ok(w), Ok(h)) = (u16::try_from(width), u16::try_from(height)) else {
        return Err(too_large(width, height));
    };
    if scene.width() == w && scene.height() == h {
        scene.reset();
    } else {
        *scene = vello_gpu::Scene::new(w, h);
    }
    Ok(())
}

fn too_large(width: u32, height: u32) -> HostError {
    HostError::Render(format!(
        "{width}x{height} exceeds the renderer's 65535 px limit"
    ))
}

#[cfg(any(feature = "debug-inject", test))]
/// Render an already resolved document offscreen at `target.buffer` through the shared
/// renderer and read it back as RGBA8, premultiplied as rendered. Blocks on the device: for
/// debug captures and tests, never the frame path.
pub(crate) fn capture_document(
    gpu: &mut GpuContext,
    doc: &mut BaseDocument,
    target: FrameTarget,
) -> Result<image::RgbaImage> {
    let (width, height) = pixel_extent(target.buffer);
    let device = gpu.device.device.clone();
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("shell-host capture"),
        size: extent(width, height),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: SURFACE_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let row = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
        * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("shell-host capture readback"),
        size: u64::from(row) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let mut scene = vello_gpu::Scene::new(1, 1);
    paint_document(
        gpu,
        doc,
        &mut scene,
        (target, &Overlay::default()),
        &mut encoder,
        &view,
    )?;
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(height),
            },
        },
        extent(width, height),
    );
    gpu.device.queue.submit([encoder.finish()]);
    read_rows(&device, &readback, row, width, height)
}

#[cfg(any(feature = "debug-inject", test))]
fn extent(width: u32, height: u32) -> wgpu::Extent3d {
    wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    }
}

#[cfg(any(feature = "debug-inject", test))]
/// Map the readback buffer and turn its padded BGRA rows into a tight RGBA image.
fn read_rows(
    device: &wgpu::Device,
    readback: &wgpu::Buffer,
    row: u32,
    width: u32,
    height: u32,
) -> Result<image::RgbaImage> {
    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| HostError::Render(e.to_string()))?;
    let tight = (width * 4) as usize;
    let mut rgba = vec![0u8; tight * height as usize];
    {
        for (src, dst) in data.chunks(row as usize).zip(rgba.chunks_mut(tight)) {
            swap_red_blue(&src[..tight], dst);
        }
    }
    readback.unmap();
    image::RgbaImage::from_raw(width, height, rgba)
        .ok_or_else(|| HostError::Render("capture buffer size mismatch".into()))
}
