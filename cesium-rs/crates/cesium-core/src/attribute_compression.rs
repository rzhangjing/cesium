//! Attribute compression and decompression functions.
//!
//! Mirrors `packages/engine/Source/Core/AttributeCompression.js`.

use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::cartesian4::Cartesian4;
use crate::developer_error::throw_developer_error;
use crate::math::CesiumMath;

const RIGHT_SHIFT8: f64 = 1.0 / 256.0;
const LEFT_SHIFT16: f64 = 65536.0;
const LEFT_SHIFT8: f64 = 256.0;

/// Compression/decompression utilities for vertex attributes.
pub struct AttributeCompression;

// ---------------------------------------------------------------------------
// Oct encoding / decoding
// ---------------------------------------------------------------------------

impl AttributeCompression {
    /// Encodes a normalized vector into 2 SNORM values in `[0, rangeMax]`
    /// using the *oct* encoding.
    ///
    /// Reference: Cigolle et al 2014 – "A Survey of Efficient Representations
    /// of Independent Unit Vectors".
    /// # Panics
    /// In debug builds, panics with `DeveloperError` when `vector` is not
    /// normalized (port of the JS debug guard).
    pub fn oct_encode_in_range<'a>(
        vector: &Cartesian3,
        range_max: f64,
        result: &'a mut Cartesian2,
    ) -> &'a mut Cartesian2 {
        //>>includeStart('debug', pragmas.debug);
        if cfg!(debug_assertions) {
            let mag_squared = Cartesian3::magnitude_squared(vector);
            if (mag_squared - 1.0).abs() > CesiumMath::EPSILON6 {
                throw_developer_error("vector must be normalized.");
            }
        }
        //>>includeEnd('debug');

        let denom = vector.x.abs() + vector.y.abs() + vector.z.abs();
        result.x = vector.x / denom;
        result.y = vector.y / denom;

        if vector.z < 0.0 {
            let x = result.x;
            let y = result.y;
            result.x = (1.0 - y.abs()) * CesiumMath::sign_not_zero(x);
            result.y = (1.0 - x.abs()) * CesiumMath::sign_not_zero(y);
        }

        result.x = CesiumMath::to_snorm(result.x, Some(range_max));
        result.y = CesiumMath::to_snorm(result.y, Some(range_max));
        result
    }

    /// Encodes a normalized vector into 2 SNORM values in `[0, 255]`.
    pub fn oct_encode<'a>(vector: &Cartesian3, result: &'a mut Cartesian2) -> &'a mut Cartesian2 {
        Self::oct_encode_in_range(vector, 255.0, result)
    }

    /// Encodes a normalized vector into 4 byte components stored in a
    /// [`Cartesian4`] (two 16-bit oct values split into high/low bytes).
    pub fn oct_encode_to_cartesian4<'a>(
        vector: &Cartesian3,
        result: &'a mut Cartesian4,
    ) -> &'a mut Cartesian4 {
        let mut scratch = Cartesian2::default();
        Self::oct_encode_in_range(vector, 65535.0, &mut scratch);
        result.x = force_uint8(scratch.x * RIGHT_SHIFT8);
        result.y = force_uint8(scratch.x);
        result.z = force_uint8(scratch.y * RIGHT_SHIFT8);
        result.w = force_uint8(scratch.y);
        result
    }

    /// Decodes a unit-length vector from *oct* encoding in `[0, rangeMax]`.
    /// # Panics
    /// In debug builds, panics with `DeveloperError` when `x` or `y` is not
    /// an unsigned normalized integer in `[0, range_max]`.
    pub fn oct_decode_in_range<'a>(
        x: f64,
        y: f64,
        range_max: f64,
        result: &'a mut Cartesian3,
    ) -> &'a mut Cartesian3 {
        //>>includeStart('debug', pragmas.debug);
        if cfg!(debug_assertions) {
            if x < 0.0 || x > range_max || y < 0.0 || y > range_max {
                throw_developer_error(&format!(
                    "x and y must be unsigned normalized integers between 0 and {}",
                    crate::check::js_number_to_string(range_max)
                ));
            }
        }
        //>>includeEnd('debug');

        result.x = CesiumMath::from_snorm(x, Some(range_max));
        result.y = CesiumMath::from_snorm(y, Some(range_max));
        result.z = 1.0 - (result.x.abs() + result.y.abs());

        if result.z < 0.0 {
            let old_vx = result.x;
            result.x = (1.0 - result.y.abs()) * CesiumMath::sign_not_zero(old_vx);
            result.y = (1.0 - old_vx.abs()) * CesiumMath::sign_not_zero(result.y);
        }

        // Port of `Cartesian3.normalize(result, result)` (copies the input to
        // satisfy the borrow checker; JS aliases both arguments).
        let unnormalized = *result;
        Cartesian3::normalize(&unnormalized, result);
        result
    }

    /// Decodes a unit-length vector from 2-byte *oct* encoding.
    pub fn oct_decode<'a>(x: f64, y: f64, result: &'a mut Cartesian3) -> &'a mut Cartesian3 {
        Self::oct_decode_in_range(x, y, 255.0, result)
    }

    /// Decodes a unit-length vector from 4-byte *oct* encoding stored in a
    /// [`Cartesian4`].
    pub fn oct_decode_from_cartesian4<'a>(
        encoded: &Cartesian4,
        result: &'a mut Cartesian3,
    ) -> &'a mut Cartesian3 {
        //>>includeStart('debug', pragmas.debug);
        if cfg!(debug_assertions) {
            if encoded.x < 0.0
                || encoded.x > 255.0
                || encoded.y < 0.0
                || encoded.y > 255.0
                || encoded.z < 0.0
                || encoded.z > 255.0
                || encoded.w < 0.0
                || encoded.w > 255.0
            {
                throw_developer_error(
                    "x, y, z, and w must be unsigned normalized integers between 0 and 255",
                );
            }
        }
        //>>includeEnd('debug');

        let x_oct16 = encoded.x * LEFT_SHIFT8 + encoded.y;
        let y_oct16 = encoded.z * LEFT_SHIFT8 + encoded.w;
        Self::oct_decode_in_range(x_oct16, y_oct16, 65535.0, result)
    }

    // --- float packing -----------------------------------------------------

    /// Packs an oct-encoded vector into a single floating-point number.
    pub fn oct_pack_float(encoded: &Cartesian2) -> f64 {
        256.0 * encoded.x + encoded.y
    }

    /// Encodes a normalized vector and returns a single float.
    pub fn oct_encode_float(vector: &Cartesian3) -> f64 {
        let mut scratch = Cartesian2::default();
        Self::oct_encode(vector, &mut scratch);
        Self::oct_pack_float(&scratch)
    }

    /// Decodes a normalized vector from a packed float.
    pub fn oct_decode_float(value: f64, result: &mut Cartesian3) -> &mut Cartesian3 {
        let temp = value / 256.0;
        let x = temp.floor();
        let y = (temp - x) * 256.0;
        Self::oct_decode(x, y, result)
    }

    /// Encodes three normalized vectors into two packed floats.
    pub fn oct_pack<'a>(
        v1: &Cartesian3,
        v2: &Cartesian3,
        v3: &Cartesian3,
        result: &'a mut Cartesian2,
    ) -> &'a mut Cartesian2 {
        let encoded1 = Self::oct_encode_float(v1);
        let encoded2 = Self::oct_encode_float(v2);

        let mut scratch = Cartesian2::default();
        Self::oct_encode(v3, &mut scratch);
        result.x = LEFT_SHIFT16 * scratch.x + encoded1;
        result.y = LEFT_SHIFT16 * scratch.y + encoded2;
        result
    }

    /// Decodes three normalized vectors from two packed floats.
    pub fn oct_unpack(packed: &Cartesian2, v1: &mut Cartesian3, v2: &mut Cartesian3, v3: &mut Cartesian3) {
        let mut temp = packed.x / LEFT_SHIFT16;
        let x = temp.floor();
        let encoded_float1 = (temp - x) * LEFT_SHIFT16;

        temp = packed.y / LEFT_SHIFT16;
        let y = temp.floor();
        let encoded_float2 = (temp - y) * LEFT_SHIFT16;

        Self::oct_decode_float(encoded_float1, v1);
        Self::oct_decode_float(encoded_float2, v2);
        Self::oct_decode(x, y, v3);
    }

    // --- texture coordinates -----------------------------------------------

    /// Packs texture coordinates into a single float (12-bit precision).
    pub fn compress_texture_coordinates(texture_coordinates: &Cartesian2) -> f64 {
        let x = (texture_coordinates.x * 4095.0) as i32 as f64;
        let y = (texture_coordinates.y * 4095.0) as i32 as f64;
        4096.0 * x + y
    }

    /// Decompresses texture coordinates from a packed float.
    pub fn decompress_texture_coordinates<'a>(compressed: f64, result: &'a mut Cartesian2) -> &'a mut Cartesian2 {
        let temp = compressed / 4096.0;
        let x_zero_to4095 = temp.floor();
        result.x = x_zero_to4095 / 4095.0;
        result.y = (compressed - x_zero_to4095 * 4096.0) / 4095.0;
        result
    }

    // --- ZigZag delta decode -----------------------------------------------

    /// Decodes delta- and ZigZag-encoded vertex buffers **in place**.
    ///
    /// Used by the quantized-mesh terrain format.
    pub fn zig_zag_delta_decode(
        u_buffer: &mut [u16],
        v_buffer: &mut [u16],
        mut height_buffer: Option<&mut [u16]>,
    ) {
        debug_assert_eq!(u_buffer.len(), v_buffer.len());
        if let Some(ref hb) = height_buffer {
            debug_assert_eq!(u_buffer.len(), hb.len());
        }

        let mut u: i32 = 0;
        let mut v: i32 = 0;
        let mut h: i32 = 0;

        for i in 0..u_buffer.len() {
            u += zig_zag_decode(u_buffer[i]);
            v += zig_zag_decode(v_buffer[i]);
            u_buffer[i] = u as u16;
            v_buffer[i] = v as u16;

            if let Some(ref mut hb) = height_buffer {
                h += zig_zag_decode(hb[i]);
                hb[i] = h as u16;
            }
        }
    }

    // --- RGB encoding (Color-dependent — stubs until Color is ported) ------

    // encodeRGB8 / decodeRGB8 require the Color type; deferred.

    /// Decodes RGB565-encoded colors into normalized `[0,1]` RGB triples.
    ///
    /// Returns a new `Vec<f32>` of length `typed_array.len() * 3`.
    ///
    /// Faithful to `AttributeCompression.decodeRGB565`: JS computes
    /// `component * normalize` in f64 (`Number`) and rounds exactly once
    /// when storing into the `Float32Array`. Computing the product in f32
    /// directly introduces a 1-ULP double-rounding difference (Phase 2
    /// finding D2).
    pub fn decode_rgb565(typed_array: &[u16]) -> Vec<f32> {
        const MASK5: u16 = (1 << 5) - 1; // 31
        const MASK6: u16 = (1 << 6) - 1; // 63
        const NORM5: f64 = 1.0 / 31.0;
        const NORM6: f64 = 1.0 / 63.0;

        let count = typed_array.len();
        let mut result = vec![0.0f32; count * 3];

        for i in 0..count {
            let value = typed_array[i];
            let red = (value >> 11) as f64;
            let green = ((value >> 5) & MASK6) as f64;
            let blue = (value & MASK5) as f64;

            let offset = 3 * i;
            result[offset] = (red * NORM5) as f32;
            result[offset + 1] = (green * NORM6) as f32;
            result[offset + 2] = (blue * NORM5) as f32;
        }
        result
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Replicates the JS trick of forcing a value through `Uint8Array` assignment
/// (clamping + truncation to `[0, 255]` integer).
fn force_uint8(value: f64) -> f64 {
    (value.round() as u8) as f64
}

/// ZigZag decodes a single unsigned 16-bit value to a signed `i32`.
fn zig_zag_decode(value: u16) -> i32 {
    let v = value as i32;
    (v >> 1) ^ -(v & 1)
}
