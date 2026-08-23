//! Ported from `packages/engine/Source/Renderer/ShaderDestination.js`.
//!
//! Bit flags for shader stage targeting.

/// Bit flags describing whether a variable should be added to the
/// vertex shader, the fragment shader, or both (or none).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ShaderDestination {
    /// No shader stage.
    None = 0,
    /// Vertex shader only.
    Vertex = 1,
    /// Fragment shader only.
    Fragment = 2,
    /// Both vertex and fragment shaders.
    Both = 3,
}

impl ShaderDestination {
    /// Check if the destination includes the vertex shader.
    pub fn includes_vertex_shader(self) -> bool {
        (self as u8) & (ShaderDestination::Vertex as u8) != 0
    }

    /// Check if the destination includes the fragment shader.
    pub fn includes_fragment_shader(self) -> bool {
        (self as u8) & (ShaderDestination::Fragment as u8) != 0
    }
}
