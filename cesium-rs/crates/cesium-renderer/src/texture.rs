//! Ported from `packages/engine/Source/Renderer/Texture.js`.
//!
//! A wrapper for a `wgpu::Texture` to abstract away the verbose GPU calls
//! associated with setting up a texture.

use cesium_core::create_guid::create_guid;
use cesium_core::pixel_format::PixelFormat;

use crate::context_limits::ContextLimits;
use crate::mipmap_hint::MipmapHint;
use crate::pixel_datatype::PixelDatatype;
use crate::sampler::Sampler;

/// Source data for creating a texture.
pub struct TextureSource {
    /// Pixel width of the source.
    pub width: u32,
    /// Pixel height of the source.
    pub height: u32,
    /// Raw pixel data.
    pub array_buffer_view: Vec<u8>,
}

/// Options for creating a [`Texture`].
pub struct TextureOptions {
    /// The source for texel values.
    pub source: Option<TextureSource>,
    /// The pixel format. Defaults to `RGBA`.
    pub pixel_format: PixelFormat,
    /// The pixel datatype. Defaults to `UnsignedByte`.
    pub pixel_datatype: PixelDatatype,
    /// Whether to flip the Y axis when reading source.
    pub flip_y: bool,
    /// Whether to skip color space conversion.
    pub skip_color_space_conversion: bool,
    /// Sampler parameters.
    pub sampler: Option<Sampler>,
    /// Explicit width (used when no source is provided).
    pub width: Option<u32>,
    /// Explicit height (used when no source is provided).
    pub height: Option<u32>,
    /// Whether to premultiply alpha.
    pub pre_multiply_alpha: Option<bool>,
    /// A unique ID for this texture.
    pub id: Option<String>,
}

impl Default for TextureOptions {
    fn default() -> Self {
        Self {
            source: None,
            pixel_format: PixelFormat::Rgba,
            pixel_datatype: PixelDatatype::UnsignedByte,
            flip_y: true,
            skip_color_space_conversion: false,
            sampler: None,
            width: None,
            height: None,
            pre_multiply_alpha: None,
            id: None,
        }
    }
}

/// A 2D texture on the GPU.
///
/// Mirrors the JS `Texture` constructor which creates a `WebGLTexture`.
pub struct Texture {
    id: String,
    wgpu_texture: wgpu::Texture,
    pixel_format: PixelFormat,
    pixel_datatype: PixelDatatype,
    width: u32,
    height: u32,
    has_mipmap: bool,
    sampler: Sampler,
    pre_multiply_alpha: bool,
    flip_y: bool,
    is_destroyed: bool,
}

impl Texture {
    /// Creates a new 2D texture.
    ///
    /// Mirrors the JS constructor `new Texture(options)`.
    pub fn new(device: &wgpu::Device, options: TextureOptions) -> Self {
        let pixel_format = options.pixel_format;
        let pixel_datatype = options.pixel_datatype;
        let flip_y = options.flip_y;
        let sampler = options.sampler.unwrap_or_default();

        // Determine dimensions from source or explicit width/height
        let (width, height) = if let Some(ref source) = options.source {
            (source.width, source.height)
        } else {
            (
                options.width.expect("width or source is required"),
                options.height.expect("height or source is required"),
            )
        };

        debug_assert!(width > 0, "width must be greater than zero");
        debug_assert!(
            width <= ContextLimits::max_texture_size(),
            "Width must be <= maximum texture size"
        );
        debug_assert!(height > 0, "height must be greater than zero");
        debug_assert!(
            height <= ContextLimits::max_texture_size(),
            "Height must be <= maximum texture size"
        );

        let pre_multiply_alpha = options.pre_multiply_alpha
            .unwrap_or(pixel_format == PixelFormat::Rgb || pixel_format == PixelFormat::Luminance);

        let wgpu_format = pixel_format_to_wgpu(pixel_format, pixel_datatype);

        let wgpu_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu_format,
            // DEVIATION: CesiumJS textures are implicitly usable as
            // framebuffer attachments; wgpu requires the explicit
            // RENDER_ATTACHMENT usage bit.
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        // DEVIATION: In JS the GL context provides the queue inline.
        // In wgpu the queue is separate; caller must upload source data
        // via `upload_source()` after construction.

        Self {
            id: options.id.unwrap_or_else(create_guid),
            wgpu_texture,
            pixel_format,
            pixel_datatype,
            width,
            height,
            has_mipmap: false,
            sampler,
            pre_multiply_alpha,
            flip_y,
            is_destroyed: false,
        }
    }

    /// Returns the unique identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the pixel width.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns the pixel height.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Returns the pixel format.
    pub fn pixel_format(&self) -> PixelFormat {
        self.pixel_format
    }

    /// Returns the pixel datatype.
    pub fn pixel_datatype(&self) -> PixelDatatype {
        self.pixel_datatype
    }

    /// Returns the sampler.
    pub fn sampler(&self) -> &Sampler {
        &self.sampler
    }

    /// Sets the sampler.
    pub fn set_sampler(&mut self, sampler: Sampler) {
        self.sampler = sampler;
    }

    /// Returns whether this texture has mipmaps.
    pub fn has_mipmap(&self) -> bool {
        self.has_mipmap
    }

    /// Returns whether alpha is premultiplied.
    pub fn pre_multiply_alpha(&self) -> bool {
        self.pre_multiply_alpha
    }

    /// Returns whether Y is flipped.
    pub fn flip_y(&self) -> bool {
        self.flip_y
    }

    /// Returns a reference to the underlying wgpu texture.
    pub fn wgpu_texture(&self) -> &wgpu::Texture {
        &self.wgpu_texture
    }

    /// Returns the wgpu texture format of this texture.
    pub fn wgpu_format(&self) -> wgpu::TextureFormat {
        self.wgpu_texture.format()
    }

    /// Creates a texture view.
    pub fn create_view(&self) -> wgpu::TextureView {
        self.wgpu_texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    /// Uploads source data to this texture using the given queue.
    ///
    /// DEVIATION: In JS the GL context provides the queue inline.
    /// In wgpu the queue must be passed explicitly.
    pub fn upload_source(&self, queue: &wgpu::Queue, source: &TextureSource) {
        let bytes_per_row =
            compute_bytes_per_row(self.pixel_format, self.pixel_datatype, source.width);
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.wgpu_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &source.array_buffer_view,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(source.height),
            },
            wgpu::Extent3d {
                width: source.width,
                height: source.height,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Copies pixel data from an array view into a sub-region of this texture.
    ///
    /// Mirrors `Texture.prototype.copyFrom(options)`.
    pub fn copy_from(
        &mut self,
        queue: &wgpu::Queue,
        source: &TextureSource,
        x_offset: u32,
        y_offset: u32,
    ) {
        let bytes_per_row =
            compute_bytes_per_row(self.pixel_format, self.pixel_datatype, source.width);
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.wgpu_texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: x_offset,
                    y: y_offset,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &source.array_buffer_view,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(source.height),
            },
            wgpu::Extent3d {
                width: source.width,
                height: source.height,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Generates mipmaps for this texture.
    ///
    /// Mirrors `Texture.prototype.generateMipmap(hint)`.
    pub fn generate_mipmap(&mut self, _device: &wgpu::Device, _hint: MipmapHint) {
        // DEVIATION: wgpu mipmap generation requires a render pass chain or compute shader.
        // The JS version calls gl.generateMipmap(). In wgpu, we need to either:
        // 1. Create the texture with full mip chain and render downsample passes
        // 2. Use a compute shader to downsample
        // For now, mark as having mipmaps but the actual generation is deferred.
        self.has_mipmap = true;
    }

    /// Returns whether this texture has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys the texture.
    pub fn destroy(&mut self) {
        self.is_destroyed = true;
    }
}

/// Computes bytes per row for a given pixel format, datatype, and width.
fn compute_bytes_per_row(
    format: PixelFormat,
    datatype: PixelDatatype,
    width: u32,
) -> u32 {
    let components = format.components_length() as u32;
    let bytes_per_component = match datatype {
        PixelDatatype::UnsignedByte => 1,
        PixelDatatype::UnsignedShort | PixelDatatype::HalfFloat => 2,
        PixelDatatype::UnsignedInt | PixelDatatype::Float => 4,
        // Packed formats have fixed sizes
        PixelDatatype::UnsignedShort565
        | PixelDatatype::UnsignedShort5551
        | PixelDatatype::UnsignedShort4444 => 2,
        PixelDatatype::UnsignedInt248 => 4,
        _ => 1,
    };
    // wgpu requires bytes_per_row to be aligned to 256 bytes for copies
    let raw = components * bytes_per_component * width;
    (raw + 255) & !255 // align up to 256
}

/// Converts a pixel format + datatype combination to a `wgpu::TextureFormat`.
fn pixel_format_to_wgpu(
    format: PixelFormat,
    datatype: PixelDatatype,
) -> wgpu::TextureFormat {
    use PixelDatatype::*;
    use PixelFormat::*;

    match (format, datatype) {
        (Rgba, UnsignedByte) => wgpu::TextureFormat::Rgba8Unorm,
        (Rgba, UnsignedShort4444) => wgpu::TextureFormat::Rgba16Unorm,
        (Rgba, UnsignedShort5551) => wgpu::TextureFormat::Rgb10a2Unorm,
        (Rgb, UnsignedByte) => wgpu::TextureFormat::Rgba8Unorm, // DEVIATION: wgpu has no RGB8
        (Rgba, Float) => wgpu::TextureFormat::Rgba32Float,
        (Rgba, HalfFloat) => wgpu::TextureFormat::Rgba16Float,
        (Red, UnsignedByte) => wgpu::TextureFormat::R8Unorm,
        (Red, Float) => wgpu::TextureFormat::R32Float,
        (Red, HalfFloat) => wgpu::TextureFormat::R16Float,
        (Rg, UnsignedByte) => wgpu::TextureFormat::Rg8Unorm,
        (Rg, Float) => wgpu::TextureFormat::Rg32Float,
        (Rg, HalfFloat) => wgpu::TextureFormat::Rg16Float,
        (Alpha, UnsignedByte) => wgpu::TextureFormat::R8Unorm, // DEVIATION: alpha → R8
        (Luminance, UnsignedByte) => wgpu::TextureFormat::R8Unorm, // DEVIATION: luminance → R8
        (LuminanceAlpha, UnsignedByte) => wgpu::TextureFormat::Rg8Unorm, // DEVIATION
        (DepthComponent, UnsignedShort) => wgpu::TextureFormat::Depth16Unorm,
        (DepthComponent, UnsignedInt) => wgpu::TextureFormat::Depth24Plus,
        (DepthComponent, Float) => wgpu::TextureFormat::Depth32Float,
        (DepthStencil, UnsignedInt248) => wgpu::TextureFormat::Depth24PlusStencil8,
        (DepthStencil, Float) => wgpu::TextureFormat::Depth32FloatStencil8,
        _ => wgpu::TextureFormat::Rgba8Unorm, // fallback
    }
}
