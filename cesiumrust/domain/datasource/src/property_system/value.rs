//! Type-erased property values and packable value types.
//!
//! Maps to the CesiumJS `Packable` interface implemented by `Cartesian2`,
//! `Cartesian3`, `Quaternion`, `Color` and the internal `PackableNumber`
//! (see `DataSources/SampledProperty.js`), plus `DataSources/ReferenceFrame.js`.

use glam::{DQuat, DVec2, DVec3};
use serde_json::Value as JsonValue;

/// The reference frame in which a position is defined.
///
/// Maps to CesiumJS `DataSources/ReferenceFrame.js`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ReferenceFrame {
    /// The fixed frame (e.g. ECEF / `FIXED`).
    #[default]
    Fixed,
    /// The inertial frame (e.g. ICRF / `INERTIAL`).
    Inertial,
}

/// A type-erased property value.
///
/// CesiumJS properties may hold any value; this enum covers the set of value
/// types used across the DataSources layer.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    /// No value (CesiumJS `undefined`).
    Undefined,
    /// A number.
    Number(f64),
    /// A boolean.
    Boolean(bool),
    /// A string.
    Text(String),
    /// A 2D Cartesian vector.
    Cartesian2(DVec2),
    /// A 3D Cartesian vector.
    Cartesian3(DVec3),
    /// A quaternion (rotation).
    Quaternion(DQuat),
    /// An RGBA color with components in `[0, 1]`.
    Color([f64; 4]),
    /// A generic packed array of `f64`.
    Array(Vec<f64>),
    /// An arbitrary JSON value.
    Json(JsonValue),
}

impl PropertyValue {
    /// Returns `true` if this value is `Undefined`.
    pub fn is_undefined(&self) -> bool {
        matches!(self, PropertyValue::Undefined)
    }

    /// Returns up to four packed `f64` components of this value.
    fn packed_components(&self) -> [f64; 4] {
        match self {
            PropertyValue::Number(v) => [*v, 0.0, 0.0, 0.0],
            PropertyValue::Cartesian2(v) => [v.x, v.y, 0.0, 0.0],
            PropertyValue::Cartesian3(v) => [v.x, v.y, v.z, 0.0],
            PropertyValue::Quaternion(q) => [q.x, q.y, q.z, q.w],
            PropertyValue::Color(c) => [c[0], c[1], c[2], c[3]],
            _ => [0.0; 4],
        }
    }
}

/// A packable value type usable with `SampledProperty`.
///
/// Maps to the CesiumJS `Packable` interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackableType {
    /// A single number (`PackableNumber`, packedLength 1).
    Number,
    /// `Cartesian2` (packedLength 2).
    Cartesian2,
    /// `Cartesian3` (packedLength 3).
    Cartesian3,
    /// `Quaternion` (packedLength 4, packedInterpolationLength 3).
    Quaternion,
    /// `Color` (packedLength 4).
    Color,
}

impl PackableType {
    /// The number of `f64` elements used to store the value.
    /// Maps to `Packable.packedLength`.
    pub fn packed_length(self) -> usize {
        match self {
            PackableType::Number => 1,
            PackableType::Cartesian2 => 2,
            PackableType::Cartesian3 => 3,
            PackableType::Quaternion => 4,
            PackableType::Color => 4,
        }
    }

    /// The number of elements used to store the value in a form suitable for
    /// interpolation. Maps to `Packable.packedInterpolationLength`.
    pub fn packed_interpolation_length(self) -> usize {
        match self {
            PackableType::Quaternion => 3,
            other => other.packed_length(),
        }
    }

    /// Appends the packed representation of `value` onto `out`.
    /// Maps to `Packable.pack(value, array, startingIndex)` (append form).
    pub fn pack(&self, value: &PropertyValue, out: &mut Vec<f64>) {
        let comps = value.packed_components();
        let len = self.packed_length();
        out.extend_from_slice(&comps[..len]);
    }

    /// Writes the packed representation of `value` into `array` starting at
    /// `starting_index`.
    pub fn pack_at(&self, value: &PropertyValue, array: &mut [f64], starting_index: usize) {
        let comps = value.packed_components();
        let len = self.packed_length();
        array[starting_index..starting_index + len].copy_from_slice(&comps[..len]);
    }

    /// Reads a value from `array` starting at `starting_index`.
    /// Maps to `Packable.unpack(array, startingIndex, result)`.
    pub fn unpack(&self, array: &[f64], starting_index: usize) -> PropertyValue {
        let s = starting_index;
        match self {
            PackableType::Number => PropertyValue::Number(array[s]),
            PackableType::Cartesian2 => {
                PropertyValue::Cartesian2(DVec2::new(array[s], array[s + 1]))
            }
            PackableType::Cartesian3 => {
                PropertyValue::Cartesian3(DVec3::new(array[s], array[s + 1], array[s + 2]))
            }
            PackableType::Quaternion => PropertyValue::Quaternion(DQuat::from_xyzw(
                array[s],
                array[s + 1],
                array[s + 2],
                array[s + 3],
            )),
            PackableType::Color => {
                PropertyValue::Color([array[s], array[s + 1], array[s + 2], array[s + 3]])
            }
        }
    }

    /// Whether this type defines `convertPackedArrayForInterpolation`
    /// (only `Quaternion` does).
    pub fn uses_interpolation_conversion(&self) -> bool {
        matches!(self, PackableType::Quaternion)
    }

    /// Converts a packed array into a form suitable for interpolation.
    ///
    /// Maps to `Quaternion.convertPackedArrayForInterpolation`. Only meaningful
    /// for `Quaternion`; converts each quaternion in the inclusive range
    /// `[first_index, last_index]` into an axis-angle vector relative to the
    /// last quaternion in the range.
    pub fn convert_packed_array_for_interpolation(
        &self,
        packed_array: &[f64],
        first_index: usize,
        last_index: usize,
        result: &mut [f64],
    ) {
        if !self.uses_interpolation_conversion() {
            return;
        }
        let last = unpack_quaternion(packed_array, last_index * 4);
        let last_conjugate = last.conjugate();

        let len = last_index - first_index + 1;
        for i in 0..len {
            let offset = i * 3;
            let mut q = unpack_quaternion(packed_array, (first_index + i) * 4);
            q *= last_conjugate;
            if q.w < 0.0 {
                q = -q;
            }
            let axis = compute_axis(q);
            let angle = compute_angle(q);
            result[offset] = axis.x * angle;
            result[offset + 1] = axis.y * angle;
            result[offset + 2] = axis.z * angle;
        }
    }

    /// Retrieves an instance from an array converted with
    /// `convert_packed_array_for_interpolation`.
    ///
    /// Maps to `Quaternion.unpackInterpolationResult`. Only meaningful for
    /// `Quaternion`.
    pub fn unpack_interpolation_result(
        &self,
        array: &[f64],
        source_array: &[f64],
        _first_index: usize,
        last_index: usize,
    ) -> PropertyValue {
        let rotation = DVec3::new(array[0], array[1], array[2]);
        let magnitude = rotation.length();
        let q0 = unpack_quaternion(source_array, last_index * 4);

        let temp = if magnitude == 0.0 {
            DQuat::IDENTITY
        } else {
            from_axis_angle(rotation, magnitude)
        };
        PropertyValue::Quaternion(temp * q0)
    }
}

/// Unpacks a quaternion (4 components) from `array` at `starting_index`.
fn unpack_quaternion(array: &[f64], starting_index: usize) -> DQuat {
    DQuat::from_xyzw(
        array[starting_index],
        array[starting_index + 1],
        array[starting_index + 2],
        array[starting_index + 3],
    )
}

const EPSILON6: f64 = 1e-6;

/// Computes the normalized rotation axis of a quaternion.
/// Maps to `Quaternion.computeAxis`.
fn compute_axis(q: DQuat) -> DVec3 {
    let w = q.w;
    if (w - 1.0).abs() < EPSILON6 || (w + 1.0).abs() < EPSILON6 {
        return DVec3::new(1.0, 0.0, 0.0);
    }
    let scalar = 1.0 / (1.0 - w * w).sqrt();
    DVec3::new(q.x * scalar, q.y * scalar, q.z * scalar)
}

/// Computes the rotation angle of a quaternion.
/// Maps to `Quaternion.computeAngle`.
fn compute_angle(q: DQuat) -> f64 {
    if (q.w - 1.0).abs() < EPSILON6 {
        return 0.0;
    }
    2.0 * q.w.acos()
}

/// Builds a quaternion from an axis (normalized internally) and an angle.
/// Maps to `Quaternion.fromAxisAngle`.
fn from_axis_angle(axis: DVec3, angle: f64) -> DQuat {
    let half_angle = angle / 2.0;
    let s = half_angle.sin();
    let axis = if axis.length_squared() > 0.0 {
        axis.normalize()
    } else {
        DVec3::X
    };
    DQuat::from_xyzw(axis.x * s, axis.y * s, axis.z * s, half_angle.cos())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_2;

    #[test]
    fn test_packed_lengths() {
        assert_eq!(PackableType::Number.packed_length(), 1);
        assert_eq!(PackableType::Cartesian2.packed_length(), 2);
        assert_eq!(PackableType::Cartesian3.packed_length(), 3);
        assert_eq!(PackableType::Quaternion.packed_length(), 4);
        assert_eq!(PackableType::Color.packed_length(), 4);
    }

    #[test]
    fn test_packed_interpolation_lengths() {
        assert_eq!(PackableType::Number.packed_interpolation_length(), 1);
        assert_eq!(PackableType::Cartesian3.packed_interpolation_length(), 3);
        assert_eq!(PackableType::Quaternion.packed_interpolation_length(), 3);
        assert_eq!(PackableType::Color.packed_interpolation_length(), 4);
    }

    #[test]
    fn test_pack_unpack_roundtrip() {
        let cases = [
            (PackableType::Number, PropertyValue::Number(42.5)),
            (
                PackableType::Cartesian2,
                PropertyValue::Cartesian2(DVec2::new(1.0, 2.0)),
            ),
            (
                PackableType::Cartesian3,
                PropertyValue::Cartesian3(DVec3::new(1.0, 2.0, 3.0)),
            ),
            (
                PackableType::Quaternion,
                PropertyValue::Quaternion(DQuat::from_xyzw(0.1, 0.2, 0.3, 0.9)),
            ),
            (
                PackableType::Color,
                PropertyValue::Color([0.25, 0.5, 0.75, 1.0]),
            ),
        ];
        for (ty, value) in &cases {
            let mut buf = Vec::new();
            ty.pack(value, &mut buf);
            assert_eq!(buf.len(), ty.packed_length());
            let unpacked = ty.unpack(&buf, 0);
            assert_eq!(&unpacked, value);
        }
    }

    #[test]
    fn test_pack_at_offset() {
        let mut buf = vec![0.0; 5];
        PackableType::Cartesian3.pack_at(&PropertyValue::Cartesian3(DVec3::new(7.0, 8.0, 9.0)), &mut buf, 2);
        assert_eq!(buf, vec![0.0, 0.0, 7.0, 8.0, 9.0]);
    }

    #[test]
    fn test_uses_interpolation_conversion() {
        assert!(PackableType::Quaternion.uses_interpolation_conversion());
        assert!(!PackableType::Number.uses_interpolation_conversion());
        assert!(!PackableType::Cartesian3.uses_interpolation_conversion());
        assert!(!PackableType::Color.uses_interpolation_conversion());
    }

    #[test]
    fn test_quaternion_interpolation_conversion_identity() {
        // Two identical quaternions: relative rotation is identity -> axis-angle zero.
        let q = DQuat::from_rotation_z(FRAC_PI_2);
        let mut packed = Vec::new();
        PackableType::Quaternion.pack(&PropertyValue::Quaternion(q), &mut packed);
        PackableType::Quaternion.pack(&PropertyValue::Quaternion(q), &mut packed);

        let mut result = vec![0.0; 6];
        PackableType::Quaternion.convert_packed_array_for_interpolation(&packed, 0, 1, &mut result);
        // Both relative to the last (itself): identity -> zero vectors.
        assert!((result[0]).abs() < 1e-9);
        assert!((result[1]).abs() < 1e-9);
        assert!((result[2]).abs() < 1e-9);
        assert!((result[3]).abs() < 1e-9);
        assert!((result[4]).abs() < 1e-9);
        assert!((result[5]).abs() < 1e-9);
    }

    #[test]
    fn test_quaternion_interpolation_roundtrip() {
        // q0 = identity, q1 = 90deg about Z. Relative to q1:
        // q0 * conj(q1) = -90deg about Z.
        let q0 = DQuat::IDENTITY;
        let q1 = DQuat::from_rotation_z(FRAC_PI_2);
        let mut packed = Vec::new();
        PackableType::Quaternion.pack(&PropertyValue::Quaternion(q0), &mut packed);
        PackableType::Quaternion.pack(&PropertyValue::Quaternion(q1), &mut packed);

        let mut result = vec![0.0; 6];
        PackableType::Quaternion.convert_packed_array_for_interpolation(&packed, 0, 1, &mut result);

        // Reconstruct q0 from its axis-angle representation relative to q1.
        let recovered = PackableType::Quaternion.unpack_interpolation_result(&result, &packed, 0, 1);
        if let PropertyValue::Quaternion(rq) = recovered {
            // Quaternions are equal up to sign.
            let dot = rq.dot(q0);
            assert!((dot.abs() - 1.0).abs() < 1e-9, "dot = {dot}");
        } else {
            panic!("expected quaternion");
        }
    }

    #[test]
    fn test_reference_frame_default() {
        assert_eq!(ReferenceFrame::default(), ReferenceFrame::Fixed);
    }

    #[test]
    fn test_property_value_is_undefined() {
        assert!(PropertyValue::Undefined.is_undefined());
        assert!(!PropertyValue::Number(1.0).is_undefined());
    }
}
