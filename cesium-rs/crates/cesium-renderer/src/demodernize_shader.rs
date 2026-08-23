//! Ported from `packages/engine/Source/Renderer/demodernizeShader.js`.
//!
//! Converts GLSL 300 ES shaders back to GLSL 100 for legacy contexts.

/// Converts GLSL 300 ES shader source to GLSL 100 compatibility.
///
/// DEVIATION: With wgpu (which uses WGSL or GLSL 300 ES via naga), this
/// function is largely unnecessary. Kept for API completeness.
pub fn demodernize_shader(source: &str) -> String {
    // DEVIATION: wgpu/naga handles shader translation; this is a no-op.
    source.to_string()
}
