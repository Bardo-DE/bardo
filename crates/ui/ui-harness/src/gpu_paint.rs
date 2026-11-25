//! A hybrid harness's painter: the document painted through `anyrender_vello_gpu`, the
//! renderer shell surfaces use, into an offscreen texture on one wgpu device kept for the
//! harness's life. The renderer, its resources (glyph caches, image atlas) and the uploaded
//! images live across frames, as in a window. Every frame is submitted and waited for, so a
//! frame's time includes the GPU finishing it.

use crate::error::HarnessError;
use crate::gpu_device::open_device;
use crate::gpu_diagnostics::GpuDiagnostics;
use crate::painter::{Canvas, PaintTime, draw};
use anyrender_vello_gpu::{ExternalTextures, ImageCache, ImageManager, VelloGpuScenePainter};
use blitz_dom::BaseDocument;
use blitz_embed::adapter::AdapterPref;
use std::time::{Duration, Instant};
use ui_blitz::Gpu;
use ui_blitz::seam::attached_gpu;
use vello_gpu::{RenderOptions, RenderTarget, RenderTargetConfig, Renderer, Resources, Scene};
use wgpu_context::DeviceHandle;

/// The target format: premultiplied RGBA bytes on readback, the layout `vello_cpu` writes.
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// `vello_gpu` on one offscreen device.
pub(crate) struct GpuPainter {
    handle: DeviceHandle,
    adapter: String,
    /// The device as an app's `use_gpu` sees it: this device, attached before the first frame.
    gpu: Gpu,
    renderer: Renderer,
    resources: Resources,
    scene: Scene,
    /// The images the renderer uploaded, by content, kept across frames as layer-shell-host
    /// keeps them; the cache evicts what no frame has drawn for a while.
    images: ImageCache,
    /// Textures the document registered (a canvas) and the uploaded images', by anyrender
    /// resource.
    textures: ExternalTextures,
    target: Target,
}

impl std::fmt::Debug for GpuPainter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GpuPainter")
            .field("adapter", &self.adapter)
            .finish_non_exhaustive()
    }
}

/// The texture painted into and the buffer it is read back through.
struct Target {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    readback: wgpu::Buffer,
    width: u32,
    height: u32,
    /// A readback row, padded to wgpu's copy alignment.
    row_bytes: u32,
}

impl Target {
    fn new(device: &wgpu::Device, width: u32, height: u32) -> Target {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("harness target"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let row_bytes = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("harness readback"),
            size: u64::from(row_bytes) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        Target {
            texture,
            view,
            readback,
            width,
            height,
            row_bytes,
        }
    }
}

impl GpuPainter {
    /// A device on the preferred adapter and a renderer for `width` x `height` device pixels.
    pub(crate) fn open(
        pref: &AdapterPref,
        diagnostics: GpuDiagnostics,
        width: u32,
        height: u32,
    ) -> Result<Self, HarnessError> {
        let (handle, adapter) = open_device(pref, diagnostics)?;
        let (scene_width, scene_height) = scene_size(width, height)?;
        let (renderer, resources) = Renderer::new(
            &handle.device,
            &handle.queue,
            &RenderTargetConfig {
                format: FORMAT,
                width: scene_width,
                height: scene_height,
            },
        );
        let mut images = ImageCache::default();
        images.set_max_image_size(max_image_size(&handle.device));
        let target = Target::new(&handle.device, width, height);
        let gpu = attached_gpu(
            handle.instance.clone(),
            handle.adapter.clone(),
            handle.device.clone(),
            handle.queue.clone(),
        );
        Ok(GpuPainter {
            gpu,
            renderer,
            resources,
            scene: Scene::new(scene_width, scene_height),
            images,
            textures: ExternalTextures::default(),
            target,
            handle,
            adapter,
        })
    }

    /// The painting device as the app's GPU: textures made on it draw in this painter's frames.
    pub(crate) fn gpu(&self) -> Gpu {
        self.gpu.clone()
    }

    /// The adapter painting: `name (backend)`.
    pub(crate) fn adapter(&self) -> &str {
        &self.adapter
    }

    /// Paint `doc` and wait for the GPU to finish, without reading it back.
    pub(crate) fn time(
        &mut self,
        doc: &mut BaseDocument,
        canvas: Canvas,
    ) -> Result<PaintTime, HarnessError> {
        let started = Instant::now();
        let (encoder, encode) = self.encode(doc, canvas)?;
        self.finish(encoder)?;
        Ok(PaintTime::new(encode, started.elapsed()))
    }

    /// Paint `doc` and read the pixels back: premultiplied RGBA, row by row.
    pub(crate) fn picture(
        &mut self,
        doc: &mut BaseDocument,
        canvas: Canvas,
    ) -> Result<Vec<u8>, HarnessError> {
        let (mut encoder, _) = self.encode(doc, canvas)?;
        let target = &self.target;
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &target.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(target.row_bytes),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: target.width,
                height: target.height,
                depth_or_array_layers: 1,
            },
        );
        let slice = self.target.readback.slice(..);
        self.handle.queue.submit([encoder.finish()]);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.wait()?;
        let row = (self.target.width * 4) as usize;
        let pixels = slice
            .get_mapped_range()
            .map_err(|error| HarnessError::Renderer(format!("wgpu readback: {error}")))?
            .chunks_exact(self.target.row_bytes as usize)
            .flat_map(|padded| padded[..row].iter().copied())
            .collect();
        self.target.readback.unmap();
        Ok(pixels)
    }

    /// Build this frame's scene from `doc` and record its render; the time the scene took.
    fn encode(
        &mut self,
        doc: &mut BaseDocument,
        canvas: Canvas,
    ) -> Result<(wgpu::CommandEncoder, Duration), HarnessError> {
        let started = Instant::now();
        let device = &self.handle.device;
        if (self.target.width, self.target.height) != (canvas.width, canvas.height) {
            let (width, height) = scene_size(canvas.width, canvas.height)?;
            self.target = Target::new(device, canvas.width, canvas.height);
            self.scene = Scene::new(width, height);
        }
        self.scene.reset();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("harness frame"),
        });
        // A node that left the document took its widget's textures with it.
        for id in doc.take_pending_resource_deallocations() {
            self.textures.remove(id);
        }
        {
            let images = ImageManager::new(
                &mut self.resources,
                device,
                &self.handle.queue,
                &mut self.images,
                &mut self.textures,
            );
            let mut painter = VelloGpuScenePainter::new(&mut self.scene, images, &self.handle);
            draw(&mut painter, doc, canvas);
        }
        let encode_time = started.elapsed();
        let rendered = self.renderer.render(
            &self.scene,
            &mut self.resources,
            &mut encoder,
            RenderTarget {
                view: &self.target.view,
                depth_view: None,
            },
            self.textures.bindings(),
            RenderOptions::default(),
        );
        // Evict the images no recent frame drew, whether or not this one rendered.
        ImageManager::new(
            &mut self.resources,
            device,
            &self.handle.queue,
            &mut self.images,
            &mut self.textures,
        )
        .finish_frame();
        rendered.map_err(|error| HarnessError::Renderer(format!("vello_gpu: {error:?}")))?;
        Ok((encoder, encode_time))
    }

    /// Submit `encoder` and wait until the GPU has run it.
    fn finish(&self, encoder: wgpu::CommandEncoder) -> Result<(), HarnessError> {
        self.handle.queue.submit([encoder.finish()]);
        self.wait()
    }

    fn wait(&self) -> Result<(), HarnessError> {
        self.handle
            .device
            .poll(wgpu::PollType::wait_indefinitely())
            .map(|_| ())
            .map_err(|error| HarnessError::Renderer(format!("wgpu poll: {error}")))
    }
}

/// `width` x `height` as the renderer's 16-bit scene size, or an error naming the limit.
fn scene_size(width: u32, height: u32) -> Result<(u16, u16), HarnessError> {
    match (u16::try_from(width), u16::try_from(height)) {
        (Ok(w), Ok(h)) => Ok((w, h)),
        _ => Err(HarnessError::Renderer(format!(
            "{width}x{height} exceeds the renderer's 65535 px limit"
        ))),
    }
}

/// The largest image the renderer uploads as it is: the device's texture limit, within what
/// `vello_gpu` sizes can hold. A larger image is scaled down to it on upload.
fn max_image_size(device: &wgpu::Device) -> (u16, u16) {
    let side = u16::try_from(device.limits().max_texture_dimension_2d).unwrap_or(u16::MAX);
    (side, side)
}
