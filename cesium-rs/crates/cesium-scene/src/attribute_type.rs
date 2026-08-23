//! Ported from `packages/engine/Source/Scene/AttributeType.js`.

/// The type of a vertex attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum AttributeType {
    /// 32-bit float.
    Float = 0,
    /// Two 32-bit floats.
    FloatVec2 = 1,
    /// Three 32-bit floats.
    FloatVec3 = 2,
    /// Four 32-bit floats.
    FloatVec4 = 3,
    /// 2x2 float matrix.
    FloatMat2 = 4,
    /// 3x3 float matrix.
    FloatMat3 = 5,
    /// 4x4 float matrix.
    FloatMat4 = 6,
}
