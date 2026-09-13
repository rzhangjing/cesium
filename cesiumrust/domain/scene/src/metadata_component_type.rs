//! MetadataComponentType enum for 3D Tiles metadata.
//!
//! Maps to CesiumJS `Scene/MetadataComponentType.js`

/// Category of a scalar metadata component type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarCategory {
    /// Signed integer types (INT8, INT16, INT32, INT64).
    Integer,
    /// Unsigned integer types (UINT8, UINT16, UINT32, UINT64).
    UnsignedInteger,
    /// Floating point types (FLOAT32, FLOAT64).
    Float,
}

/// An enum of metadata component types for 3D Tiles metadata.
///
/// Maps to CesiumJS `Scene/MetadataComponentType.js`
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
    /// A 32-bit (single precision) floating point number.
    Float32,
    /// A 64-bit (double precision) floating point number.
    Float64,
}

impl MetadataComponentType {
    /// Gets the minimum value for the numeric type.
    ///
    /// Maps to CesiumJS `MetadataComponentType.getMinimum`.
    pub fn get_minimum(&self) -> f64 {
        match self {
            Self::Int8 => i8::MIN as f64,
            Self::Uint8 => 0.0,
            Self::Int16 => i16::MIN as f64,
            Self::Uint16 => 0.0,
            Self::Int32 => i32::MIN as f64,
            Self::Uint32 => 0.0,
            Self::Int64 => i64::MIN as f64,
            Self::Uint64 => 0.0,
            Self::Float32 => -f32::MAX as f64,
            Self::Float64 => -f64::MAX,
        }
    }

    /// Gets the maximum value for the numeric type.
    ///
    /// Maps to CesiumJS `MetadataComponentType.getMaximum`.
    pub fn get_maximum(&self) -> f64 {
        match self {
            Self::Int8 => i8::MAX as f64,
            Self::Uint8 => u8::MAX as f64,
            Self::Int16 => i16::MAX as f64,
            Self::Uint16 => u16::MAX as f64,
            Self::Int32 => i32::MAX as f64,
            Self::Uint32 => u32::MAX as f64,
            Self::Int64 => i64::MAX as f64,
            Self::Uint64 => u64::MAX as f64,
            Self::Float32 => f32::MAX as f64,
            Self::Float64 => f64::MAX,
        }
    }

    /// Returns whether the type is an integer type.
    ///
    /// Maps to CesiumJS `MetadataComponentType.isIntegerType`.
    pub fn is_integer_type(&self) -> bool {
        self.category() != ScalarCategory::Float
    }

    /// Returns whether the type is an unsigned integer type.
    ///
    /// Maps to CesiumJS `MetadataComponentType.isUnsignedIntegerType`.
    pub fn is_unsigned_integer_type(&self) -> bool {
        self.category() == ScalarCategory::UnsignedInteger
    }

    /// Gets the category of the numeric type.
    ///
    /// Maps to CesiumJS `MetadataComponentType.category`.
    pub fn category(&self) -> ScalarCategory {
        match self {
            Self::Int8 | Self::Int16 | Self::Int32 | Self::Int64 => ScalarCategory::Integer,
            Self::Uint8 | Self::Uint16 | Self::Uint32 | Self::Uint64 => {
                ScalarCategory::UnsignedInteger
            }
            Self::Float32 | Self::Float64 => ScalarCategory::Float,
        }
    }

    /// Gets the size in bytes for the numeric type.
    ///
    /// Maps to CesiumJS `MetadataComponentType.getSizeInBytes`.
    pub fn get_size_in_bytes(&self) -> usize {
        match self {
            Self::Int8 | Self::Uint8 => 1,
            Self::Int16 | Self::Uint16 => 2,
            Self::Int32 | Self::Uint32 | Self::Float32 => 4,
            Self::Int64 | Self::Uint64 | Self::Float64 => 8,
        }
    }

    /// Normalizes an integer value to the range [-1.0, 1.0] (signed) or [0.0, 1.0] (unsigned).
    ///
    /// Maps to CesiumJS `MetadataComponentType.normalize`.
    pub fn normalize(&self, value: f64) -> f64 {
        let max = self.get_maximum();
        (value / max).max(-1.0)
    }

    /// Unnormalizes a value in [-1.0, 1.0] (signed) or [0.0, 1.0] (unsigned) back to integer.
    ///
    /// Maps to CesiumJS `MetadataComponentType.unnormalize`.
    pub fn unnormalize(&self, value: f64) -> f64 {
        let max = self.get_maximum();
        let min = if self.is_unsigned_integer_type() {
            0.0
        } else {
            -max
        };

        let result = value.signum() * (value.abs() * max).round();

        if result > max {
            return max;
        }
        if result < min {
            return min;
        }
        result
    }

    /// Converts from a ComponentDatatype value to MetadataComponentType.
    ///
    /// Maps to CesiumJS `MetadataComponentType.fromComponentDatatype`.
    pub fn from_component_datatype(datatype: u32) -> Option<Self> {
        // ComponentDatatype values: BYTE=5120, UNSIGNED_BYTE=5121, SHORT=5122,
        // UNSIGNED_SHORT=5123, INT=5124, UNSIGNED_INT=5125, FLOAT=5126, DOUBLE=5130
        match datatype {
            5120 => Some(Self::Int8),
            5121 => Some(Self::Uint8),
            5122 => Some(Self::Int16),
            5123 => Some(Self::Uint16),
            5124 => Some(Self::Int32),
            5125 => Some(Self::Uint32),
            5126 => Some(Self::Float32),
            5130 => Some(Self::Float64),
            _ => None,
        }
    }

    /// Converts to a ComponentDatatype value.
    /// Returns None for INT64/UINT64 (no GPU equivalent).
    ///
    /// Maps to CesiumJS `MetadataComponentType.toComponentDatatype`.
    pub fn to_component_datatype(&self) -> Option<u32> {
        match self {
            Self::Int8 => Some(5120),
            Self::Uint8 => Some(5121),
            Self::Int16 => Some(5122),
            Self::Uint16 => Some(5123),
            Self::Int32 => Some(5124),
            Self::Uint32 => Some(5125),
            Self::Float32 => Some(5126),
            Self::Float64 => Some(5130),
            Self::Int64 | Self::Uint64 => None,
        }
    }

    /// Gets the downcast function result for a value.
    /// INT64 → clamp to INT32, UINT64 → clamp to UINT32, FLOAT64 → f32 precision.
    ///
    /// Maps to CesiumJS `MetadataComponentType.downcastFunction`.
    pub fn downcast(&self, value: f64) -> f64 {
        match self {
            Self::Int64 => {
                let min = i32::MIN as f64;
                let max = i32::MAX as f64;
                value.max(min).min(max)
            }
            Self::Uint64 => {
                let min = 0.0_f64;
                let max = u32::MAX as f64;
                value.max(min).min(max)
            }
            Self::Float64 => (value as f32) as f64,
            _ => value,
        }
    }
}
