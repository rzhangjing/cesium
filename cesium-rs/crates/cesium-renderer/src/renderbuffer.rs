//! Ported from `packages/engine/Source/Renderer/Renderbuffer.js`.
//!
//! A renderbuffer for off-screen rendering attachments.

use crate::context_limits::ContextLimits;
use crate::renderbuffer_format::RenderbufferFormat;

/// Options for creating a [`Renderbuffer`].
pub struct RenderbufferOptions {
    /// The renderbuffer format. Defaults to `RGBA4`.
    pub format: RenderbufferFormat,
    /// Width in pixels. Defaults to the context's drawing buffer width.
    pub width: Option<u32>,
    /// Height in pixels. Defaults to the context's drawing buffer height.
    pub height: Option<u32>,
    /// Number of samples for multisampling. Defaults to 1.
    pub num_samples: Option<u32>,
}

/// A renderbuffer that can be used as a framebuffer attachment.
///
/// Mirrors the JS `Renderbuffer` constructor which creates a GL renderbuffer
/// via `gl.createRenderbuffer()` + `gl.renderbufferStorage()`.
pub struct Renderbuffer {
    wgpu_texture: wgpu::Texture,
    format: RenderbufferFormat,
    width: u32,
    height: u32,
    is_destroyed: bool,
}

impl Renderbuffer {
    /// Creates a new renderbuffer.
    ///
    /// Mirrors the JS private constructor `new Renderbuffer(options)`.
    ///
    /// `context_width` and `context_height` are the drawing buffer dimensions
    /// used as defaults when `options.width`/`options.height` are not provided.
    pub fn new(
        device: &wgpu::Device,
        options: RenderbufferOptions,
        context_width: u32,
        context_height: u32,
    ) -> Self {
        let format = options.format;
        let width = options.width.unwrap_or(context_width);
        let height = options.height.unwrap_or(context_height);
        let _num_samples = options.num_samples.unwrap_or(1);

        debug_assert!(width > 0, "width must be greater than zero");
        debug_assert!(
            width <= ContextLimits::max_renderbuffer_size(),
            "Width must be <= maximum renderbuffer size"
        );
        debug_assert!(height > 0, "height must be greater than zero");
        debug_assert!(
            height <= ContextLimits::max_renderbuffer_size(),
            "Height must be <= maximum renderbuffer size"
        );

        let wgpu_format = renderbuffer_format_to_wgpu(format);

        let wgpu_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: _num_samples,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        Self {
            wgpu_texture,
            format,
            width,
            height,
            is_destroyed: false,
        }
    }

    /// Returns the renderbuffer format.
    pub fn format(&self) -> RenderbufferFormat {
        self.format
    }

    /// Returns the width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns the height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Returns a reference to the underlying wgpu texture.
    pub fn wgpu_texture(&self) -> &wgpu::Texture {
        &self.wgpu_texture
    }

    /// Creates a view of this renderbuffer for use as a framebuffer attachment.
    pub fn create_view(&self) -> wgpu::TextureView {
        self.wgpu_texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    /// Returns whether this renderbuffer has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys the renderbuffer.
    pub fn destroy(&mut self) {
        self.is_destroyed = true;
        // wgpu textures are reference-counted and dropped automatically
    }
}

/// Converts a [`RenderbufferFormat`] to a `wgpu::TextureFormat`.
fn renderbuffer_format_to_wgpu(format: RenderbufferFormat) -> wgpu::TextureFormat {
    match format {
        RenderbufferFormat::Rgba4 => wgpu::TextureFormat::Rgba16Unorm,
        RenderbufferFormat::Rgb5A1 => wgpu::TextureFormat::Rgb10a2Unorm,
        RenderbufferFormat::Rgb565 => wgpu::TextureFormat::Rgba16Unorm, // DEVIATION: no exact BGR565 in wgpu
        RenderbufferFormat::DepthComponent16 => wgpu::TextureFormat::Depth16Unorm,
        RenderbufferFormat::DepthComponent24 => wgpu::TextureFormat::Depth24Plus,
        RenderbufferFormat::DepthComponent32f => wgpu::TextureFormat::Depth32Float,
        RenderbufferFormat::StencilIndex8 => wgpu::TextureFormat::Stencil8,
        RenderbufferFormat::DepthStencil => wgpu::TextureFormat::Depth24PlusStencil8,
        RenderbufferFormat::Rgba8 => wgpu::TextureFormat::Rgba8Unorm,
        RenderbufferFormat::Srgb8Alpha8 => wgpu::TextureFormat::Rgba8UnormSrgb,
        _ => wgpu::TextureFormat::Rgba8Unorm, // fallback
    }
}
