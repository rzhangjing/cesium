//! Render state and GPU resource domain models.
//!
//! Maps to CesiumJS `Renderer/RenderState.js`, `Renderer/ClearCommand.js`,
//! `Renderer/ComputeCommand.js`, `Renderer/PassState.js`,
//! `Renderer/Texture.js`, `Renderer/Framebuffer.js`,
//! `Renderer/TextureAtlas.js`, `Renderer/Buffer.js`.

use glam::DVec4;
use serde::{Deserialize, Serialize};

/// Cull face mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CullFace {
    #[default]
    Back,
    Front,
    FrontAndBack,
}

/// Stencil operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum StencilOp {
    #[default]
    Keep,
    Zero,
    Replace,
    Increment,
    Decrement,
    Invert,
}

/// Stencil test state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StencilState {
    pub enabled: bool,
    pub front_op: StencilOp,
    pub back_op: StencilOp,
    pub ref_value: u32,
    pub mask: u32,
}

impl Default for StencilState {
    fn default() -> Self {
        Self {
            enabled: false,
            front_op: StencilOp::Keep,
            back_op: StencilOp::Keep,
            ref_value: 0,
            mask: 0xFFFFFFFF,
        }
    }
}

/// Polygon offset state.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PolygonOffsetState {
    pub enabled: bool,
    pub factor: f32,
    pub units: f32,
}

impl Default for PolygonOffsetState {
    fn default() -> Self {
        Self { enabled: false, factor: 0.0, units: 0.0 }
    }
}

/// Scissor test state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ScissorState {
    pub enabled: bool,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Complete render state.
///
/// Maps to CesiumJS `Renderer/RenderState.js`
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RenderState {
    pub cull_enabled: bool,
    pub cull_face: CullFace,
    pub depth_test_enabled: bool,
    pub depth_write_enabled: bool,
    pub depth_func: DepthFunc,
    pub blend_enabled: bool,
    pub stencil: StencilState,
    pub polygon_offset: PolygonOffsetState,
    pub scissor: ScissorState,
    pub line_width: f32,
}

/// Depth comparison function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DepthFunc {
    Never,
    Less,
    Equal,
    LessOrEqual,
    #[default]
    Greater,
    NotEqual,
    GreaterOrEqual,
    Always,
}

impl RenderState {
    /// Create a default opaque render state.
    pub fn opaque() -> Self {
        Self {
            cull_enabled: true,
            cull_face: CullFace::Back,
            depth_test_enabled: true,
            depth_write_enabled: true,
            depth_func: DepthFunc::Less,
            blend_enabled: false,
            ..Default::default()
        }
    }

    /// Create a translucent render state with alpha blending.
    pub fn translucent() -> Self {
        Self {
            cull_enabled: true,
            cull_face: CullFace::Back,
            depth_test_enabled: true,
            depth_write_enabled: false,
            depth_func: DepthFunc::Less,
            blend_enabled: true,
            ..Default::default()
        }
    }

    /// Create a render state for 2D (no depth test).
    pub fn state_2d() -> Self {
        Self {
            cull_enabled: false,
            depth_test_enabled: false,
            depth_write_enabled: false,
            blend_enabled: true,
            ..Default::default()
        }
    }
}

/// A clear command.
///
/// Maps to CesiumJS `Renderer/ClearCommand.js`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClearCommand {
    pub color: Option<DVec4>,
    pub depth: Option<f32>,
    pub stencil: Option<u32>,
}

impl Default for ClearCommand {
    fn default() -> Self {
        Self {
            color: Some(DVec4::new(0.0, 0.0, 0.0, 1.0)),
            depth: Some(1.0),
            stencil: Some(0),
        }
    }
}

impl ClearCommand {
    /// Clear color only.
    pub fn color_only(color: DVec4) -> Self {
        Self { color: Some(color), depth: None, stencil: None }
    }

    /// Clear depth only.
    pub fn depth_only(depth: f32) -> Self {
        Self { color: None, depth: Some(depth), stencil: None }
    }

    /// Clear all buffers.
    pub fn all(color: DVec4, depth: f32, stencil: u32) -> Self {
        Self { color: Some(color), depth: Some(depth), stencil: Some(stencil) }
    }
}

/// A compute command for GPU compute operations.
///
/// Maps to CesiumJS `Renderer/ComputeCommand.js`
#[derive(Debug, Clone, PartialEq)]
pub struct ComputeCommand {
    pub shader_id: u64,
    pub work_groups: [u32; 3],
    pub uniform_map: Vec<(String, ComputeUniformValue)>,
}

/// Compute uniform value types.
#[derive(Debug, Clone, PartialEq)]
pub enum ComputeUniformValue {
    Float(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Int(i32),
    Uint(u32),
    Texture(u64),
}

impl ComputeCommand {
    pub fn new(shader_id: u64, work_groups: [u32; 3]) -> Self {
        Self {
            shader_id,
            work_groups,
            uniform_map: Vec::new(),
        }
    }

    pub fn set_uniform(&mut self, name: &str, value: ComputeUniformValue) {
        self.uniform_map.push((name.to_string(), value));
    }
}

/// Pass state for a render pass.
///
/// Maps to CesiumJS `Renderer/PassState.js`
#[derive(Debug, Clone, PartialEq)]
pub struct PassState {
    pub render_state: RenderState,
    pub framebuffer_id: Option<u64>,
    pub viewport: [i32; 4],
}

impl Default for PassState {
    fn default() -> Self {
        Self {
            render_state: RenderState::default(),
            framebuffer_id: None,
            viewport: [0, 0, 1920, 1080],
        }
    }
}

/// Texture pixel format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PixelFormat {
    #[default]
    Rgba,
    Rgb,
    Rg,
    Red,
    Depth,
    DepthStencil,
}

impl PixelFormat {
    /// Returns the number of components per pixel for this format.
    ///
    /// Maps to CesiumJS `PixelFormat.componentsLength`.
    pub fn components_per_pixel(&self) -> usize {
        match self {
            Self::Rgba => 4,
            Self::Rgb => 3,
            Self::Rg => 2,
            Self::Red => 1,
            Self::Depth => 1,
            Self::DepthStencil => 1,
        }
    }

    /// Flips pixel data vertically (Y-axis).
    ///
    /// Maps to CesiumJS `PixelFormat.flipY`.
    pub fn flip_y(data: &[u8], format: PixelFormat, width: usize, height: usize) -> Vec<u8> {
        if height == 1 {
            return data.to_vec();
        }
        let components = format.components_per_pixel();
        let row_bytes = width * components;
        let mut result = vec![0u8; data.len()];
        for row in 0..height {
            let src_offset = row * row_bytes;
            let dst_offset = (height - 1 - row) * row_bytes;
            result[dst_offset..dst_offset + row_bytes]
                .copy_from_slice(&data[src_offset..src_offset + row_bytes]);
        }
        result
    }
}

/// Texture data type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PixelDatatype {
    #[default]
    UnsignedByte,
    Float,
    HalfFloat,
    UnsignedShort,
    UnsignedInt,
}

/// Texture filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TextureFilter {
    #[default]
    Linear,
    Nearest,
    LinearMipmapLinear,
    LinearMipmapNearest,
    NearestMipmapLinear,
    NearestMipmapNearest,
}

/// Texture wrap mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TextureWrap {
    #[default]
    ClampToEdge,
    Repeat,
    MirroredRepeat,
}

/// A texture resource (domain representation).
///
/// Maps to CesiumJS `Renderer/Texture.js`
#[derive(Debug, Clone, PartialEq)]
pub struct Texture {
    pub id: u64,
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    pub datatype: PixelDatatype,
    pub min_filter: TextureFilter,
    pub mag_filter: TextureFilter,
    pub wrap_s: TextureWrap,
    pub wrap_t: TextureWrap,
    pub generate_mipmaps: bool,
}

impl Texture {
    pub fn new(id: u64, width: u32, height: u32) -> Self {
        Self {
            id,
            width,
            height,
            format: PixelFormat::Rgba,
            datatype: PixelDatatype::UnsignedByte,
            min_filter: TextureFilter::Linear,
            mag_filter: TextureFilter::Linear,
            wrap_s: TextureWrap::ClampToEdge,
            wrap_t: TextureWrap::ClampToEdge,
            generate_mipmaps: false,
        }
    }

    pub fn with_mipmaps(mut self) -> Self {
        self.generate_mipmaps = true;
        self.min_filter = TextureFilter::LinearMipmapLinear;
        self
    }
}

/// A framebuffer resource (domain representation).
///
/// Maps to CesiumJS `Renderer/Framebuffer.js`
#[derive(Debug, Clone, PartialEq)]
pub struct Framebuffer {
    pub id: u64,
    pub color_textures: Vec<u64>,
    pub depth_texture: Option<u64>,
    pub stencil_texture: Option<u64>,
    pub width: u32,
    pub height: u32,
}

impl Framebuffer {
    pub fn new(id: u64, width: u32, height: u32) -> Self {
        Self {
            id,
            color_textures: Vec::new(),
            depth_texture: None,
            stencil_texture: None,
            width,
            height,
        }
    }

    pub fn attach_color(&mut self, texture_id: u64) {
        self.color_textures.push(texture_id);
    }

    pub fn attach_depth(&mut self, texture_id: u64) {
        self.depth_texture = Some(texture_id);
    }
}

/// A texture atlas for batching small textures.
///
/// Maps to CesiumJS `Renderer/TextureAtlas.js`
#[derive(Debug, Clone, PartialEq)]
pub struct TextureAtlas {
    pub id: u64,
    pub texture: Texture,
    pub entries: Vec<TextureAtlasEntry>,
    pub padding: u32,
}

/// An entry in a texture atlas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextureAtlasEntry {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl TextureAtlas {
    pub fn new(id: u64, width: u32, height: u32, padding: u32) -> Self {
        Self {
            id,
            texture: Texture::new(id, width, height),
            entries: Vec::new(),
            padding,
        }
    }

    /// Add an entry to the atlas (simple row-based packing).
    pub fn add_entry(&mut self, width: u32, height: u32) -> Option<TextureAtlasEntry> {
        let mut x = self.padding;
        let mut y = self.padding;
        let mut row_height = 0u32;

        for entry in &self.entries {
            if x + width + self.padding <= self.texture.width {
                // Check if it fits in current row
                if entry.y == y {
                    x = x.max(entry.x + entry.width + self.padding);
                    row_height = row_height.max(entry.height);
                }
            }
        }

        if x + width + self.padding > self.texture.width {
            // Move to next row
            x = self.padding;
            y += row_height + self.padding;
        }

        if y + height + self.padding > self.texture.height {
            return None; // Atlas full
        }

        let entry = TextureAtlasEntry { x, y, width, height };
        self.entries.push(entry);
        Some(entry)
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

/// GPU buffer usage.
///
/// Maps to CesiumJS `Renderer/BufferUsage.js`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BufferUsage {
    #[default]
    StaticDraw,
    DynamicDraw,
    StreamDraw,
}

/// A GPU buffer (domain representation).
///
/// Maps to CesiumJS `Renderer/Buffer.js`
#[derive(Debug, Clone, PartialEq)]
pub struct GpuBuffer {
    pub id: u64,
    pub size_in_bytes: usize,
    pub usage: BufferUsage,
}

impl GpuBuffer {
    pub fn new(id: u64, size_in_bytes: usize, usage: BufferUsage) -> Self {
        Self { id, size_in_bytes, usage }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_state_presets() {
        let opaque = RenderState::opaque();
        assert!(opaque.cull_enabled);
        assert!(opaque.depth_test_enabled);
        assert!(opaque.depth_write_enabled);
        assert!(!opaque.blend_enabled);

        let translucent = RenderState::translucent();
        assert!(translucent.blend_enabled);
        assert!(!translucent.depth_write_enabled);

        let state_2d = RenderState::state_2d();
        assert!(!state_2d.cull_enabled);
        assert!(!state_2d.depth_test_enabled);
    }

    #[test]
    fn test_clear_command() {
        let clear = ClearCommand::default();
        assert!(clear.color.is_some());
        assert!(clear.depth.is_some());
        assert!(clear.stencil.is_some());

        let color_only = ClearCommand::color_only(DVec4::new(1.0, 0.0, 0.0, 1.0));
        assert!(color_only.color.is_some());
        assert!(color_only.depth.is_none());
    }

    #[test]
    fn test_compute_command() {
        let mut cmd = ComputeCommand::new(0, [64, 1, 1]);
        cmd.set_uniform("u_scale", ComputeUniformValue::Float(2.0));
        assert_eq!(cmd.work_groups, [64, 1, 1]);
        assert_eq!(cmd.uniform_map.len(), 1);
    }

    #[test]
    fn test_pass_state() {
        let state = PassState::default();
        assert_eq!(state.viewport, [0, 0, 1920, 1080]);
        assert!(state.framebuffer_id.is_none());
    }

    #[test]
    fn test_texture() {
        let tex = Texture::new(0, 256, 256).with_mipmaps();
        assert_eq!(tex.width, 256);
        assert!(tex.generate_mipmaps);
        assert_eq!(tex.min_filter, TextureFilter::LinearMipmapLinear);
    }

    #[test]
    fn test_framebuffer() {
        let mut fb = Framebuffer::new(0, 1024, 768);
        fb.attach_color(1);
        fb.attach_depth(2);
        assert_eq!(fb.color_textures.len(), 1);
        assert_eq!(fb.depth_texture, Some(2));
    }

    #[test]
    fn test_texture_atlas() {
        let mut atlas = TextureAtlas::new(0, 512, 512, 2);
        let e1 = atlas.add_entry(64, 64);
        assert!(e1.is_some());
        let e2 = atlas.add_entry(64, 64);
        assert!(e2.is_some());
        assert_eq!(atlas.entry_count(), 2);
        // Entries should not overlap
        let e1 = e1.unwrap();
        let e2 = e2.unwrap();
        assert!(e1.x + e1.width + 2 <= e2.x || e2.x + e2.width + 2 <= e1.x || e1.y != e2.y);
    }

    #[test]
    fn test_gpu_buffer() {
        let buf = GpuBuffer::new(0, 1024, BufferUsage::DynamicDraw);
        assert_eq!(buf.size_in_bytes, 1024);
        assert_eq!(buf.usage, BufferUsage::DynamicDraw);
    }
}
