//! Ported from `packages/engine/Source/Core/webGLConstantToGlslType.js`.
//!
//! Maps WebGL constants to their GLSL type names.

use crate::webgl_constants::WebGLConstants;

/// Converts a WebGL constant to its GLSL type name.
pub fn webgl_constant_to_glsl_type(webgl_value: u32) -> Option<&'static str> {
    match webgl_value {
        x if x == WebGLConstants::FLOAT => Some("float"),
        x if x == WebGLConstants::FLOAT_VEC2 => Some("vec2"),
        x if x == WebGLConstants::FLOAT_VEC3 => Some("vec3"),
        x if x == WebGLConstants::FLOAT_VEC4 => Some("vec4"),
        x if x == WebGLConstants::FLOAT_MAT2 => Some("mat2"),
        x if x == WebGLConstants::FLOAT_MAT3 => Some("mat3"),
        x if x == WebGLConstants::FLOAT_MAT4 => Some("mat4"),
        x if x == WebGLConstants::SAMPLER_2D => Some("sampler2D"),
        x if x == WebGLConstants::BOOL => Some("bool"),
        _ => None,
    }
}
