//! Ported from `packages/engine/Source/Core/ComponentDatatype.js`.
//!
//! WebGL component datatypes. Components are intrinsics which form attributes,
//! which form vertices.

use crate::webgl_constants::WebGLConstants;

/// WebGL component datatypes. Components are intrinsics, which form attributes,
/// which form vertices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ComponentDatatype {
    /// 8-bit signed byte (`Int8Array`).
    Byte = WebGLConstants::BYTE,
    /// 8-bit unsigned byte (`Uint8Array`).
    UnsignedByte = WebGLConstants::UNSIGNED_BYTE,
    /// 16-bit signed short (`Int16Array`).
    Short = WebGLConstants::SHORT,
    /// 16-bit unsigned short (`Uint16Array`).
    UnsignedShort = WebGLConstants::UNSIGNED_SHORT,
    /// 32-bit signed int (`Int32Array`).
    Int = WebGLConstants::INT,
    /// 32-bit unsigned int (`Uint32Array`).
    UnsignedInt = WebGLConstants::UNSIGNED_INT,
    /// 32-bit floating-point (`Float32Array`).
    Float = WebGLConstants::FLOAT,
    /// 64-bit floating-point (`Float64Array`). Emulated via `encodeAttribute` in
    /// WebGL (not natively supported).
    Double = WebGLConstants::DOUBLE,
}

impl ComponentDatatype {
    /// Returns the size, in bytes, of the corresponding datatype.
    pub fn size_in_bytes(self) -> usize {
        match self {
            ComponentDatatype::Byte => 1,
            ComponentDatatype::UnsignedByte => 1,
            ComponentDatatype::Short => 2,
            ComponentDatatype::UnsignedShort => 2,
            ComponentDatatype::Int => 4,
            ComponentDatatype::UnsignedInt => 4,
            ComponentDatatype::Float => 4,
            ComponentDatatype::Double => 8,
        }
    }

    /// Validates that the provided component datatype is valid.
    pub fn validate(component_datatype: u32) -> bool {
        matches!(
            component_datatype,
            WebGLConstants::BYTE
                | WebGLConstants::UNSIGNED_BYTE
                | WebGLConstants::SHORT
                | WebGLConstants::UNSIGNED_SHORT
                | WebGLConstants::INT
                | WebGLConstants::UNSIGNED_INT
                | WebGLConstants::FLOAT
                | WebGLConstants::DOUBLE
        )
    }

    /// Get the `ComponentDatatype` from its name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "BYTE" => Some(ComponentDatatype::Byte),
            "UNSIGNED_BYTE" => Some(ComponentDatatype::UnsignedByte),
            "SHORT" => Some(ComponentDatatype::Short),
            "UNSIGNED_SHORT" => Some(ComponentDatatype::UnsignedShort),
            "INT" => Some(ComponentDatatype::Int),
            "UNSIGNED_INT" => Some(ComponentDatatype::UnsignedInt),
            "FLOAT" => Some(ComponentDatatype::Float),
            "DOUBLE" => Some(ComponentDatatype::Double),
            _ => None,
        }
    }

    /// Try to convert from a raw `u32` value.
    pub fn try_from_u32(value: u32) -> Option<Self> {
        match value {
            WebGLConstants::BYTE => Some(ComponentDatatype::Byte),
            WebGLConstants::UNSIGNED_BYTE => Some(ComponentDatatype::UnsignedByte),
            WebGLConstants::SHORT => Some(ComponentDatatype::Short),
            WebGLConstants::UNSIGNED_SHORT => Some(ComponentDatatype::UnsignedShort),
            WebGLConstants::INT => Some(ComponentDatatype::Int),
            WebGLConstants::UNSIGNED_INT => Some(ComponentDatatype::UnsignedInt),
            WebGLConstants::FLOAT => Some(ComponentDatatype::Float),
            WebGLConstants::DOUBLE => Some(ComponentDatatype::Double),
            _ => None,
        }
    }

    /// Converts a single normalized integer value to a floating-point value in
    /// the range [-1, 1] (for signed types) or [0, 1] (for unsigned types),
    /// following the WebGL and glTF normalization conventions.
    pub fn dequantize(value: f64, component_datatype: Self) -> f64 {
        match component_datatype {
            ComponentDatatype::Byte => (value / 127.0).max(-1.0),
            ComponentDatatype::UnsignedByte => value / 255.0,
            ComponentDatatype::Short => (value / 32767.0).max(-1.0),
            ComponentDatatype::UnsignedShort => value / 65535.0,
            ComponentDatatype::Int => (value / 2_147_483_647.0).max(-1.0),
            ComponentDatatype::UnsignedInt => value / 4_294_967_295.0,
            // FLOAT and DOUBLE are not integer types
            _ => value,
        }
    }
}
