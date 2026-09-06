//! Ported from `packages/engine/Source/Scene/Model/UniformType.js`.

/// An enum of the basic GLSL uniform types.
///
/// These can be used with `CustomShader` to declare user-defined uniforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UniformType {
    /// A single floating point value.
    Float,
    /// A vector of 2 floating point values.
    Vec2,
    /// A vector of 3 floating point values.
    Vec3,
    /// A vector of 4 floating point values.
    Vec4,
    /// A single integer value.
    Int,
    /// A vector of 2 integer values.
    IntVec2,
    /// A vector of 3 integer values.
    IntVec3,
    /// A vector of 4 integer values.
    IntVec4,
    /// A single boolean value.
    Bool,
    /// A vector of 2 boolean values.
    BoolVec2,
    /// A vector of 3 boolean values.
    BoolVec3,
    /// A vector of 4 boolean values.
    BoolVec4,
    /// A 2x2 matrix of floating point values.
    Mat2,
    /// A 3x3 matrix of floating point values.
    Mat3,
    /// A 4x4 matrix of floating point values.
    Mat4,
    /// A 2D sampled texture.
    Sampler2D,
    /// A cube-map sampled texture.
    SamplerCube,
}

impl UniformType {
    /// Returns the GLSL type name string (e.g. `"float"`, `"vec3"`, `"mat4"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Float => "float",
            Self::Vec2 => "vec2",
            Self::Vec3 => "vec3",
            Self::Vec4 => "vec4",
            Self::Int => "int",
            Self::IntVec2 => "ivec2",
            Self::IntVec3 => "ivec3",
            Self::IntVec4 => "ivec4",
            Self::Bool => "bool",
            Self::BoolVec2 => "bvec2",
            Self::BoolVec3 => "bvec3",
            Self::BoolVec4 => "bvec4",
            Self::Mat2 => "mat2",
            Self::Mat3 => "mat3",
            Self::Mat4 => "mat4",
            Self::Sampler2D => "sampler2D",
            Self::SamplerCube => "samplerCube",
        }
    }

    /// Parses from a GLSL type name string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "float" => Some(Self::Float),
            "vec2" => Some(Self::Vec2),
            "vec3" => Some(Self::Vec3),
            "vec4" => Some(Self::Vec4),
            "int" => Some(Self::Int),
            "ivec2" => Some(Self::IntVec2),
            "ivec3" => Some(Self::IntVec3),
            "ivec4" => Some(Self::IntVec4),
            "bool" => Some(Self::Bool),
            "bvec2" => Some(Self::BoolVec2),
            "bvec3" => Some(Self::BoolVec3),
            "bvec4" => Some(Self::BoolVec4),
            "mat2" => Some(Self::Mat2),
            "mat3" => Some(Self::Mat3),
            "mat4" => Some(Self::Mat4),
            "sampler2D" => Some(Self::Sampler2D),
            "samplerCube" => Some(Self::SamplerCube),
            _ => None,
        }
    }

    /// Returns `true` if this is a matrix type.
    pub fn is_matrix_type(&self) -> bool {
        matches!(self, Self::Mat2 | Self::Mat3 | Self::Mat4)
    }

    /// Returns `true` if this is a vector type (vec2/3/4, ivec2/3/4, bvec2/3/4).
    pub fn is_vector_type(&self) -> bool {
        matches!(
            self,
            Self::Vec2
                | Self::Vec3
                | Self::Vec4
                | Self::IntVec2
                | Self::IntVec3
                | Self::IntVec4
                | Self::BoolVec2
                | Self::BoolVec3
                | Self::BoolVec4
        )
    }

    /// Returns `true` if this is a sampler type.
    pub fn is_sampler_type(&self) -> bool {
        matches!(self, Self::Sampler2D | Self::SamplerCube)
    }
}
