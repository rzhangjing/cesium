//! AttributeType enum for 3D Tiles metadata and custom shaders.
//!
//! Maps to CesiumJS `Scene/AttributeType.js`

/// An enum describing the attribute types for metadata and custom shaders.
///
/// Maps to CesiumJS `Scene/AttributeType.js`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttributeType {
    /// A single scalar value.
    Scalar,
    /// A 2D vector.
    Vec2,
    /// A 3D vector.
    Vec3,
    /// A 4D vector.
    Vec4,
    /// A 2x2 matrix.
    Mat2,
    /// A 3x3 matrix.
    Mat3,
    /// A 4x4 matrix.
    Mat4,
}

impl AttributeType {
    /// Gets the GLSL type string for this attribute type.
    ///
    /// Maps to CesiumJS `AttributeType.getGlslType`.
    pub fn get_glsl_type(&self) -> &'static str {
        match self {
            AttributeType::Scalar => "float",
            AttributeType::Vec2 => "vec2",
            AttributeType::Vec3 => "vec3",
            AttributeType::Vec4 => "vec4",
            AttributeType::Mat2 => "mat2",
            AttributeType::Mat3 => "mat3",
            AttributeType::Mat4 => "mat4",
        }
    }

    /// Gets the number of components for this attribute type.
    ///
    /// Maps to CesiumJS `AttributeType.getNumberOfComponents`.
    pub fn get_number_of_components(&self) -> usize {
        match self {
            AttributeType::Scalar => 1,
            AttributeType::Vec2 => 2,
            AttributeType::Vec3 => 3,
            AttributeType::Vec4 => 4,
            AttributeType::Mat2 => 4,
            AttributeType::Mat3 => 9,
            AttributeType::Mat4 => 16,
        }
    }

    /// Gets the number of attribute locations needed for this type.
    /// Matrices require multiple locations (one per row).
    ///
    /// Maps to CesiumJS `AttributeType.getAttributeLocationCount`.
    pub fn get_attribute_location_count(&self) -> usize {
        match self {
            AttributeType::Scalar => 1,
            AttributeType::Vec2 => 1,
            AttributeType::Vec3 => 1,
            AttributeType::Vec4 => 1,
            AttributeType::Mat2 => 2,
            AttributeType::Mat3 => 3,
            AttributeType::Mat4 => 4,
        }
    }

    /// Gets the math type name for this attribute type.
    ///
    /// Maps to CesiumJS `AttributeType.getMathType`.
    pub fn get_math_type_name(&self) -> &'static str {
        match self {
            AttributeType::Scalar => "Number",
            AttributeType::Vec2 => "Cartesian2",
            AttributeType::Vec3 => "Cartesian3",
            AttributeType::Vec4 => "Cartesian4",
            AttributeType::Mat2 => "Matrix2",
            AttributeType::Mat3 => "Matrix3",
            AttributeType::Mat4 => "Matrix4",
        }
    }

    /// Parses an attribute type from its string representation.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "SCALAR" => Some(AttributeType::Scalar),
            "VEC2" => Some(AttributeType::Vec2),
            "VEC3" => Some(AttributeType::Vec3),
            "VEC4" => Some(AttributeType::Vec4),
            "MAT2" => Some(AttributeType::Mat2),
            "MAT3" => Some(AttributeType::Mat3),
            "MAT4" => Some(AttributeType::Mat4),
            _ => None,
        }
    }
}
