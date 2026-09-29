//! EXT_structural_metadata 扩展的结构化元数据。
//!
//! 镜像 CesiumJS：
//! - `Scene/PropertyTable.js`
//! - `Scene/PropertyTexture.js`
//! - `Scene/PropertyAttribute.js`
//! - `Scene/StructuralMetadata.js`
//! - `Scene/MetadataClass.js`
//! - `Scene/MetadataClassProperty.js`
//! - `Scene/MetadataEnum.js`

use std::collections::HashMap;

// ============================================================================
// MetadataType
// ============================================================================

/// 按 EXT_structural_metadata 规范定义的元数据属性类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetadataType {
    /// 8 位有符号整数。
    Int8,
    /// 8 位无符号整数。
    Uint8,
    /// 16 位有符号整数。
    Int16,
    /// 16 位无符号整数。
    Uint16,
    /// 32 位有符号整数。
    Int32,
    /// 32 位无符号整数。
    Uint32,
    /// 64 位有符号整数。
    Int64,
    /// 64 位无符号整数。
    Uint64,
    /// 32 位浮点数。
    Float32,
    /// 64 位浮点数。
    Float64,
    /// 布尔值。
    Boolean,
    /// 字符串。
    String,
    /// 枚举。
    Enum,
}

impl MetadataType {
    /// 获取本类型的字节大小（变长类型为 0）。
    pub fn byte_size(&self) -> usize {
        match self {
            Self::Int8 | Self::Uint8 | Self::Boolean => 1,
            Self::Int16 | Self::Uint16 => 2,
            Self::Int32 | Self::Uint32 | Self::Float32 | Self::Enum => 4,
            Self::Int64 | Self::Uint64 | Self::Float64 => 8,
            Self::String => 0,
        }
    }
}

/// 元数据分量类型（标量、vecN、matN）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataComponentType {
    /// 标量值。
    Scalar,
    /// 2 分量向量。
    Vec2,
    /// 3 分量向量。
    Vec3,
    /// 4 分量向量。
    Vec4,
    /// 2x2 矩阵。
    Mat2,
    /// 3x3 矩阵。
    Mat3,
    /// 4x4 矩阵。
    Mat4,
}

impl MetadataComponentType {
    /// 获取分量数量。
    pub fn component_count(&self) -> usize {
        match self {
            Self::Scalar => 1,
            Self::Vec2 => 2,
            Self::Vec3 => 3,
            Self::Vec4 => 4,
            Self::Mat2 => 4,
            Self::Mat3 => 9,
            Self::Mat4 => 16,
        }
    }
}

// ============================================================================
// MetadataValue
// ============================================================================

/// 一个元数据属性值。
#[derive(Debug, Clone, PartialEq)]
pub enum MetadataValue {
    /// 布尔值。
    Bool(bool),
    /// 整数值（i64 涵盖所有整数类型）。
    Int(i64),
    /// 无符号整数值。
    Uint(u64),
    /// 浮点值。
    Float(f64),
    /// 字符串值。
    String(String),
    /// 值的数组。
    Array(Vec<MetadataValue>),
}

impl MetadataValue {
    /// 若为数值则作为 f64 获取。
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Int(v) => Some(*v as f64),
            Self::Uint(v) => Some(*v as f64),
            Self::Float(v) => Some(*v),
            Self::Bool(v) => Some(if *v { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    /// 作为字符串引用获取。
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }
}

// ============================================================================
// MetadataClassProperty
// ============================================================================

/// 元数据类中单个属性的定义。
#[derive(Debug, Clone, PartialEq)]
pub struct MetadataClassProperty {
    /// 属性名。
    pub name: String,
    /// 属性描述。
    pub description: Option<String>,
    /// 值类型。
    pub value_type: MetadataType,
    /// 分量类型（用于向量/矩阵）。
    pub component_type: MetadataComponentType,
    /// 本属性是否为数组属性。
    pub array: bool,
    /// 本属性是否为必需。
    pub required: bool,
    /// 无数据值（属性缺失时使用）。
    pub no_data: Option<MetadataValue>,
    /// 默认值。
    pub default: Option<MetadataValue>,
    /// 归一化标志。
    pub normalized: bool,
    /// 用于反量子化的偏移。
    pub offset: Option<MetadataValue>,
    /// 用于反量子化的缩放。
    pub scale: Option<MetadataValue>,
    /// 最大值。
    pub max: Option<MetadataValue>,
    /// 最小值。
    pub min: Option<MetadataValue>,
    /// 枚举 ID（若类型为 Enum）。
    pub enum_id: Option<String>,
}

impl MetadataClassProperty {
    /// 创建一个新的标量属性。
    pub fn new_scalar(name: &str, value_type: MetadataType) -> Self {
        Self {
            name: name.to_string(),
            description: None,
            value_type,
            component_type: MetadataComponentType::Scalar,
            array: false,
            required: false,
            no_data: None,
            default: None,
            normalized: false,
            offset: None,
            scale: None,
            max: None,
            min: None,
            enum_id: None,
        }
    }

    /// 创建一个新的向量属性。
    pub fn new_vector(
        name: &str,
        value_type: MetadataType,
        component_type: MetadataComponentType,
    ) -> Self {
        Self {
            name: name.to_string(),
            description: None,
            value_type,
            component_type,
            array: false,
            required: false,
            no_data: None,
            default: None,
            normalized: false,
            offset: None,
            scale: None,
            max: None,
            min: None,
            enum_id: None,
        }
    }
}

// ============================================================================
// MetadataClass
// ============================================================================

/// 一个元数据类定义（schema 类）。
#[derive(Debug, Clone, PartialEq)]
pub struct MetadataClass {
    /// 类 ID。
    pub id: String,
    /// 可读名称。
    pub name: Option<String>,
    /// 描述。
    pub description: Option<String>,
    /// 本类中的属性。
    pub properties: HashMap<String, MetadataClassProperty>,
}

impl MetadataClass {
    /// 创建一个空的类。
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            name: None,
            description: None,
            properties: HashMap::new(),
        }
    }

    /// 向类中添加一个属性。
    pub fn add_property(&mut self, property: MetadataClassProperty) {
        self.properties.insert(property.name.clone(), property);
    }

    /// 按 ID 获取一个属性。
    pub fn get_property(&self, id: &str) -> Option<&MetadataClassProperty> {
        self.properties.get(id)
    }
}

// ============================================================================
// MetadataEnum
// ============================================================================

/// 一个元数据枚举定义。
#[derive(Debug, Clone, PartialEq)]
pub struct MetadataEnum {
    /// 枚举 ID。
    pub id: String,
    /// 可读名称。
    pub name: Option<String>,
    /// 描述。
    pub description: Option<String>,
    /// 值类型（Int8、Uint8、Int16 等）。
    pub value_type: MetadataType,
    /// 枚举值：名称 → 数值。
    pub values: HashMap<String, i64>,
}

impl MetadataEnum {
    /// 创建一个新的枚举。
    pub fn new(id: &str, value_type: MetadataType) -> Self {
        Self {
            id: id.to_string(),
            name: None,
            description: None,
            value_type,
            values: HashMap::new(),
        }
    }

    /// 向枚举中添加一个值。
    pub fn add_value(&mut self, name: &str, value: i64) {
        self.values.insert(name.to_string(), value);
    }

    /// 获取一个数值对应的名称。
    pub fn name_for_value(&self, value: i64) -> Option<&str> {
        self.values
            .iter()
            .find(|(_, v)| **v == value)
            .map(|(k, _)| k.as_str())
    }
}

// ============================================================================
// PropertyTable
// ============================================================================

/// 包含逐 feature 元数据的属性表。
///
/// 映射到 CesiumJS `Scene/PropertyTable.js`。
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyTable {
    /// 表名。
    pub name: Option<String>,
    /// 表 ID。
    pub id: Option<String>,
    /// feature 数量。
    pub count: usize,
    /// 本表所从属的类。
    pub class: MetadataClass,
    /// 属性值：property_id → feature_index → value。
    pub values: HashMap<String, Vec<MetadataValue>>,
    /// 额外的用户自定义数据。
    pub extras: Option<serde_json::Value>,
}

impl PropertyTable {
    /// 创建一个新的属性表。
    pub fn new(count: usize, class: MetadataClass) -> Self {
        Self {
            name: None,
            id: None,
            count,
            class,
            values: HashMap::new(),
            extras: None,
        }
    }

    /// 为一个 feature 设置属性值。
    pub fn set_value(&mut self, property_id: &str, feature_index: usize, value: MetadataValue) {
        let values = self
            .values
            .entry(property_id.to_string())
            .or_insert_with(|| vec![MetadataValue::Bool(false); self.count]);
        if feature_index < values.len() {
            values[feature_index] = value;
        }
    }

    /// 获取一个 feature 的属性值。
    pub fn get_value(&self, property_id: &str, feature_index: usize) -> Option<&MetadataValue> {
        self.values
            .get(property_id)
            .and_then(|v| v.get(feature_index))
    }

    /// 获取本表中的所有属性 ID。
    pub fn property_ids(&self) -> Vec<&str> {
        self.values.keys().map(|s| s.as_str()).collect()
    }

    /// 获取属性数量。
    pub fn property_count(&self) -> usize {
        self.values.len()
    }
}

// ============================================================================
// PropertyTexture
// ============================================================================

/// 存储于纹理中的属性。
///
/// 映射到 CesiumJS `Scene/PropertyTexture.js`。
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyTexture {
    /// 纹理名。
    pub name: Option<String>,
    /// 纹理 ID。
    pub id: Option<String>,
    /// 本纹理所从属的类。
    pub class: MetadataClass,
    /// 属性定义：property_id → 纹理通道信息。
    pub properties: HashMap<String, PropertyTextureProperty>,
    /// 额外的用户自定义数据。
    pub extras: Option<serde_json::Value>,
}

/// 属性纹理中的单个属性。
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyTextureProperty {
    /// 纹理索引。
    pub texture_index: usize,
    /// 纹理坐标集索引。
    pub tex_coord: usize,
    /// 通道索引（例如 RGB 对应 [0,1,2]）。
    pub channels: Vec<usize>,
}

impl PropertyTexture {
    /// 创建一个新的属性纹理。
    pub fn new(class: MetadataClass) -> Self {
        Self {
            name: None,
            id: None,
            class,
            properties: HashMap::new(),
            extras: None,
        }
    }

    /// 向纹理中添加一个属性。
    pub fn add_property(&mut self, property_id: &str, prop: PropertyTextureProperty) {
        self.properties.insert(property_id.to_string(), prop);
    }

    /// 按 ID 获取一个属性。
    pub fn get_property(&self, property_id: &str) -> Option<&PropertyTextureProperty> {
        self.properties.get(property_id)
    }
}

// ============================================================================
// PropertyAttribute
// ============================================================================

/// 作为自定义属性存储的逐顶点属性。
///
/// 映射到 CesiumJS `Scene/PropertyAttribute.js`。
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyAttribute {
    /// 属性名。
    pub name: Option<String>,
    /// 属性 ID。
    pub id: Option<String>,
    /// 本属性所从属的类。
    pub class: MetadataClass,
    /// 属性定义：property_id → 几何中的属性名。
    pub properties: HashMap<String, PropertyAttributeProperty>,
    /// 额外的用户自定义数据。
    pub extras: Option<serde_json::Value>,
}

/// 属性 attribute 中的单个属性。
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyAttributeProperty {
    /// 顶点属性名（例如 "_HEIGHT"）。
    pub attribute: String,
}

impl PropertyAttribute {
    /// 创建一个新的属性 attribute。
    pub fn new(class: MetadataClass) -> Self {
        Self {
            name: None,
            id: None,
            class,
            properties: HashMap::new(),
            extras: None,
        }
    }

    /// 向 attribute 中添加一个属性。
    pub fn add_property(&mut self, property_id: &str, prop: PropertyAttributeProperty) {
        self.properties.insert(property_id.to_string(), prop);
    }

    /// 按 ID 获取一个属性。
    pub fn get_property(&self, property_id: &str) -> Option<&PropertyAttributeProperty> {
        self.properties.get(property_id)
    }
}

// ============================================================================
// StructuralMetadata
// ============================================================================

/// 一个瓦片/模型中所有结构化元数据的容器。
///
/// 映射到 CesiumJS `Scene/StructuralMetadata.js`。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StructuralMetadata {
    /// 属性表。
    pub property_tables: Vec<PropertyTable>,
    /// 属性纹理。
    pub property_textures: Vec<PropertyTexture>,
    /// 属性 attribute。
    pub property_attributes: Vec<PropertyAttribute>,
    /// 枚举定义。
    pub enums: HashMap<String, MetadataEnum>,
    /// 类定义。
    pub classes: HashMap<String, MetadataClass>,
}

impl StructuralMetadata {
    /// 创建空的结构化元数据。
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加一个属性表。
    pub fn add_property_table(&mut self, table: PropertyTable) {
        self.property_tables.push(table);
    }

    /// 添加一个属性纹理。
    pub fn add_property_texture(&mut self, texture: PropertyTexture) {
        self.property_textures.push(texture);
    }

    /// 添加一个属性 attribute。
    pub fn add_property_attribute(&mut self, attribute: PropertyAttribute) {
        self.property_attributes.push(attribute);
    }

    /// 添加一个枚举定义。
    pub fn add_enum(&mut self, metadata_enum: MetadataEnum) {
        self.enums.insert(metadata_enum.id.clone(), metadata_enum);
    }

    /// 添加一个类定义。
    pub fn add_class(&mut self, class: MetadataClass) {
        self.classes.insert(class.id.clone(), class);
    }

    /// 按索引获取一个属性表。
    pub fn get_property_table(&self, index: usize) -> Option<&PropertyTable> {
        self.property_tables.get(index)
    }

    /// 按 ID 获取一个类。
    pub fn get_class(&self, id: &str) -> Option<&MetadataClass> {
        self.classes.get(id)
    }

    /// 按 ID 获取一个枚举。
    pub fn get_enum(&self, id: &str) -> Option<&MetadataEnum> {
        self.enums.get(id)
    }

    /// 元数据是否为空。
    pub fn is_empty(&self) -> bool {
        self.property_tables.is_empty()
            && self.property_textures.is_empty()
            && self.property_attributes.is_empty()
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_type_byte_size() {
        assert_eq!(MetadataType::Int8.byte_size(), 1);
        assert_eq!(MetadataType::Uint16.byte_size(), 2);
        assert_eq!(MetadataType::Float32.byte_size(), 4);
        assert_eq!(MetadataType::Float64.byte_size(), 8);
        assert_eq!(MetadataType::String.byte_size(), 0);
    }

    #[test]
    fn test_component_count() {
        assert_eq!(MetadataComponentType::Scalar.component_count(), 1);
        assert_eq!(MetadataComponentType::Vec3.component_count(), 3);
        assert_eq!(MetadataComponentType::Mat4.component_count(), 16);
    }

    #[test]
    #[allow(clippy::approx_constant)] // 3.14 为任意测试数据，并非 PI；参见 docs/deferred.md #1
    fn test_metadata_value_as_f64() {
        assert_eq!(MetadataValue::Int(42).as_f64(), Some(42.0));
        assert_eq!(MetadataValue::Uint(10).as_f64(), Some(10.0));
        assert_eq!(MetadataValue::Float(3.14).as_f64(), Some(3.14));
        assert_eq!(MetadataValue::Bool(true).as_f64(), Some(1.0));
        assert_eq!(MetadataValue::String("x".into()).as_f64(), None);
    }

    #[test]
    fn test_metadata_class() {
        let mut class = MetadataClass::new("building");
        class.name = Some("Building".to_string());
        class.add_property(MetadataClassProperty::new_scalar("height", MetadataType::Float32));
        class.add_property(MetadataClassProperty::new_scalar("name", MetadataType::String));

        assert_eq!(class.properties.len(), 2);
        assert!(class.get_property("height").is_some());
        assert!(class.get_property("missing").is_none());
    }

    #[test]
    fn test_metadata_enum() {
        let mut e = MetadataEnum::new("color", MetadataType::Uint8);
        e.add_value("red", 0);
        e.add_value("green", 1);
        e.add_value("blue", 2);

        assert_eq!(e.name_for_value(1), Some("green"));
        assert_eq!(e.name_for_value(99), None);
    }

    #[test]
    fn test_property_table() {
        let mut class = MetadataClass::new("feature");
        class.add_property(MetadataClassProperty::new_scalar("height", MetadataType::Float32));

        let mut table = PropertyTable::new(3, class);
        table.name = Some("Buildings".to_string());

        table.set_value("height", 0, MetadataValue::Float(10.5));
        table.set_value("height", 1, MetadataValue::Float(20.0));
        table.set_value("height", 2, MetadataValue::Float(15.3));

        assert_eq!(table.count, 3);
        assert_eq!(
            table.get_value("height", 1),
            Some(&MetadataValue::Float(20.0))
        );
        assert_eq!(table.get_value("height", 5), None);
        assert_eq!(table.property_count(), 1);
    }

    #[test]
    fn test_property_texture() {
        let class = MetadataClass::new("texture_class");
        let mut tex = PropertyTexture::new(class);
        tex.name = Some("HeightMap".to_string());

        tex.add_property(
            "height",
            PropertyTextureProperty {
                texture_index: 0,
                tex_coord: 0,
                channels: vec![0],
            },
        );

        assert!(tex.get_property("height").is_some());
        assert!(tex.get_property("missing").is_none());
    }

    #[test]
    fn test_property_attribute() {
        let class = MetadataClass::new("vertex_class");
        let mut attr = PropertyAttribute::new(class);
        attr.name = Some("PerVertex".to_string());

        attr.add_property(
            "height",
            PropertyAttributeProperty {
                attribute: "_HEIGHT".to_string(),
            },
        );

        assert!(attr.get_property("height").is_some());
        assert_eq!(
            attr.get_property("height").unwrap().attribute,
            "_HEIGHT"
        );
    }

    #[test]
    fn test_structural_metadata() {
        let mut metadata = StructuralMetadata::new();
        assert!(metadata.is_empty());

        let class = MetadataClass::new("building");
        metadata.add_class(class.clone());

        let table = PropertyTable::new(10, class);
        metadata.add_property_table(table);

        let mut e = MetadataEnum::new("type", MetadataType::Uint8);
        e.add_value("residential", 0);
        e.add_value("commercial", 1);
        metadata.add_enum(e);

        assert!(!metadata.is_empty());
        assert_eq!(metadata.property_tables.len(), 1);
        assert!(metadata.get_class("building").is_some());
        assert!(metadata.get_enum("type").is_some());
        assert!(metadata.get_property_table(0).is_some());
        assert!(metadata.get_property_table(5).is_none());
    }

    #[test]
    fn test_vector_property() {
        let prop = MetadataClassProperty::new_vector(
            "position",
            MetadataType::Float32,
            MetadataComponentType::Vec3,
        );
        assert_eq!(prop.component_type, MetadataComponentType::Vec3);
        assert!(!prop.array);
    }
}
