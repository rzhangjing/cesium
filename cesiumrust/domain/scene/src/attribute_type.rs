//! 用于 3D Tiles 元数据与自定义着色器的 AttributeType 枚举。

// 遗留移植风格债（deferred.md #18）；将在 M13 lint 清理，或本文件在其所属里程碑被重写时重新审视
#![allow(clippy::should_implement_trait)]
/// 描述元数据与自定义着色器属性类型的枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttributeType {
    /// 单个标量值。
    Scalar,
    /// 一个二维向量。
    Vec2,
    /// 一个三维向量。
    Vec3,
    /// 一个四维向量。
    Vec4,
    /// 一个 2x2 矩阵。
    Mat2,
    /// 一个 3x3 矩阵。
    Mat3,
    /// 一个 4x4 矩阵。
    Mat4,
}

impl AttributeType {
    /// 获取该属性类型的 GLSL 类型字符串。
    ///
    /// 标量返回 `float`，向量/矩阵按维度返回对应的 `vecN`/`matN`。
    /// 结果用于拼接自定义着色器中的类型声明。
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

    /// 获取该属性类型的分量数量。
    ///
    /// 标量为 1，`vecN` 为 N，`matN` 为 N²。
    /// 反映底层数据打包时连续存储的分量总数。
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

    /// 获取该类型所需的属性位置数量。
    /// 矩阵需要多个位置（每行一个）。
    ///
    /// 标量与向量各占 1 个位置，`matN` 按行数占 N 个位置，
    /// 因 GLSL 中矩阵 attribute 的每一行需独立的位置槽。
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

    /// 获取该属性类型的数学类型名称。
    ///
    /// 返回对应的数学类型名（如 `Cartesian3`、`Matrix4`），
    /// 供样式求值与属性转换按名分派。
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

    /// 从其字符串表示解析属性类型。
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
