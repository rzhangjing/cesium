//! AttributeCompression - oct encoding, texture coordinate compression, zigzag decode.
//! Maps to CesiumJS `Core/AttributeCompression.js`

use crate::ellipsoid::normalize_cartesian3;
use crate::math_utils;
use glam::{DVec2, DVec3};

const RIGHT_SHIFT8: f64 = 1.0 / 256.0;
const LEFT_SHIFT16: f64 = 65536.0;
const LEFT_SHIFT8: f64 = 256.0;

/// Encodes a normalized vector into 2 SNORM values in the range [0, range_max]
/// following the 'oct' encoding.
///
/// Maps to `AttributeCompression.octEncodeInRange`
pub fn oct_encode_in_range(vector: DVec3, range_max: f64) -> DVec2 {
    let denom = vector.x.abs() + vector.y.abs() + vector.z.abs();
    let mut x = vector.x / denom;
    let mut y = vector.y / denom;

    if vector.z < 0.0 {
        let old_x = x;
        let old_y = y;
        x = (1.0 - old_y.abs()) * math_utils::sign_not_zero(old_x);
        y = (1.0 - old_x.abs()) * math_utils::sign_not_zero(old_y);
    }

    DVec2::new(
        math_utils::to_snorm(x, range_max),
        math_utils::to_snorm(y, range_max),
    )
}

/// Encodes a normalized vector into 2 SNORM values in the range [0, 255].
///
/// Maps to `AttributeCompression.octEncode`
pub fn oct_encode(vector: DVec3) -> DVec2 {
    oct_encode_in_range(vector, 255.0)
}

/// Encodes a normalized vector into 4 bytes (Cartesian4 representation).
/// Returns (x, y, z, w) as f64 values in [0, 255].
///
/// Maps to `AttributeCompression.octEncodeToCartesian4`
pub fn oct_encode_to_cartesian4(vector: DVec3) -> (f64, f64, f64, f64) {
    let encoded = oct_encode_in_range(vector, 65535.0);
    let x = force_uint8(encoded.x * RIGHT_SHIFT8);
    let y = force_uint8(encoded.x);
    let z = force_uint8(encoded.y * RIGHT_SHIFT8);
    let w = force_uint8(encoded.y);
    (x, y, z, w)
}

/// Decodes a unit-length vector from 'oct' encoding in range [0, range_max].
///
/// Maps to `AttributeCompression.octDecodeInRange`
pub fn oct_decode_in_range(x: f64, y: f64, range_max: f64) -> DVec3 {
    let mut rx = math_utils::from_snorm(x, range_max);
    let mut ry = math_utils::from_snorm(y, range_max);
    let rz = 1.0 - (rx.abs() + ry.abs());

    if rz < 0.0 {
        let old_vx = rx;
        rx = (1.0 - ry.abs()) * math_utils::sign_not_zero(old_vx);
        ry = (1.0 - old_vx.abs()) * math_utils::sign_not_zero(ry);
    }

    normalize_cartesian3(DVec3::new(rx, ry, rz))
}

/// Decodes a unit-length vector from 2-byte 'oct' encoding.
///
/// Maps to `AttributeCompression.octDecode`
pub fn oct_decode(x: f64, y: f64) -> DVec3 {
    oct_decode_in_range(x, y, 255.0)
}

/// Decodes a unit-length vector from 4-byte 'oct' encoding.
///
/// Maps to `AttributeCompression.octDecodeFromCartesian4`
pub fn oct_decode_from_cartesian4(x: f64, y: f64, z: f64, w: f64) -> DVec3 {
    let x_oct16 = x * LEFT_SHIFT8 + y;
    let y_oct16 = z * LEFT_SHIFT8 + w;
    oct_decode_in_range(x_oct16, y_oct16, 65535.0)
}

/// Packs an oct-encoded vector (2 bytes) into a single float.
///
/// Maps to `AttributeCompression.octPackFloat`
pub fn oct_pack_float(encoded: DVec2) -> f64 {
    256.0 * encoded.x + encoded.y
}

/// Encodes a normalized vector into a single float (2-byte oct encoding packed).
///
/// Maps to `AttributeCompression.octEncodeFloat`
pub fn oct_encode_float(vector: DVec3) -> f64 {
    let encoded = oct_encode(vector);
    oct_pack_float(encoded)
}

/// Decodes a unit-length vector from a float-packed oct encoding.
///
/// Maps to `AttributeCompression.octDecodeFloat`
pub fn oct_decode_float(value: f64) -> DVec3 {
    let temp = value / 256.0;
    let x = temp.floor();
    let y = (temp - x) * 256.0;
    oct_decode(x, y)
}

/// Encodes three normalized vectors into two floats (packed oct encoding).
///
/// Maps to `AttributeCompression.octPack`
pub fn oct_pack(v1: DVec3, v2: DVec3, v3: DVec3) -> DVec2 {
    let encoded1 = oct_encode_float(v1);
    let encoded2 = oct_encode_float(v2);
    let encoded3 = oct_encode(v3);
    DVec2::new(
        LEFT_SHIFT16 * encoded3.x + encoded1,
        LEFT_SHIFT16 * encoded3.y + encoded2,
    )
}

/// Decodes three unit-length vectors from two packed floats.
///
/// Maps to `AttributeCompression.octUnpack`
pub fn oct_unpack(packed: DVec2) -> (DVec3, DVec3, DVec3) {
    let temp = packed.x / LEFT_SHIFT16;
    let x = temp.floor();
    let encoded_float1 = (temp - x) * LEFT_SHIFT16;

    let temp = packed.y / LEFT_SHIFT16;
    let y = temp.floor();
    let encoded_float2 = (temp - y) * LEFT_SHIFT16;

    let v1 = oct_decode_float(encoded_float1);
    let v2 = oct_decode_float(encoded_float2);
    let v3 = oct_decode(x, y);
    (v1, v2, v3)
}

/// Compresses texture coordinates into a single float (12-bit precision per component).
///
/// Maps to `AttributeCompression.compressTextureCoordinates`
pub fn compress_texture_coordinates(texture_coordinates: DVec2) -> f64 {
    let x = (texture_coordinates.x * 4095.0) as i64;
    let y = (texture_coordinates.y * 4095.0) as i64;
    4096.0 * x as f64 + y as f64
}

/// Decompresses texture coordinates from a single float.
///
/// Maps to `AttributeCompression.decompressTextureCoordinates`
pub fn decompress_texture_coordinates(compressed: f64) -> DVec2 {
    let temp = compressed / 4096.0;
    let x_zero_to_4095 = temp.floor();
    DVec2::new(
        x_zero_to_4095 / 4095.0,
        (compressed - x_zero_to_4095 * 4096.0) / 4095.0,
    )
}

fn zig_zag_decode(value: u16) -> i32 {
    let v = value as i32;
    (v >> 1) ^ -(v & 1)
}

/// Decodes delta and ZigZag encoded vertices in place.
///
/// Maps to `AttributeCompression.zigZagDeltaDecode`
pub fn zig_zag_delta_decode(u_buffer: &mut [u16], v_buffer: &mut [u16], mut height_buffer: Option<&mut [u16]>) {
    let count = u_buffer.len();
    let mut u: i32 = 0;
    let mut v: i32 = 0;
    let mut height: i32 = 0;

    for i in 0..count {
        u += zig_zag_decode(u_buffer[i]);
        v += zig_zag_decode(v_buffer[i]);
        u_buffer[i] = u as u16;
        v_buffer[i] = v as u16;

        if let Some(ref mut hb) = height_buffer {
            height += zig_zag_decode(hb[i]);
            hb[i] = height as u16;
        }
    }
}

/// WebGL component datatypes. Components are intrinsics,
/// which form attributes, which form vertices.
///
/// Maps to CesiumJS `Core/ComponentDatatype.js`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentDatatype {
    /// 8-bit signed byte (gl.BYTE = 0x1400)
    Byte,
    /// 8-bit unsigned byte (gl.UNSIGNED_BYTE = 0x1401)
    UnsignedByte,
    /// 16-bit signed short (gl.SHORT = 0x1402)
    Short,
    /// 16-bit unsigned short (gl.UNSIGNED_SHORT = 0x1403)
    UnsignedShort,
    /// 32-bit signed int (gl.INT = 0x1404)
    Int,
    /// 32-bit unsigned int (gl.UNSIGNED_INT = 0x1405)
    UnsignedInt,
    /// 32-bit float (gl.FLOAT = 0x1406)
    Float,
    /// 64-bit float (gl.DOUBLE = 0x140A)
    Double,
}

impl ComponentDatatype {
    /// Returns the size in bytes of this component datatype.
    ///
    /// Maps to CesiumJS `ComponentDatatype.getSizeInBytes`
    pub fn size_in_bytes(self) -> usize {
        match self {
            Self::Byte | Self::UnsignedByte => 1,
            Self::Short | Self::UnsignedShort => 2,
            Self::Int | Self::UnsignedInt | Self::Float => 4,
            Self::Double => 8,
        }
    }

    /// Returns the WebGL constant value for this component datatype.
    pub fn gl_value(self) -> u32 {
        match self {
            Self::Byte => 0x1400,
            Self::UnsignedByte => 0x1401,
            Self::Short => 0x1402,
            Self::UnsignedShort => 0x1403,
            Self::Int => 0x1404,
            Self::UnsignedInt => 0x1405,
            Self::Float => 0x1406,
            Self::Double => 0x140A,
        }
    }

    /// Validates that the provided value is a valid ComponentDatatype.
    /// In Rust, any enum variant is inherently valid.
    ///
    /// Maps to CesiumJS `ComponentDatatype.validate`
    pub fn validate(self) -> bool {
        true
    }

    /// Returns the ComponentDatatype for the provided name string.
    ///
    /// Maps to CesiumJS `ComponentDatatype.fromName`
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "BYTE" => Some(Self::Byte),
            "UNSIGNED_BYTE" => Some(Self::UnsignedByte),
            "SHORT" => Some(Self::Short),
            "UNSIGNED_SHORT" => Some(Self::UnsignedShort),
            "INT" => Some(Self::Int),
            "UNSIGNED_INT" => Some(Self::UnsignedInt),
            "FLOAT" => Some(Self::Float),
            "DOUBLE" => Some(Self::Double),
            _ => None,
        }
    }

    /// Returns the ComponentDatatype from a WebGL constant value.
    pub fn from_gl_value(value: u32) -> Option<Self> {
        match value {
            0x1400 => Some(Self::Byte),
            0x1401 => Some(Self::UnsignedByte),
            0x1402 => Some(Self::Short),
            0x1403 => Some(Self::UnsignedShort),
            0x1404 => Some(Self::Int),
            0x1405 => Some(Self::UnsignedInt),
            0x1406 => Some(Self::Float),
            0x140A => Some(Self::Double),
            _ => None,
        }
    }

    /// Divisor used for dequantization (integer types only).
    pub fn divisor(self) -> f64 {
        match self {
            Self::Byte => 127.0,
            Self::UnsignedByte => 255.0,
            Self::Short => 32767.0,
            Self::UnsignedShort => 65535.0,
            Self::Int => 2147483647.0,
            Self::UnsignedInt => 4294967295.0,
            Self::Float | Self::Double => 1.0,
        }
    }
}

/// Index datatype for geometry indices.
///
/// Maps to CesiumJS `Core/IndexDatatype.js`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndexDatatype {
    /// 8-bit unsigned byte (gl.UNSIGNED_BYTE = 0x1401)
    UnsignedByte,
    /// 16-bit unsigned short (gl.UNSIGNED_SHORT = 0x1403)
    UnsignedShort,
    /// 32-bit unsigned int (gl.UNSIGNED_INT = 0x1405)
    UnsignedInt,
}

impl IndexDatatype {
    /// 64K = 65536, the maximum number of vertices for UNSIGNED_SHORT indices.
    pub const SIXTY_FOUR_KILOBYTES: u64 = 65536;

    /// Returns the size in bytes of this index datatype.
    ///
    /// Maps to CesiumJS `IndexDatatype.getSizeInBytes`
    pub fn size_in_bytes(self) -> usize {
        match self {
            Self::UnsignedByte => 1,
            Self::UnsignedShort => 2,
            Self::UnsignedInt => 4,
        }
    }

    /// Returns the WebGL constant value for this index datatype.
    pub fn gl_value(self) -> u32 {
        match self {
            Self::UnsignedByte => 0x1401,
            Self::UnsignedShort => 0x1403,
            Self::UnsignedInt => 0x1405,
        }
    }

    /// Validates that the provided value is a valid IndexDatatype.
    ///
    /// Maps to CesiumJS `IndexDatatype.validate`
    pub fn validate(self) -> bool {
        true
    }

    /// Returns the IndexDatatype for the provided name string.
    ///
    /// Maps to CesiumJS `IndexDatatype.fromName`
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "UNSIGNED_BYTE" => Some(Self::UnsignedByte),
            "UNSIGNED_SHORT" => Some(Self::UnsignedShort),
            "UNSIGNED_INT" => Some(Self::UnsignedInt),
            _ => None,
        }
    }

    /// Returns the IndexDatatype from a WebGL constant value.
    pub fn from_gl_value(value: u32) -> Option<Self> {
        match value {
            0x1401 => Some(Self::UnsignedByte),
            0x1403 => Some(Self::UnsignedShort),
            0x1405 => Some(Self::UnsignedInt),
            _ => None,
        }
    }

    /// Determines the appropriate IndexDatatype for the given number of vertices.
    /// If numberOfVertices >= 65536, returns UnsignedInt; otherwise UnsignedShort.
    ///
    /// Maps to CesiumJS `IndexDatatype.createTypedArray` logic
    pub fn for_vertex_count(number_of_vertices: u64) -> Self {
        if number_of_vertices >= Self::SIXTY_FOUR_KILOBYTES {
            Self::UnsignedInt
        } else {
            Self::UnsignedShort
        }
    }
}

/// Dequantizes a typed array of i32 values into f32 (as f64 here).
///
/// Maps to `AttributeCompression.dequantize`
pub fn dequantize(
    typed_array: &[i32],
    component_datatype: ComponentDatatype,
    components_per_attribute: usize,
    count: usize,
) -> Vec<f64> {
    let divisor = component_datatype.divisor();
    let mut result = vec![0.0_f64; count * components_per_attribute];

    for i in 0..count {
        for j in 0..components_per_attribute {
            let index = i * components_per_attribute + j;
            result[index] = (typed_array[index] as f64 / divisor).max(-1.0);
        }
    }
    result
}

/// Encodes RGB values at 8-bit precision into a single float (0xFFFFFF representation).
///
/// Maps to `AttributeCompression.encodeRGB8`
pub fn encode_rgb8(red: f64, green: f64, blue: f64) -> f64 {
    let r = (math_utils::clamp(red * 255.0, 0.0, 255.0)).round();
    let g = (math_utils::clamp(green * 255.0, 0.0, 255.0)).round();
    let b = (math_utils::clamp(blue * 255.0, 0.0, 255.0)).round();
    r * LEFT_SHIFT16 + g * LEFT_SHIFT8 + b
}

/// Decodes RGB values at 8-bit precision from a single float.
/// Returns (red, green, blue) in [0, 1].
///
/// Maps to `AttributeCompression.decodeRGB8`
pub fn decode_rgb8(encoded: f64) -> (f64, f64, f64) {
    let encoded = encoded.floor() as i64;
    let red = ((encoded >> 16) & 255) as f64 / 255.0;
    let green = ((encoded >> 8) & 255) as f64 / 255.0;
    let blue = (encoded & 255) as f64 / 255.0;
    (red, green, blue)
}

/// Decodes RGB565-encoded colors into normalized RGB values.
///
/// Maps to `AttributeCompression.decodeRGB565`
pub fn decode_rgb565(typed_array: &[u16]) -> Vec<f64> {
    let count = typed_array.len();
    let mut result = vec![0.0_f64; count * 3];

    let mask5: u16 = (1 << 5) - 1;
    let mask6: u16 = (1 << 6) - 1;
    let normalize5 = 1.0 / 31.0;
    let normalize6 = 1.0 / 63.0;

    for i in 0..count {
        let value = typed_array[i];
        let red = (value >> 11) as f64;
        let green = ((value >> 5) & mask6) as f64;
        let blue = (value & mask5) as f64;

        let offset = 3 * i;
        result[offset] = red * normalize5;
        result[offset + 1] = green * normalize6;
        result[offset + 2] = blue * normalize5;
    }
    result
}

/// Forces a value into uint8 range (mimics JS Uint8Array truncation).
#[inline]
fn force_uint8(value: f64) -> f64 {
    (value as u32 & 0xFF) as f64
}
