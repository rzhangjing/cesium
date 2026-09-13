//! Structural metadata deep specs - ported from MetadataClassPropertySpec.js,
//! MetadataEntitySpec.js, GroupMetadataSpec.js, TilesetMetadataSpec.js
//!
//! Tests MetadataClassProperty validation, MetadataClass, MetadataEnum,
//! PropertyTable get/set, PropertyTexture, PropertyAttribute, StructuralMetadata container.

use cesium_tileset::structural_metadata::{
    MetadataClass, MetadataClassProperty, MetadataComponentType, MetadataEnum, MetadataType,
    MetadataValue, PropertyAttribute, PropertyAttributeProperty, PropertyTable, PropertyTexture,
    PropertyTextureProperty, StructuralMetadata,
};

// ─── MetadataType ──────────────────────────────────────────────────────────

#[test]
fn metadata_type_byte_sizes() {
    assert_eq!(MetadataType::Int8.byte_size(), 1);
    assert_eq!(MetadataType::Uint8.byte_size(), 1);
    assert_eq!(MetadataType::Int16.byte_size(), 2);
    assert_eq!(MetadataType::Uint16.byte_size(), 2);
    assert_eq!(MetadataType::Int32.byte_size(), 4);
    assert_eq!(MetadataType::Uint32.byte_size(), 4);
    assert_eq!(MetadataType::Int64.byte_size(), 8);
    assert_eq!(MetadataType::Uint64.byte_size(), 8);
    assert_eq!(MetadataType::Float32.byte_size(), 4);
    assert_eq!(MetadataType::Float64.byte_size(), 8);
    assert_eq!(MetadataType::Boolean.byte_size(), 1);
    assert_eq!(MetadataType::String.byte_size(), 0);
}

// ─── MetadataComponentType ─────────────────────────────────────────────────

#[test]
fn component_type_counts() {
    assert_eq!(MetadataComponentType::Scalar.component_count(), 1);
    assert_eq!(MetadataComponentType::Vec2.component_count(), 2);
    assert_eq!(MetadataComponentType::Vec3.component_count(), 3);
    assert_eq!(MetadataComponentType::Vec4.component_count(), 4);
    assert_eq!(MetadataComponentType::Mat2.component_count(), 4);
    assert_eq!(MetadataComponentType::Mat3.component_count(), 9);
    assert_eq!(MetadataComponentType::Mat4.component_count(), 16);
}

// ─── MetadataValue ─────────────────────────────────────────────────────────

#[test]
fn metadata_value_as_f64_all_types() {
    assert_eq!(MetadataValue::Int(-5).as_f64(), Some(-5.0));
    assert_eq!(MetadataValue::Uint(42).as_f64(), Some(42.0));
    assert_eq!(MetadataValue::Float(2.718).as_f64(), Some(2.718));
    assert_eq!(MetadataValue::Bool(false).as_f64(), Some(0.0));
    assert_eq!(MetadataValue::Bool(true).as_f64(), Some(1.0));
    assert_eq!(MetadataValue::String("hello".into()).as_f64(), None);
}

#[test]
fn metadata_value_as_str() {
    assert_eq!(MetadataValue::String("test".into()).as_str(), Some("test"));
    assert_eq!(MetadataValue::Int(1).as_str(), None);
    assert_eq!(MetadataValue::Float(1.0).as_str(), None);
}

// ─── MetadataClassProperty ─────────────────────────────────────────────────

#[test]
fn class_property_new_scalar() {
    let prop = MetadataClassProperty::new_scalar("height", MetadataType::Float32);
    assert_eq!(prop.name, "height");
    assert_eq!(prop.value_type, MetadataType::Float32);
    assert_eq!(prop.component_type, MetadataComponentType::Scalar);
    assert!(!prop.required);
    assert!(prop.default.is_none());
}

#[test]
fn class_property_new_vector() {
    let prop = MetadataClassProperty::new_vector("position", MetadataType::Float64, MetadataComponentType::Vec3);
    assert_eq!(prop.name, "position");
    assert_eq!(prop.value_type, MetadataType::Float64);
    assert_eq!(prop.component_type, MetadataComponentType::Vec3);
}

#[test]
fn class_property_with_options() {
    let mut prop = MetadataClassProperty::new_scalar("id", MetadataType::Uint32);
    prop.required = true;
    prop.default = Some(MetadataValue::Uint(0));
    prop.description = Some("Feature identifier".to_string());
    assert!(prop.required);
    assert_eq!(prop.default, Some(MetadataValue::Uint(0)));
}

// ─── MetadataClass ─────────────────────────────────────────────────────────

#[test]
fn metadata_class_add_and_get() {
    let mut class = MetadataClass::new("building");
    class.name = Some("Building".to_string());
    class.add_property(MetadataClassProperty::new_scalar("height", MetadataType::Float32));
    class.add_property(MetadataClassProperty::new_scalar("floors", MetadataType::Uint8));
    class.add_property(MetadataClassProperty::new_vector("center", MetadataType::Float64, MetadataComponentType::Vec3));

    assert_eq!(class.properties.len(), 3);
    assert!(class.get_property("height").is_some());
    assert!(class.get_property("floors").is_some());
    assert!(class.get_property("center").is_some());
    assert!(class.get_property("nonexistent").is_none());
}

#[test]
fn metadata_class_id() {
    let class = MetadataClass::new("road");
    assert_eq!(class.id, "road");
}

// ─── MetadataEnum ──────────────────────────────────────────────────────────

#[test]
fn metadata_enum_operations() {
    let mut e = MetadataEnum::new("land_use", MetadataType::Uint16);
    e.name = Some("Land Use Type".to_string());
    e.add_value("residential", 0);
    e.add_value("commercial", 1);
    e.add_value("industrial", 2);
    e.add_value("park", 3);

    assert_eq!(e.values.len(), 4);
    assert_eq!(e.name_for_value(0), Some("residential"));
    assert_eq!(e.name_for_value(2), Some("industrial"));
    assert_eq!(e.name_for_value(3), Some("park"));
    assert_eq!(e.name_for_value(99), None);
}

// ─── PropertyTable ─────────────────────────────────────────────────────────

#[test]
fn property_table_set_get() {
    let mut class = MetadataClass::new("feature");
    class.add_property(MetadataClassProperty::new_scalar("height", MetadataType::Float32));
    class.add_property(MetadataClassProperty::new_scalar("name", MetadataType::String));

    let mut table = PropertyTable::new(3, class);
    table.set_value("height", 0, MetadataValue::Float(10.5));
    table.set_value("height", 1, MetadataValue::Float(20.0));
    table.set_value("height", 2, MetadataValue::Float(15.3));
    table.set_value("name", 0, MetadataValue::String("A".into()));
    table.set_value("name", 1, MetadataValue::String("B".into()));

    assert_eq!(table.get_value("height", 0), Some(&MetadataValue::Float(10.5)));
    assert_eq!(table.get_value("height", 2), Some(&MetadataValue::Float(15.3)));
    assert_eq!(table.get_value("name", 1), Some(&MetadataValue::String("B".into())));
    // Unset index returns default Bool(false) due to initialization
    assert_eq!(table.get_value("name", 2), Some(&MetadataValue::Bool(false)));
    assert_eq!(table.get_value("missing", 0), None);
}

#[test]
fn property_table_count_and_ids() {
    let mut class = MetadataClass::new("f");
    class.add_property(MetadataClassProperty::new_scalar("a", MetadataType::Int8));
    class.add_property(MetadataClassProperty::new_scalar("b", MetadataType::Int8));

    let mut table = PropertyTable::new(10, class);
    assert_eq!(table.count, 10);
    // property_count reflects values map (properties with data set)
    assert_eq!(table.property_count(), 0);
    table.set_value("a", 0, MetadataValue::Int(1));
    table.set_value("b", 0, MetadataValue::Int(2));
    assert_eq!(table.property_count(), 2);
    let ids = table.property_ids();
    assert!(ids.contains(&"a"));
    assert!(ids.contains(&"b"));
}

#[test]
fn property_table_name() {
    let class = MetadataClass::new("x");
    let mut table = PropertyTable::new(5, class);
    assert!(table.name.is_none());
    table.name = Some("Buildings".to_string());
    assert_eq!(table.name.as_deref(), Some("Buildings"));
}

// ─── PropertyTexture ───────────────────────────────────────────────────────

#[test]
fn property_texture_operations() {
    let class = MetadataClass::new("texture_class");
    let mut tex = PropertyTexture::new(class);
    tex.name = Some("Facade".to_string());

    let prop = PropertyTextureProperty {
        texture_index: 0,
        tex_coord: 0,
        channels: vec![0, 1, 2, 3],
    };
    tex.add_property("color", prop);

    assert!(tex.get_property("color").is_some());
    assert!(tex.get_property("missing").is_none());
    let p = tex.get_property("color").unwrap();
    assert_eq!(p.channels, vec![0, 1, 2, 3]);
    assert_eq!(p.texture_index, 0);
}

// ─── PropertyAttribute ─────────────────────────────────────────────────────

#[test]
fn property_attribute_operations() {
    let class = MetadataClass::new("attr_class");
    let mut attr = PropertyAttribute::new(class);
    attr.name = Some("Per-vertex data".to_string());

    attr.add_property("height", PropertyAttributeProperty {
        attribute: "_HEIGHT".to_string(),
    });
    attr.add_property("id", PropertyAttributeProperty {
        attribute: "_FEATURE_ID".to_string(),
    });

    assert!(attr.get_property("height").is_some());
    assert_eq!(attr.get_property("height").unwrap().attribute, "_HEIGHT");
    assert!(attr.get_property("id").is_some());
    assert!(attr.get_property("missing").is_none());
}

// ─── StructuralMetadata container ──────────────────────────────────────────

#[test]
fn structural_metadata_empty() {
    let sm = StructuralMetadata::new();
    assert!(sm.is_empty());
    assert!(sm.get_property_table(0).is_none());
    assert!(sm.get_class("x").is_none());
    assert!(sm.get_enum("x").is_none());
}

#[test]
fn structural_metadata_add_and_query() {
    let mut sm = StructuralMetadata::new();

    // Add class
    let mut class = MetadataClass::new("building");
    class.add_property(MetadataClassProperty::new_scalar("height", MetadataType::Float32));
    sm.add_class(class);

    // Add enum
    let mut e = MetadataEnum::new("use_type", MetadataType::Uint8);
    e.add_value("res", 0);
    sm.add_enum(e);

    // Add property table
    let mut table_class = MetadataClass::new("building");
    table_class.add_property(MetadataClassProperty::new_scalar("height", MetadataType::Float32));
    let mut table = PropertyTable::new(2, table_class);
    table.set_value("height", 0, MetadataValue::Float(30.0));
    sm.add_property_table(table);

    // Add property texture
    let tex_class = MetadataClass::new("texture");
    sm.add_property_texture(PropertyTexture::new(tex_class));

    // Add property attribute
    let attr_class = MetadataClass::new("attr");
    sm.add_property_attribute(PropertyAttribute::new(attr_class));

    assert!(!sm.is_empty());
    assert!(sm.get_class("building").is_some());
    assert!(sm.get_enum("use_type").is_some());
    assert!(sm.get_property_table(0).is_some());
    assert_eq!(sm.property_tables.len(), 1);
    assert_eq!(sm.property_textures.len(), 1);
    assert_eq!(sm.property_attributes.len(), 1);
}

#[test]
fn structural_metadata_multiple_tables() {
    let mut sm = StructuralMetadata::new();
    for i in 0..5 {
        let class = MetadataClass::new(&format!("class_{i}"));
        sm.add_property_table(PropertyTable::new(i + 1, class));
    }
    assert_eq!(sm.property_tables.len(), 5);
    assert_eq!(sm.get_property_table(3).unwrap().count, 4);
    assert!(sm.get_property_table(5).is_none());
}
