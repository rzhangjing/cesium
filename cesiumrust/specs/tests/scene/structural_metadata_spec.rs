//! StructuralMetadata specs - ported from Scene/StructuralMetadataSpec, PropertyTableSpec, etc.
//! Covers: MetadataType, MetadataComponentType, MetadataValue, MetadataClass,
//! MetadataEnum, PropertyTable, PropertyTexture, PropertyAttribute, StructuralMetadata

use cesium_tileset::structural_metadata::{
    MetadataClass, MetadataClassProperty, MetadataComponentType, MetadataEnum, MetadataType,
    MetadataValue, PropertyAttribute, PropertyTable, PropertyTexture, StructuralMetadata,
};

// ─── MetadataType ───────────────────────────────────────────────────────────

#[test]
fn metadata_type_byte_size() {
    assert_eq!(MetadataType::Int8.byte_size(), 1);
    assert_eq!(MetadataType::Uint8.byte_size(), 1);
    assert_eq!(MetadataType::Boolean.byte_size(), 1);
    assert_eq!(MetadataType::Int16.byte_size(), 2);
    assert_eq!(MetadataType::Uint16.byte_size(), 2);
    assert_eq!(MetadataType::Int32.byte_size(), 4);
    assert_eq!(MetadataType::Float32.byte_size(), 4);
    assert_eq!(MetadataType::Int64.byte_size(), 8);
    assert_eq!(MetadataType::Float64.byte_size(), 8);
    assert_eq!(MetadataType::String.byte_size(), 0);
}

// ─── MetadataComponentType ──────────────────────────────────────────────────

#[test]
fn metadata_component_type_count() {
    assert_eq!(MetadataComponentType::Scalar.component_count(), 1);
    assert_eq!(MetadataComponentType::Vec2.component_count(), 2);
    assert_eq!(MetadataComponentType::Vec3.component_count(), 3);
    assert_eq!(MetadataComponentType::Vec4.component_count(), 4);
    assert_eq!(MetadataComponentType::Mat2.component_count(), 4);
    assert_eq!(MetadataComponentType::Mat3.component_count(), 9);
    assert_eq!(MetadataComponentType::Mat4.component_count(), 16);
}

// ─── MetadataValue ──────────────────────────────────────────────────────────

#[test]
fn metadata_value_as_f64() {
    assert_eq!(MetadataValue::Int(42).as_f64(), Some(42.0));
    assert_eq!(MetadataValue::Uint(100).as_f64(), Some(100.0));
    assert_eq!(MetadataValue::Float(3.14).as_f64(), Some(3.14));
    assert_eq!(MetadataValue::Bool(true).as_f64(), Some(1.0));
    assert_eq!(MetadataValue::Bool(false).as_f64(), Some(0.0));
    assert_eq!(MetadataValue::String("hi".into()).as_f64(), None);
}

#[test]
fn metadata_value_variants() {
    let v = MetadataValue::Array(vec![
        MetadataValue::Int(1),
        MetadataValue::Int(2),
        MetadataValue::Int(3),
    ]);
    if let MetadataValue::Array(items) = v {
        assert_eq!(items.len(), 3);
    } else {
        panic!("expected Array variant");
    }
}

// ─── MetadataClassProperty ──────────────────────────────────────────────────

#[test]
fn metadata_class_property_scalar() {
    let prop = MetadataClassProperty::new_scalar("height", MetadataType::Float32);
    assert_eq!(prop.name, "height");
    assert_eq!(prop.value_type, MetadataType::Float32);
    assert_eq!(prop.component_type, MetadataComponentType::Scalar);
}

#[test]
fn metadata_class_property_vector() {
    let prop = MetadataClassProperty::new_vector(
        "position",
        MetadataType::Float64,
        MetadataComponentType::Vec3,
    );
    assert_eq!(prop.component_type, MetadataComponentType::Vec3);
}

// ─── MetadataClass ──────────────────────────────────────────────────────────

#[test]
fn metadata_class_creation() {
    let mut class = MetadataClass::new("building");
    class.properties.insert(
        "height".to_string(),
        MetadataClassProperty::new_scalar("height", MetadataType::Float32),
    );
    assert_eq!(class.id, "building");
    assert!(class.get_property("height").is_some());
    assert!(class.get_property("nonexistent").is_none());
}

// ─── MetadataEnum ───────────────────────────────────────────────────────────

#[test]
fn metadata_enum_creation() {
    let e = MetadataEnum::new("color_type", MetadataType::Uint8);
    assert_eq!(e.id, "color_type");
    assert_eq!(e.value_type, MetadataType::Uint8);
}

// ─── PropertyTable ──────────────────────────────────────────────────────────

#[test]
fn property_table_creation() {
    let class = MetadataClass::new("test_class");
    let table = PropertyTable::new(10, class);
    assert_eq!(table.count, 10);
    assert_eq!(table.property_count(), 0);
}

// ─── PropertyTexture ────────────────────────────────────────────────────────

#[test]
fn property_texture_creation() {
    let class = MetadataClass::new("texture_class");
    let texture = PropertyTexture::new(class);
    assert!(texture.get_property("nonexistent").is_none());
}

// ─── PropertyAttribute ──────────────────────────────────────────────────────

#[test]
fn property_attribute_creation() {
    let class = MetadataClass::new("attr_class");
    let attr = PropertyAttribute::new(class);
    assert!(attr.get_property("nonexistent").is_none());
}

// ─── StructuralMetadata ─────────────────────────────────────────────────────

#[test]
fn structural_metadata_empty() {
    let sm = StructuralMetadata::new();
    assert!(sm.property_tables.is_empty());
    assert!(sm.classes.is_empty());
}

#[test]
fn structural_metadata_add_table() {
    let mut sm = StructuralMetadata::new();
    let class = MetadataClass::new("buildings");
    sm.classes.insert("buildings".to_string(), class.clone());
    let table = PropertyTable::new(5, class);
    sm.add_property_table(table);
    assert_eq!(sm.property_tables.len(), 1);
    assert!(sm.get_property_table(0).is_some());
    assert!(sm.get_class("buildings").is_some());
}
