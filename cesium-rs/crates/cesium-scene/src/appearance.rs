//! Ported from `packages/engine/Source/Scene/Appearance.js`.
//!
//! Base class for appearances that define how a primitive is shaded.

/// Base class for appearances that define how a primitive is shaded.
///
/// An appearance defines the GLSL vertex and fragment shaders, as well as
/// the render state (depth test, blending, etc.).
pub struct Appearance {
    /// Whether the appearance is translucent.
    pub translucent: bool,
    /// Whether to close the primitive.
    pub closed: bool,
    /// The vertex shader source.
    pub vertex_shader_source: String,
    /// The fragment shader source.
    pub fragment_shader_source: String,
    /// The material (if any).
    pub material_name: Option<String>,
    /// Whether to render above/below.
    pub render_state: RenderState,
}

/// The render state for an appearance.
pub struct RenderState {
    /// Whether depth testing is enabled.
    pub depth_test: bool,
    /// Whether depth writing is enabled.
    pub depth_mask: bool,
    /// Whether blending is enabled.
    pub blending: bool,
}

impl RenderState {
    /// Creates a default render state.
    pub fn new() -> Self {
        Self { depth_test: true, depth_mask: true, blending: false }
    }
}

impl Default for RenderState {
    fn default() -> Self { Self::new() }
}

impl Appearance {
    /// Creates a new appearance.
    pub fn new() -> Self {
        Self {
            translucent: false,
            closed: false,
            vertex_shader_source: String::new(),
            fragment_shader_source: String::new(),
            material_name: None,
            render_state: RenderState::default(),
        }
    }
}

impl Default for Appearance {
    fn default() -> Self { Self::new() }
}
