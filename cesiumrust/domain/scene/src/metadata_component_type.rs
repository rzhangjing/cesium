//! 用于 3D Tiles 元数据的 MetadataComponentType 枚举。
//!
//! 映射到 CesiumJS `Scene/MetadataComponentType.js`

/// 标量元数据分量类型的类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarCategory {
    /// 有符号整数类型（INT8, INT16, INT32, INT64）。
    Integer,
    /// 无符号整数类型（UINT8, UINT16, UINT32, UINT64）。
    UnsignedInteger,
    /// 浮点类型（FLOAT32, FLOAT64）。
    Float,
}

/// 用于 3D Tiles 元数据的元数据分量类型枚举。
///
/// 映射到 CesiumJS `Scene/MetadataComponentType.js`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetadataComponentType {
    /// 一个 8 位有符号整数。
    Int8,
    /// 一个 8 位无符号整数。
    Uint8,
    /// 一个 16 位有符号整数。
    Int16,
    /// 一个 16 位无符号整数。
    Uint16,
    /// 一个 32 位有符号整数。
    Int32,
    /// 一个 32 位无符号整数。
    Uint32,
    /// 一个 64 位有符号整数。
    Int64,
    /// 一个 64 位无符号整数。
    Uint64,
    /// 一个 32 位（单精度）浮点数。
    Float32,
    /// 一个 64 位（双精度）浮点数。
    Float64,
}

impl MetadataComponentType {
    /// 获取该数值类型的最小值。
    ///
    /// 映射到 CesiumJS `MetadataComponentType.getMinimum`。
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

    /// 获取该数值类型的最大值。
    ///
    /// 映射到 CesiumJS `MetadataComponentType.getMaximum`。
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

    /// 返回该类型是否为整数类型。
    ///
    /// 映射到 CesiumJS `MetadataComponentType.isIntegerType`。
    pub fn is_integer_type(&self) -> bool {
        self.category() != ScalarCategory::Float
    }

    /// 返回该类型是否为无符号整数类型。
    ///
    /// 映射到 CesiumJS `MetadataComponentType.isUnsignedIntegerType`。
    pub fn is_unsigned_integer_type(&self) -> bool {
        self.category() == ScalarCategory::UnsignedInteger
    }

    /// 获取该数值类型的类别。
    ///
    /// 映射到 CesiumJS `MetadataComponentType.category`。
    pub fn category(&self) -> ScalarCategory {
        match self {
            Self::Int8 | Self::Int16 | Self::Int32 | Self::Int64 => ScalarCategory::Integer,
            Self::Uint8 | Self::Uint16 | Self::Uint32 | Self::Uint64 => {
                ScalarCategory::UnsignedInteger
            }
            Self::Float32 | Self::Float64 => ScalarCategory::Float,
        }
    }

    /// 获取该数值类型的字节大小。
    ///
    /// 映射到 CesiumJS `MetadataComponentType.getSizeInBytes`。
    pub fn get_size_in_bytes(&self) -> usize {
        match self {
            Self::Int8 | Self::Uint8 => 1,
            Self::Int16 | Self::Uint16 => 2,
            Self::Int32 | Self::Uint32 | Self::Float32 => 4,
            Self::Int64 | Self::Uint64 | Self::Float64 => 8,
        }
    }

    /// 将一个整数值归一化到 [-1.0, 1.0]（有符号）或 [0.0, 1.0]（无符号）范围。
    ///
    /// 映射到 CesiumJS `MetadataComponentType.normalize`。
    pub fn normalize(&self, value: f64) -> f64 {
        let max = self.get_maximum();
        (value / max).max(-1.0)
    }

    /// 将 [-1.0, 1.0]（有符号）或 [0.0, 1.0]（无符号）范围内的值反归一化回整数。
    ///
    /// 映射到 CesiumJS `MetadataComponentType.unnormalize`。
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

    /// 从 ComponentDatatype 值转换为 MetadataComponentType。
    ///
    /// 映射到 CesiumJS `MetadataComponentType.fromComponentDatatype`。
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

    /// 转换为 ComponentDatatype 值。
    /// 对于 INT64/UINT64 返回 None（无对应的 GPU 类型）。
    ///
    /// 映射到 CesiumJS `MetadataComponentType.toComponentDatatype`。
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

    /// 获取某个值的向下转换（downcast）函数结果。
    /// INT64 → 钳制到 INT32，UINT64 → 钳制到 UINT32，FLOAT64 → f32 精度。
    ///
    /// 映射到 CesiumJS `MetadataComponentType.downcastFunction`。
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
