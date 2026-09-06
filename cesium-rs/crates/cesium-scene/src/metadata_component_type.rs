//! Ported from `packages/engine/Source/Scene/MetadataComponentType.js`.
//!
//! An enum of metadata component types defined by the 3D Tiles metadata spec.

use cesium_core::component_datatype::ComponentDatatype;

/// Category of a scalar component type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScalarCategory {
    /// Signed integer.
    Integer,
    /// Unsigned integer.
    UnsignedInteger,
    /// Floating-point.
    Float,
}

/// An enum of metadata component types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetadataComponentType {
    /// An 8-bit signed integer.
    Int8,
    /// An 8-bit unsigned integer.
    Uint8,
    /// A 16-bit signed integer.
    Int16,
    /// A 16-bit unsigned integer.
    Uint16,
    /// A 32-bit signed integer.
    Int32,
    /// A 32-bit unsigned integer.
    Uint32,
    /// A 64-bit signed integer.
    Int64,
    /// A 64-bit unsigned integer.
    Uint64,
    /// A 32-bit floating-point number.
    Float32,
    /// A 64-bit floating-point number.
    Float64,
}

impl MetadataComponentType {
    /// Returns the JSON string representation (e.g. `"INT8"`, `"FLOAT32"`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Int8 => "INT8",
            Self::Uint8 => "UINT8",
            Self::Int16 => "INT16",
            Self::Uint16 => "UINT16",
            Self::Int32 => "INT32",
            Self::Uint32 => "UINT32",
            Self::Int64 => "INT64",
            Self::Uint64 => "UINT64",
            Self::Float32 => "FLOAT32",
            Self::Float64 => "FLOAT64",
        }
    }

    /// Parses from a JSON string representation.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "INT8" => Some(Self::Int8),
            "UINT8" => Some(Self::Uint8),
            "INT16" => Some(Self::Int16),
            "UINT16" => Some(Self::Uint16),
            "INT32" => Some(Self::Int32),
            "UINT32" => Some(Self::Uint32),
            "INT64" => Some(Self::Int64),
            "UINT64" => Some(Self::Uint64),
            "FLOAT32" => Some(Self::Float32),
            "FLOAT64" => Some(Self::Float64),
            _ => None,
        }
    }

    /// Returns `true` if the type can be used as a Cartesian vector component.
    ///
    /// All numeric types except `Int64` and `Uint64` are vector-compatible.
    pub fn is_vector_compatible(&self) -> bool {
        !matches!(self, Self::Int64 | Self::Uint64)
    }

    /// Returns the size in bytes.
    pub fn size_in_bytes(&self) -> usize {
        match self {
            Self::Int8 | Self::Uint8 => 1,
            Self::Int16 | Self::Uint16 => 2,
            Self::Int32 | Self::Uint32 | Self::Float32 => 4,
            Self::Int64 | Self::Uint64 | Self::Float64 => 8,
        }
    }

    /// Returns the minimum value as `f64`.
    ///
    /// For `Int64`/`Uint64` the true range exceeds `f64` precision; this
    /// returns the closest `f64` approximation.
    pub fn minimum_value(&self) -> f64 {
        match self {
            Self::Int8 => -128.0,
            Self::Uint8 => 0.0,
            Self::Int16 => -32768.0,
            Self::Uint16 => 0.0,
            Self::Int32 => -2_147_483_648.0,
            Self::Uint32 => 0.0,
            Self::Int64 => -9_223_372_036_854_775_808.0,
            Self::Uint64 => 0.0,
            Self::Float32 => -3.4028234663852886e38,
            Self::Float64 => -f64::MAX,
        }
    }

    /// Returns the maximum value as `f64`.
    pub fn maximum_value(&self) -> f64 {
        match self {
            Self::Int8 => 127.0,
            Self::Uint8 => 255.0,
            Self::Int16 => 32767.0,
            Self::Uint16 => 65535.0,
            Self::Int32 => 2_147_483_647.0,
            Self::Uint32 => 4_294_967_295.0,
            Self::Int64 => 9_223_372_036_854_775_807.0,
            Self::Uint64 => 18_446_744_073_709_551_615.0,
            Self::Float32 => 3.4028234663852886e38,
            Self::Float64 => f64::MAX,
        }
    }

    /// Returns `true` for integer types (signed or unsigned).
    pub fn is_integer_type(&self) -> bool {
        self.category() != ScalarCategory::Float
    }

    /// Returns `true` for unsigned integer types.
    pub fn is_unsigned_integer_type(&self) -> bool {
        self.category() == ScalarCategory::UnsignedInteger
    }

    /// Returns the scalar category of this type.
    pub fn category(&self) -> ScalarCategory {
        match self {
            Self::Int8 | Self::Int16 | Self::Int32 | Self::Int64 => ScalarCategory::Integer,
            Self::Uint8 | Self::Uint16 | Self::Uint32 | Self::Uint64 => {
                ScalarCategory::UnsignedInteger
            }
            Self::Float32 | Self::Float64 => ScalarCategory::Float,
        }
    }

    /// Returns the GPU component type (64-bit types are downcast).
    pub fn gpu_component_type(&self) -> Self {
        match self {
            Self::Int64 => Self::Int32,
            Self::Uint64 => Self::Uint32,
            Self::Float64 => Self::Float32,
            other => *other,
        }
    }

    /// Normalizes an integer value to `[-1.0, 1.0]` (signed) or `[0.0, 1.0]`
    /// (unsigned).
    pub fn normalize(value: f64, component_type: &Self) -> f64 {
        let max = component_type.maximum_value();
        (value / max).max(-1.0)
    }

    /// Unnormalizes a value from `[-1.0, 1.0]` or `[0.0, 1.0]` back to the
    /// integer range.
    pub fn unnormalize(value: f64, component_type: &Self) -> f64 {
        let max = component_type.maximum_value();
        let sign = if value < 0.0 { -1.0 } else { 1.0 };
        let result = sign * (value.abs() * max).round();
        let min = if component_type.is_unsigned_integer_type() {
            0.0
        } else {
            -max
        };
        result.clamp(min, max)
    }

    /// Converts from a [`ComponentDatatype`].
    pub fn from_component_datatype(datatype: ComponentDatatype) -> Option<Self> {
        match datatype {
            ComponentDatatype::Byte => Some(Self::Int8),
            ComponentDatatype::UnsignedByte => Some(Self::Uint8),
            ComponentDatatype::Short => Some(Self::Int16),
            ComponentDatatype::UnsignedShort => Some(Self::Uint16),
            ComponentDatatype::Int => Some(Self::Int32),
            ComponentDatatype::UnsignedInt => Some(Self::Uint32),
            ComponentDatatype::Float => Some(Self::Float32),
            ComponentDatatype::Double => Some(Self::Float64),
        }
    }

    /// Converts to a [`ComponentDatatype`].
    ///
    /// Returns `None` for `Int64`/`Uint64` which have no WebGL equivalent.
    pub fn to_component_datatype(&self) -> Option<ComponentDatatype> {
        match self {
            Self::Int8 => Some(ComponentDatatype::Byte),
            Self::Uint8 => Some(ComponentDatatype::UnsignedByte),
            Self::Int16 => Some(ComponentDatatype::Short),
            Self::Uint16 => Some(ComponentDatatype::UnsignedShort),
            Self::Int32 => Some(ComponentDatatype::Int),
            Self::Uint32 => Some(ComponentDatatype::UnsignedInt),
            Self::Float32 => Some(ComponentDatatype::Float),
            Self::Float64 => Some(ComponentDatatype::Double),
            Self::Int64 | Self::Uint64 => None,
        }
    }
}
