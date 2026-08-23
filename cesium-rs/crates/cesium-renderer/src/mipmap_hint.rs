//! Ported from `packages/engine/Source/Renderer/MipmapHint.js`.
//!
//! Mipmap generation quality hints.

use cesium_core::webgl_constants::WebGLConstants;

/// Mipmap generation quality hints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MipmapHint {
    /// No preference.
    DontCare = WebGLConstants::DONT_CARE,
    /// Fastest generation.
    Fastest = WebGLConstants::FASTEST,
    /// Best quality.
    Nicest = WebGLConstants::NICEST,
}

impl MipmapHint {
    /// Validates a mipmap hint value.
    pub fn validate(hint: MipmapHint) -> bool {
        matches!(hint, MipmapHint::DontCare | MipmapHint::Fastest | MipmapHint::Nicest)
    }
}
