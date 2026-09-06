//! Ported from `packages/engine/Source/Scene/MetadataType.js`.
//!
//! An enum of metadata value types defined by the 3D Tiles metadata spec.

/// An enum of metadata types.
///
/// These are containers containing one or more components of type
/// [`MetadataComponentType`](crate::metadata_component_type::MetadataComponentType).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetadataType {
    /// A single component.
    Scalar,
    /// A vector with two components.
    Vec2,
    /// A vector with three components.
    Vec3,
    /// A vector with four components.
    Vec4,
    /// A 2×2 matrix, stored in column-major format.
    Mat2,
    /// A 3×3 matrix, stored in column-major format.
    Mat3,
    /// A 4×4 matrix, stored in column-major format.
    Mat4,
    /// A boolean (true/false) value.
    Boolean,
    /// A UTF-8 encoded string value.
    String,
    /// An enumerated value.
    Enum,
}

impl MetadataType {
    /// Returns the JSON string representation (e.g. `"SCALAR"`, `"VEC3"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Scalar => "SCALAR",
            Self::Vec2 => "VEC2",
            Self::Vec3 => "VEC3",
            Self::Vec4 => "VEC4",
            Self::Mat2 => "MAT2",
            Self::Mat3 => "MAT3",
            Self::Mat4 => "MAT4",
            Self::Boolean => "BOOLEAN",
            Self::String => "STRING",
            Self::Enum => "ENUM",
        }
    }

    /// Parses a type from its JSON string representation.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "SCALAR" => Some(Self::Scalar),
            "VEC2" => Some(Self::Vec2),
            "VEC3" => Some(Self::Vec3),
            "VEC4" => Some(Self::Vec4),
            "MAT2" => Some(Self::Mat2),
            "MAT3" => Some(Self::Mat3),
            "MAT4" => Some(Self::Mat4),
            "BOOLEAN" => Some(Self::Boolean),
            "STRING" => Some(Self::String),
            "ENUM" => Some(Self::Enum),
            _ => None,
        }
    }

    /// Returns `true` if the type is `Vec2`, `Vec3`, or `Vec4`.
    pub fn is_vector_type(&self) -> bool {
        matches!(self, Self::Vec2 | Self::Vec3 | Self::Vec4)
    }

    /// Returns `true` if the type is `Mat2`, `Mat3`, or `Mat4`.
    pub fn is_matrix_type(&self) -> bool {
        matches!(self, Self::Mat2 | Self::Mat3 | Self::Mat4)
    }

    /// Returns the number of components.
    ///
    /// For vectors, returns N. For matrices, returns N*N. All other types
    /// return 1.
    pub fn component_count(&self) -> usize {
        match self {
            Self::Scalar | Self::String | Self::Enum | Self::Boolean => 1,
            Self::Vec2 => 2,
            Self::Vec3 => 3,
            Self::Vec4 => 4,
            Self::Mat2 => 4,
            Self::Mat3 => 9,
            Self::Mat4 => 16,
        }
    }
}
