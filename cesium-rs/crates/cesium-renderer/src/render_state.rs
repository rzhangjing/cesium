//! Ported from `packages/engine/Source/Renderer/RenderState.js`.
//!
//! Describes the complete rendering state for a draw command.
//! In CesiumJS, this is a nested object with sub-objects for cull, polygonOffset,
//! scissorTest, depthRange, depthTest, colorMask, blending, stencilTest, etc.
//! In the Rust port, this is flattened into a single struct for better performance
//! and easier hashing.

use std::hash::{Hash, Hasher};
use cesium_core::bounding_rectangle::BoundingRectangle;

/// Blending function factors (mirrors WebGL constants).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlendingFactor {
    Zero,
    One,
    SrcColor,
    OneMinusSrcColor,
    DstColor,
    OneMinusDstColor,
    SrcAlpha,
    OneMinusSrcAlpha,
    DstAlpha,
    OneMinusDstAlpha,
    ConstantColor,
    OneMinusConstantColor,
    ConstantAlpha,
    OneMinusConstantAlpha,
    SrcAlphaSaturate,
}

/// Blending equation (mirrors WebGL constants).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlendEquation {
    FuncAdd,
    FuncSubtract,
    FuncReverseSubtract,
    Min,
    Max,
}

/// Depth comparison function (mirrors WebGL constants).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DepthFunction {
    Never,
    Less,
    Equal,
    Lequal,
    Greater,
    Notequal,
    Gequal,
    Always,
}

/// Stencil operation (mirrors WebGL constants).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StencilOperation {
    Zero,
    Keep,
    Replace,
    Incr,
    Decr,
    Invert,
    IncrWrap,
    DecrWrap,
}

/// Face culling mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CullFace {
    Front,
    Back,
    FrontAndBack,
}

/// Winding order for front-facing polygons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrontFace {
    Clockwise,
    CounterClockwise,
}

/// Stencil operation configuration for front or back faces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StencilOperationConfig {
    pub fail: StencilOperation,
    pub z_fail: StencilOperation,
    pub z_pass: StencilOperation,
}

impl Default for StencilOperationConfig {
    fn default() -> Self {
        Self {
            fail: StencilOperation::Keep,
            z_fail: StencilOperation::Keep,
            z_pass: StencilOperation::Keep,
        }
    }
}

/// Polygon offset configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PolygonOffset {
    pub enabled: bool,
    pub factor: f32,
    pub units: f32,
}

impl Default for PolygonOffset {
    fn default() -> Self {
        Self {
            enabled: false,
            factor: 0.0,
            units: 0.0,
        }
    }
}

/// Depth range configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DepthRange {
    pub near: f64,
    pub far: f64,
}

impl Default for DepthRange {
    fn default() -> Self {
        Self {
            near: 0.0,
            far: 1.0,
        }
    }
}

/// Color mask for RGBA channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColorMask {
    pub red: bool,
    pub green: bool,
    pub blue: bool,
    pub alpha: bool,
}

impl Default for ColorMask {
    fn default() -> Self {
        Self {
            red: true,
            green: true,
            blue: true,
            alpha: true,
        }
    }
}

/// Blending configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Blending {
    pub enabled: bool,
    pub color: [f32; 4],
    pub equation_rgb: BlendEquation,
    pub equation_alpha: BlendEquation,
    pub function_source_rgb: BlendingFactor,
    pub function_source_alpha: BlendingFactor,
    pub function_destination_rgb: BlendingFactor,
    pub function_destination_alpha: BlendingFactor,
}

impl Default for Blending {
    fn default() -> Self {
        Self {
            enabled: false,
            color: [0.0, 0.0, 0.0, 0.0],
            equation_rgb: BlendEquation::FuncAdd,
            equation_alpha: BlendEquation::FuncAdd,
            function_source_rgb: BlendingFactor::One,
            function_source_alpha: BlendingFactor::One,
            function_destination_rgb: BlendingFactor::Zero,
            function_destination_alpha: BlendingFactor::Zero,
        }
    }
}

/// Stencil test configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StencilTest {
    pub enabled: bool,
    pub front_function: DepthFunction,
    pub back_function: DepthFunction,
    pub reference: u32,
    pub mask: u32,
    pub front_operation: StencilOperationConfig,
    pub back_operation: StencilOperationConfig,
}

impl Default for StencilTest {
    fn default() -> Self {
        Self {
            enabled: false,
            front_function: DepthFunction::Always,
            back_function: DepthFunction::Always,
            reference: 0,
            mask: 0xFFFFFFFF,
            front_operation: StencilOperationConfig::default(),
            back_operation: StencilOperationConfig::default(),
        }
    }
}

/// Sample coverage configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SampleCoverage {
    pub enabled: bool,
    pub value: f32,
    pub invert: bool,
}

impl Default for SampleCoverage {
    fn default() -> Self {
        Self {
            enabled: false,
            value: 1.0,
            invert: false,
        }
    }
}

/// The complete render state for a draw command.
///
/// This is a hashable value type used as a pipeline cache key.
/// Mirrors the nested structure of CesiumJS's RenderState but flattened
/// for better performance in Rust.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderState {
    /// Front face winding order.
    pub front_face: FrontFace,
    /// Face culling configuration.
    pub cull: CullConfig,
    /// Line width for line primitives.
    pub line_width: f32,
    /// Polygon offset configuration.
    pub polygon_offset: PolygonOffset,
    /// Scissor test configuration.
    pub scissor_test: ScissorTestConfig,
    /// Depth range configuration.
    pub depth_range: DepthRange,
    /// Depth test configuration.
    pub depth_test: DepthTestConfig,
    /// Color mask configuration.
    pub color_mask: ColorMask,
    /// Whether depth writing is enabled.
    pub depth_mask: bool,
    /// Stencil mask.
    pub stencil_mask: u32,
    /// Blending configuration.
    pub blending: Blending,
    /// Stencil test configuration.
    pub stencil_test: StencilTest,
    /// Sample coverage configuration.
    pub sample_coverage: SampleCoverage,
    /// The viewport rectangle.
    pub viewport: Option<BoundingRectangle>,
}

/// Face culling configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CullConfig {
    pub enabled: bool,
    pub face: CullFace,
}

impl Default for CullConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            face: CullFace::Back,
        }
    }
}

/// Scissor test configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScissorTestConfig {
    pub enabled: bool,
    pub rectangle: BoundingRectangle,
}

impl Default for ScissorTestConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            rectangle: BoundingRectangle::default(),
        }
    }
}

/// Depth test configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DepthTestConfig {
    pub enabled: bool,
    pub func: DepthFunction,
}

impl Default for DepthTestConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            func: DepthFunction::Less,
        }
    }
}

impl RenderState {
    /// Creates a new default render state.
    pub fn new() -> Self {
        Self {
            front_face: FrontFace::CounterClockwise,
            cull: CullConfig::default(),
            line_width: 1.0,
            polygon_offset: PolygonOffset::default(),
            scissor_test: ScissorTestConfig::default(),
            depth_range: DepthRange::default(),
            depth_test: DepthTestConfig::default(),
            color_mask: ColorMask::default(),
            depth_mask: true,
            stencil_mask: 0xFFFFFFFF,
            blending: Blending::default(),
            stencil_test: StencilTest::default(),
            sample_coverage: SampleCoverage::default(),
            viewport: None,
        }
    }

    /// Computes a hash of this render state for use as a pipeline cache key.
    ///
    /// This is a simplified hash that captures the most important state changes.
    /// For a full hash, all fields would need to be included.
    pub fn compute_hash(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

impl Default for RenderState {
    fn default() -> Self { Self::new() }
}

// ── wgpu state mapping ──────────────────────────────────────────────────────
//
// DEVIATION: CesiumJS applies RenderState imperatively per draw via GL calls
// (`gl.depthFunc`, `gl.blendFunc`, ...). In wgpu all of this state is baked
// into immutable `RenderPipeline` objects; the functions below translate the
// CesiumJS state model into the corresponding wgpu descriptor fragments.

/// Maps a [`BlendingFactor`] to `wgpu::BlendFactor`.
pub fn blending_factor_to_wgpu(factor: BlendingFactor) -> wgpu::BlendFactor {
    match factor {
        BlendingFactor::Zero => wgpu::BlendFactor::Zero,
        BlendingFactor::One => wgpu::BlendFactor::One,
        BlendingFactor::SrcColor => wgpu::BlendFactor::Src,
        BlendingFactor::OneMinusSrcColor => wgpu::BlendFactor::OneMinusSrc,
        BlendingFactor::DstColor => wgpu::BlendFactor::Dst,
        BlendingFactor::OneMinusDstColor => wgpu::BlendFactor::OneMinusDst,
        BlendingFactor::SrcAlpha => wgpu::BlendFactor::SrcAlpha,
        BlendingFactor::OneMinusSrcAlpha => wgpu::BlendFactor::OneMinusSrcAlpha,
        BlendingFactor::DstAlpha => wgpu::BlendFactor::DstAlpha,
        BlendingFactor::OneMinusDstAlpha => wgpu::BlendFactor::OneMinusDstAlpha,
        BlendingFactor::ConstantColor | BlendingFactor::ConstantAlpha => {
            wgpu::BlendFactor::Constant
        }
        BlendingFactor::OneMinusConstantColor | BlendingFactor::OneMinusConstantAlpha => {
            wgpu::BlendFactor::OneMinusConstant
        }
        BlendingFactor::SrcAlphaSaturate => wgpu::BlendFactor::SrcAlphaSaturated,
    }
}

/// Maps a [`BlendEquation`] to `wgpu::BlendOperation`.
pub fn blend_equation_to_wgpu(equation: BlendEquation) -> wgpu::BlendOperation {
    match equation {
        BlendEquation::FuncAdd => wgpu::BlendOperation::Add,
        BlendEquation::FuncSubtract => wgpu::BlendOperation::Subtract,
        BlendEquation::FuncReverseSubtract => wgpu::BlendOperation::ReverseSubtract,
        BlendEquation::Min => wgpu::BlendOperation::Min,
        BlendEquation::Max => wgpu::BlendOperation::Max,
    }
}

/// Maps a [`DepthFunction`] to `wgpu::CompareFunction`.
pub fn depth_function_to_wgpu(func: DepthFunction) -> wgpu::CompareFunction {
    match func {
        DepthFunction::Never => wgpu::CompareFunction::Never,
        DepthFunction::Less => wgpu::CompareFunction::Less,
        DepthFunction::Equal => wgpu::CompareFunction::Equal,
        DepthFunction::Lequal => wgpu::CompareFunction::LessEqual,
        DepthFunction::Greater => wgpu::CompareFunction::Greater,
        DepthFunction::Notequal => wgpu::CompareFunction::NotEqual,
        DepthFunction::Gequal => wgpu::CompareFunction::GreaterEqual,
        DepthFunction::Always => wgpu::CompareFunction::Always,
    }
}

/// Maps a [`StencilOperation`] to `wgpu::StencilOperation`.
pub fn stencil_operation_to_wgpu(op: StencilOperation) -> wgpu::StencilOperation {
    match op {
        StencilOperation::Zero => wgpu::StencilOperation::Zero,
        StencilOperation::Keep => wgpu::StencilOperation::Keep,
        StencilOperation::Replace => wgpu::StencilOperation::Replace,
        StencilOperation::Incr => wgpu::StencilOperation::IncrementClamp,
        StencilOperation::Decr => wgpu::StencilOperation::DecrementClamp,
        StencilOperation::Invert => wgpu::StencilOperation::Invert,
        StencilOperation::IncrWrap => wgpu::StencilOperation::IncrementWrap,
        StencilOperation::DecrWrap => wgpu::StencilOperation::DecrementWrap,
    }
}

impl RenderState {
    /// Maps the front-face winding order to `wgpu::FrontFace`.
    pub fn to_wgpu_front_face(&self) -> wgpu::FrontFace {
        match self.front_face {
            FrontFace::Clockwise => wgpu::FrontFace::Cw,
            FrontFace::CounterClockwise => wgpu::FrontFace::Ccw,
        }
    }

    /// Maps the cull configuration to the culled `wgpu::Face`,
    /// or `None` when culling is disabled.
    pub fn to_wgpu_cull_mode(&self) -> Option<wgpu::Face> {
        if !self.cull.enabled {
            return None;
        }
        match self.cull.face {
            CullFace::Front => Some(wgpu::Face::Front),
            CullFace::Back => Some(wgpu::Face::Back),
            // DEVIATION: wgpu has no "cull both faces" mode; emulated by
            // skipping the draw. Treated as back-face culling here; the
            // context checks this case and skips such draws.
            CullFace::FrontAndBack => Some(wgpu::Face::Back),
        }
    }

    /// Builds the `wgpu::PrimitiveState` fragment for this render state.
    pub fn to_wgpu_primitive_state(
        &self,
        topology: wgpu::PrimitiveTopology,
    ) -> wgpu::PrimitiveState {
        // DEVIATION: CesiumJS applies polygonOffset (gl.polygonOffset) per
        // render state to bias depth values. wgpu 30's `PrimitiveState` has
        // no polygon-offset field and the render pass exposes no dynamic
        // depth bias, so the CesiumJS API field is retained for fidelity but
        // maps to a no-op here.
        wgpu::PrimitiveState {
            topology,
            strip_index_format: None,
            front_face: self.to_wgpu_front_face(),
            cull_mode: self.to_wgpu_cull_mode(),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        }
    }

    /// Builds the `wgpu::DepthStencilState` fragment for this render state.
    ///
    /// `format` is the depth(-stencil) attachment format of the render pass.
    pub fn to_wgpu_depth_stencil_state(&self, format: wgpu::TextureFormat) -> wgpu::DepthStencilState {
        let stencil_op = |config: StencilOperationConfig| wgpu::StencilFaceState {
            compare: wgpu::CompareFunction::Always,
            fail_op: stencil_operation_to_wgpu(config.fail),
            depth_fail_op: stencil_operation_to_wgpu(config.z_fail),
            pass_op: stencil_operation_to_wgpu(config.z_pass),
        };
        wgpu::DepthStencilState {
            format,
            depth_write_enabled: Some(self.depth_mask),
            depth_compare: Some(if self.depth_test.enabled {
                depth_function_to_wgpu(self.depth_test.func)
            } else {
                wgpu::CompareFunction::Always
            }),
            stencil: wgpu::StencilState {
                front: wgpu::StencilFaceState {
                    compare: if self.stencil_test.enabled {
                        depth_function_to_wgpu(self.stencil_test.front_function)
                    } else {
                        wgpu::CompareFunction::Always
                    },
                    ..stencil_op(self.stencil_test.front_operation)
                },
                back: wgpu::StencilFaceState {
                    compare: if self.stencil_test.enabled {
                        depth_function_to_wgpu(self.stencil_test.back_function)
                    } else {
                        wgpu::CompareFunction::Always
                    },
                    ..stencil_op(self.stencil_test.back_operation)
                },
                read_mask: self.stencil_test.mask,
                write_mask: self.stencil_mask,
            },
            // DEVIATION: CesiumJS sets depthRange (near/far) per draw via
            // gl.depthRange. wgpu exposes this only as a render-pass-level
            // `depth_range` in recent versions; the pipeline state keeps the
            // full [0, 1] range.
            bias: wgpu::DepthBiasState::default(),
        }
    }

    /// Builds the `wgpu::BlendState` for the blending configuration,
    /// or `None` when blending is disabled.
    pub fn to_wgpu_blend_state(&self) -> Option<wgpu::BlendState> {
        if !self.blending.enabled {
            return None;
        }
        Some(wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: blending_factor_to_wgpu(self.blending.function_source_rgb),
                dst_factor: blending_factor_to_wgpu(self.blending.function_destination_rgb),
                operation: blend_equation_to_wgpu(self.blending.equation_rgb),
            },
            alpha: wgpu::BlendComponent {
                src_factor: blending_factor_to_wgpu(self.blending.function_source_alpha),
                dst_factor: blending_factor_to_wgpu(self.blending.function_destination_alpha),
                operation: blend_equation_to_wgpu(self.blending.equation_alpha),
            },
        })
    }

    /// Maps the color mask to `wgpu::ColorWrites`.
    pub fn to_wgpu_write_mask(&self) -> wgpu::ColorWrites {
        let mut mask = wgpu::ColorWrites::empty();
        if self.color_mask.red {
            mask |= wgpu::ColorWrites::RED;
        }
        if self.color_mask.green {
            mask |= wgpu::ColorWrites::GREEN;
        }
        if self.color_mask.blue {
            mask |= wgpu::ColorWrites::BLUE;
        }
        if self.color_mask.alpha {
            mask |= wgpu::ColorWrites::ALPHA;
        }
        mask
    }

    /// Builds the `wgpu::ColorTargetState` fragment for this render state.
    ///
    /// `format` is the color attachment format of the render pass.
    pub fn to_wgpu_color_target_state(&self, format: wgpu::TextureFormat) -> wgpu::ColorTargetState {
        wgpu::ColorTargetState {
            format,
            blend: self.to_wgpu_blend_state(),
            write_mask: self.to_wgpu_write_mask(),
        }
    }

    /// Builds the `wgpu::MultisampleState` fragment for this render state.
    ///
    /// DEVIATION: CesiumJS sample coverage (`gl.sampleCoverage`) has no wgpu
    /// equivalent; MSAA count is fixed at 1 for the smoke path (multisampled
    /// attachments will override this when implemented).
    pub fn to_wgpu_multisample_state(&self) -> wgpu::MultisampleState {
        wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        }
    }

    /// Maps the WebGL-style primitive type constant to a
    /// `wgpu::PrimitiveTopology`, if representable.
    pub fn primitive_type_to_wgpu_topology(primitive_type: u32) -> Option<wgpu::PrimitiveTopology> {
        use cesium_core::webgl_constants::WebGLConstants;
        match primitive_type {
            WebGLConstants::POINTS => Some(wgpu::PrimitiveTopology::PointList),
            WebGLConstants::LINES => Some(wgpu::PrimitiveTopology::LineList),
            WebGLConstants::LINE_STRIP => Some(wgpu::PrimitiveTopology::LineStrip),
            WebGLConstants::TRIANGLES => Some(wgpu::PrimitiveTopology::TriangleList),
            WebGLConstants::TRIANGLE_STRIP => Some(wgpu::PrimitiveTopology::TriangleStrip),
            // DEVIATION: LINE_LOOP and TRIANGLE_FAN have no wgpu equivalent;
            // callers must convert the geometry (e.g. fan → indexed list).
            _ => None,
        }
    }
}

impl Hash for RenderState {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.front_face.hash(state);
        self.cull.hash(state);
        // line_width as bits
        self.line_width.to_bits().hash(state);
        self.polygon_offset.enabled.hash(state);
        self.polygon_offset.factor.to_bits().hash(state);
        self.polygon_offset.units.to_bits().hash(state);
        self.scissor_test.enabled.hash(state);
        self.scissor_test.rectangle.x.to_bits().hash(state);
        self.scissor_test.rectangle.y.to_bits().hash(state);
        self.scissor_test.rectangle.width.to_bits().hash(state);
        self.scissor_test.rectangle.height.to_bits().hash(state);
        self.depth_range.near.to_bits().hash(state);
        self.depth_range.far.to_bits().hash(state);
        self.depth_test.hash(state);
        self.color_mask.hash(state);
        self.depth_mask.hash(state);
        self.stencil_mask.hash(state);
        self.blending.enabled.hash(state);
        self.blending.equation_rgb.hash(state);
        self.blending.equation_alpha.hash(state);
        self.blending.function_source_rgb.hash(state);
        self.blending.function_source_alpha.hash(state);
        self.blending.function_destination_rgb.hash(state);
        self.blending.function_destination_alpha.hash(state);
        // Blend color as bits
        for c in &self.blending.color {
            c.to_bits().hash(state);
        }
        self.stencil_test.hash(state);
        self.sample_coverage.enabled.hash(state);
        self.sample_coverage.value.to_bits().hash(state);
        self.sample_coverage.invert.hash(state);
        // Viewport
        if let Some(ref vp) = self.viewport {
            vp.x.to_bits().hash(state);
            vp.y.to_bits().hash(state);
            vp.width.to_bits().hash(state);
            vp.height.to_bits().hash(state);
        }
    }
}
