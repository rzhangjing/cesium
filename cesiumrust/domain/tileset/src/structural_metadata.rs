//! Structural metadata for EXT_structural_metadata extension.
//!
//! Maps to CesiumJS:
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

/// Metadata property types per EXT_structural_metadata spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetadataType {
    /// 8-bit signed integer.
    Int8,
    /// 8-bit unsigned integer.
    Uint8,
    /// 16-bit signed integer.
    Int16,
    /// 16-bit unsigned integer.
    Uint16,
    /// 32-bit signed integer.
    Int32,
    /// 32-bit unsigned integer.
    Uint32,
    /// 64-bit signed integer.
    Int64,
    /// 64-bit unsigned integer.
    Uint64,
    /// 32-bit float.
    Float32,
    /// 64-bit float.
    Float64,
    /// Boolean.
    Boolean,
    /// String.
    String,
    /// Enum.
    Enum,
}

impl MetadataType {
    /// Get the byte size of this type (0 for variable-length types).
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

/// Metadata component type (scalar, vecN, matN).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataComponentType {
    /// Scalar value.
    Scalar,
    /// 2-component vector.
    Vec2,
    /// 3-component vector.
    Vec3,
    /// 4-component vector.
    Vec4,
    /// 2x2 matrix.
    Mat2,
    /// 3x3 matrix.
    Mat3,
    /// 4x4 matrix.
    Mat4,
}

impl MetadataComponentType {
    /// Get the number of components.
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

/// A metadata property value.
#[derive(Debug, Clone, PartialEq)]
pub enum MetadataValue {
    /// Boolean value.
    Bool(bool),
    /// Integer value (i64 covers all int types).
    Int(i64),
    /// Unsigned integer value.
    Uint(u64),
    /// Float value.
    Float(f64),
    /// String value.
    String(String),
    /// Array of values.
    Array(Vec<MetadataValue>),
}

impl MetadataValue {
    /// Get as f64 if numeric.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Int(v) => Some(*v as f64),
            Self::Uint(v) => Some(*v as f64),
            Self::Float(v) => Some(*v),
            Self::Bool(v) => Some(if *v { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    /// Get as string reference.
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

/// Definition of a single property in a metadata class.
#[derive(Debug, Clone, PartialEq)]
pub struct MetadataClassProperty {
    /// Property name.
    pub name: String,
    /// Property description.
    pub description: Option<String>,
    /// Value type.
    pub value_type: MetadataType,
    /// Component type (for vectors/matrices).
    pub component_type: MetadataComponentType,
    /// Whether this is an array property.
    pub array: bool,
    /// Whether this property is required.
    pub required: bool,
    /// No-data value (used when property is missing).
    pub no_data: Option<MetadataValue>,
    /// Default value.
    pub default: Option<MetadataValue>,
    /// Normalization flag.
    pub normalized: bool,
    /// Offset for dequantization.
    pub offset: Option<MetadataValue>,
    /// Scale for dequantization.
    pub scale: Option<MetadataValue>,
    /// Maximum value.
    pub max: Option<MetadataValue>,
    /// Minimum value.
    pub min: Option<MetadataValue>,
    /// Enum ID (if type is Enum).
    pub enum_id: Option<String>,
}

impl MetadataClassProperty {
    /// Create a new scalar property.
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

    /// Create a new vector property.
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

/// A metadata class definition (schema class).
#[derive(Debug, Clone, PartialEq)]
pub struct MetadataClass {
    /// Class ID.
    pub id: String,
    /// Human-readable name.
    pub name: Option<String>,
    /// Description.
    pub description: Option<String>,
    /// Properties in this class.
    pub properties: HashMap<String, MetadataClassProperty>,
}

impl MetadataClass {
    /// Create a new empty class.
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            name: None,
            description: None,
            properties: HashMap::new(),
        }
    }

    /// Add a property to the class.
    pub fn add_property(&mut self, property: MetadataClassProperty) {
        self.properties.insert(property.name.clone(), property);
    }

    /// Get a property by ID.
    pub fn get_property(&self, id: &str) -> Option<&MetadataClassProperty> {
        self.properties.get(id)
    }
}

// ============================================================================
// MetadataEnum
// ============================================================================

/// A metadata enum definition.
#[derive(Debug, Clone, PartialEq)]
pub struct MetadataEnum {
    /// Enum ID.
    pub id: String,
    /// Human-readable name.
    pub name: Option<String>,
    /// Description.
    pub description: Option<String>,
    /// Value type (Int8, Uint8, Int16, etc.).
    pub value_type: MetadataType,
    /// Enum values: name → numeric value.
    pub values: HashMap<String, i64>,
}

impl MetadataEnum {
    /// Create a new enum.
    pub fn new(id: &str, value_type: MetadataType) -> Self {
        Self {
            id: id.to_string(),
            name: None,
            description: None,
            value_type,
            values: HashMap::new(),
        }
    }

    /// Add a value to the enum.
    pub fn add_value(&mut self, name: &str, value: i64) {
        self.values.insert(name.to_string(), value);
    }

    /// Get the name for a numeric value.
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

/// A property table containing per-feature metadata.
///
/// Maps to CesiumJS `Scene/PropertyTable.js`.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyTable {
    /// Table name.
    pub name: Option<String>,
    /// Table ID.
    pub id: Option<String>,
    /// Number of features.
    pub count: usize,
    /// Class this table conforms to.
    pub class: MetadataClass,
    /// Property values: property_id → feature_index → value.
    pub values: HashMap<String, Vec<MetadataValue>>,
    /// Extra user-defined data.
    pub extras: Option<serde_json::Value>,
}

impl PropertyTable {
    /// Create a new property table.
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

    /// Set a property value for a feature.
    pub fn set_value(&mut self, property_id: &str, feature_index: usize, value: MetadataValue) {
        let values = self
            .values
            .entry(property_id.to_string())
            .or_insert_with(|| vec![MetadataValue::Bool(false); self.count]);
        if feature_index < values.len() {
            values[feature_index] = value;
        }
    }

    /// Get a property value for a feature.
    pub fn get_value(&self, property_id: &str, feature_index: usize) -> Option<&MetadataValue> {
        self.values
            .get(property_id)
            .and_then(|v| v.get(feature_index))
    }

    /// Get all property IDs in this table.
    pub fn property_ids(&self) -> Vec<&str> {
        self.values.keys().map(|s| s.as_str()).collect()
    }

    /// Get the number of properties.
    pub fn property_count(&self) -> usize {
        self.values.len()
    }
}

// ============================================================================
// PropertyTexture
// ============================================================================

/// A property stored in a texture.
///
/// Maps to CesiumJS `Scene/PropertyTexture.js`.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyTexture {
    /// Texture name.
    pub name: Option<String>,
    /// Texture ID.
    pub id: Option<String>,
    /// Class this texture conforms to.
    pub class: MetadataClass,
    /// Property definitions: property_id → texture channel info.
    pub properties: HashMap<String, PropertyTextureProperty>,
    /// Extra user-defined data.
    pub extras: Option<serde_json::Value>,
}

/// A single property within a property texture.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyTextureProperty {
    /// Texture index.
    pub texture_index: usize,
    /// Texture coordinate set index.
    pub tex_coord: usize,
    /// Channel indices (e.g., [0,1,2] for RGB).
    pub channels: Vec<usize>,
}

impl PropertyTexture {
    /// Create a new property texture.
    pub fn new(class: MetadataClass) -> Self {
        Self {
            name: None,
            id: None,
            class,
            properties: HashMap::new(),
            extras: None,
        }
    }

    /// Add a property to the texture.
    pub fn add_property(&mut self, property_id: &str, prop: PropertyTextureProperty) {
        self.properties.insert(property_id.to_string(), prop);
    }

    /// Get a property by ID.
    pub fn get_property(&self, property_id: &str) -> Option<&PropertyTextureProperty> {
        self.properties.get(property_id)
    }
}

// ============================================================================
// PropertyAttribute
// ============================================================================

/// Per-vertex properties stored as custom attributes.
///
/// Maps to CesiumJS `Scene/PropertyAttribute.js`.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyAttribute {
    /// Attribute name.
    pub name: Option<String>,
    /// Attribute ID.
    pub id: Option<String>,
    /// Class this attribute conforms to.
    pub class: MetadataClass,
    /// Property definitions: property_id → attribute name in geometry.
    pub properties: HashMap<String, PropertyAttributeProperty>,
    /// Extra user-defined data.
    pub extras: Option<serde_json::Value>,
}

/// A single property within a property attribute.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyAttributeProperty {
    /// The vertex attribute name (e.g., "_HEIGHT").
    pub attribute: String,
}

impl PropertyAttribute {
    /// Create a new property attribute.
    pub fn new(class: MetadataClass) -> Self {
        Self {
            name: None,
            id: None,
            class,
            properties: HashMap::new(),
            extras: None,
        }
    }

    /// Add a property to the attribute.
    pub fn add_property(&mut self, property_id: &str, prop: PropertyAttributeProperty) {
        self.properties.insert(property_id.to_string(), prop);
    }

    /// Get a property by ID.
    pub fn get_property(&self, property_id: &str) -> Option<&PropertyAttributeProperty> {
        self.properties.get(property_id)
    }
}

// ============================================================================
// StructuralMetadata
// ============================================================================

/// Container for all structural metadata in a tile/model.
///
/// Maps to CesiumJS `Scene/StructuralMetadata.js`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StructuralMetadata {
    /// Property tables.
    pub property_tables: Vec<PropertyTable>,
    /// Property textures.
    pub property_textures: Vec<PropertyTexture>,
    /// Property attributes.
    pub property_attributes: Vec<PropertyAttribute>,
    /// Enum definitions.
    pub enums: HashMap<String, MetadataEnum>,
    /// Class definitions.
    pub classes: HashMap<String, MetadataClass>,
}

impl StructuralMetadata {
    /// Create empty structural metadata.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a property table.
    pub fn add_property_table(&mut self, table: PropertyTable) {
        self.property_tables.push(table);
    }

    /// Add a property texture.
    pub fn add_property_texture(&mut self, texture: PropertyTexture) {
        self.property_textures.push(texture);
    }

    /// Add a property attribute.
    pub fn add_property_attribute(&mut self, attribute: PropertyAttribute) {
        self.property_attributes.push(attribute);
    }

    /// Add an enum definition.
    pub fn add_enum(&mut self, metadata_enum: MetadataEnum) {
        self.enums.insert(metadata_enum.id.clone(), metadata_enum);
    }

    /// Add a class definition.
    pub fn add_class(&mut self, class: MetadataClass) {
        self.classes.insert(class.id.clone(), class);
    }

    /// Get a property table by index.
    pub fn get_property_table(&self, index: usize) -> Option<&PropertyTable> {
        self.property_tables.get(index)
    }

    /// Get a class by ID.
    pub fn get_class(&self, id: &str) -> Option<&MetadataClass> {
        self.classes.get(id)
    }

    /// Get an enum by ID.
    pub fn get_enum(&self, id: &str) -> Option<&MetadataEnum> {
        self.enums.get(id)
    }

    /// Whether the metadata is empty.
    pub fn is_empty(&self) -> bool {
        self.property_tables.is_empty()
            && self.property_textures.is_empty()
            && self.property_attributes.is_empty()
    }
}

// ============================================================================
// Tests
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
