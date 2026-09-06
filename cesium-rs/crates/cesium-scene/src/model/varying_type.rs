//! Ported from `packages/engine/Source/Scene/Model/VaryingType.js`.

/// An enum for the GLSL varying types.
///
/// These can be used for declaring varyings in `CustomShader`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VaryingType {
    /// A single floating point value.
    Float,
    /// A vector of 2 floating point values.
    Vec2,
    /// A vector of 3 floating point values.
    Vec3,
    /// A vector of 4 floating point values.
    Vec4,
    /// A 2x2 matrix of floating point values.
    Mat2,
    /// A 3x3 matrix of floating point values.
    Mat3,
    /// A 4x4 matrix of floating point values.
    Mat4,
}

impl VaryingType {
    /// Returns the GLSL type name string (e.g. `"float"`, `"vec3"`, `"mat4"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Float => "float",
            Self::Vec2 => "vec2",
            Self::Vec3 => "vec3",
            Self::Vec4 => "vec4",
            Self::Mat2 => "mat2",
            Self::Mat3 => "mat3",
            Self::Mat4 => "mat4",
        }
    }

    /// Parses from a GLSL type name string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "float" => Some(Self::Float),
            "vec2" => Some(Self::Vec2),
            "vec3" => Some(Self::Vec3),
            "vec4" => Some(Self::Vec4),
            "mat2" => Some(Self::Mat2),
            "mat3" => Some(Self::Mat3),
            "mat4" => Some(Self::Mat4),
            _ => None,
        }
    }

    /// Returns `true` if this is a matrix type.
    pub fn is_matrix_type(&self) -> bool {
        matches!(self, Self::Mat2 | Self::Mat3 | Self::Mat4)
    }

    /// Returns `true` if this is a vector type.
    pub fn is_vector_type(&self) -> bool {
        matches!(self, Self::Vec2 | Self::Vec3 | Self::Vec4)
    }
}
