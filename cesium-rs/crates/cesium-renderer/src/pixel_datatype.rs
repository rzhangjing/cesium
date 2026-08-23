//! Ported from `packages/engine/Source/Renderer/PixelDatatype.js`.
//!
//! Pixel data type enums for textures and renderbuffers.

use cesium_core::webgl_constants::WebGLConstants;

/// Pixel data types for texture and renderbuffer formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PixelDatatype {
    /// Unsigned byte (8-bit).
    UnsignedByte = WebGLConstants::UNSIGNED_BYTE,
    /// Unsigned short (16-bit).
    UnsignedShort = WebGLConstants::UNSIGNED_SHORT,
    /// Unsigned int (32-bit).
    UnsignedInt = WebGLConstants::UNSIGNED_INT,
    /// Float (32-bit IEEE).
    Float = WebGLConstants::FLOAT,
    /// Half float (16-bit IEEE).
    HalfFloat = WebGLConstants::HALF_FLOAT,
    /// Unsigned short 5-6-5 packed.
    UnsignedShort565 = WebGLConstants::UNSIGNED_SHORT_5_6_5,
    /// Unsigned short 5-5-5-1 packed.
    UnsignedShort5551 = WebGLConstants::UNSIGNED_SHORT_5_5_5_1,
    /// Unsigned int 24-8 packed (depth-stencil).
    UnsignedInt248 = WebGLConstants::UNSIGNED_INT_24_8,
    /// Unsigned int 10F 11F 11F reversed.
    UnsignedInt10f11f11fRev = WebGLConstants::UNSIGNED_INT_10F_11F_11F_REV,
    /// Unsigned short 4-4-4-4 packed (deprecated in WebGL2).
    UnsignedShort4444 = WebGLConstants::UNSIGNED_SHORT_4_4_4_4,
    /// Unsigned int 5-9-9-9 reversed.
    UnsignedInt5999Rev = WebGLConstants::UNSIGNED_INT_5_9_9_9_REV,
}

impl PixelDatatype {
    /// Returns the size in bytes of a single component.
    pub fn size_in_bytes(&self) -> usize {
        match self {
            PixelDatatype::UnsignedByte => 1,
            PixelDatatype::UnsignedShort | PixelDatatype::HalfFloat
            | PixelDatatype::UnsignedShort565 | PixelDatatype::UnsignedShort5551
            | PixelDatatype::UnsignedShort4444 => 2,
            PixelDatatype::UnsignedInt | PixelDatatype::Float
            | PixelDatatype::UnsignedInt248 | PixelDatatype::UnsignedInt10f11f11fRev
            | PixelDatatype::UnsignedInt5999Rev => 4,
        }
    }

    /// Whether this is a packed format (multiple components in one value).
    pub fn is_packed(&self) -> bool {
        matches!(
            self,
            PixelDatatype::UnsignedShort565
                | PixelDatatype::UnsignedShort5551
                | PixelDatatype::UnsignedInt248
                | PixelDatatype::UnsignedInt10f11f11fRev
                | PixelDatatype::UnsignedShort4444
                | PixelDatatype::UnsignedInt5999Rev
        )
    }
}
