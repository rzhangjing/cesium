//! StructuralMetadata extended specs - ported from MetadataClassPropertySpec.js + PropertyTableSpec.js
//! Tests: MetadataClassProperty detailed, MetadataClass, MetadataEnum, PropertyTable get/set,
//! PropertyTexture, PropertyAttribute, StructuralMetadata

use cesium_tileset::structural_metadata::{
    MetadataClass, MetadataClassProperty, MetadataComponentType, MetadataEnum, MetadataType,
    MetadataValue, PropertyAttribute, PropertyTable, PropertyTexture, StructuralMetadata,
};

// ═══════════════════════════════════════════════════════════════════════════════
// MetadataType extended
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn metadata_type_all_byte_sizes() {
    assert_eq!(MetadataType::Int8.byte_size(), 1);
    assert_eq!(MetadataType::Uint8.byte_size(), 1);
    assert_eq!(MetadataType::Boolean.byte_size(), 1);
    assert_eq!(MetadataType::Int16.byte_size(), 2);
    assert_eq!(MetadataType::Uint16.byte_size(), 2);
    assert_eq!(MetadataType::Int32.byte_size(), 4);
    assert_eq!(MetadataType::Uint32.byte_size(), 4);
    assert_eq!(MetadataType::Float32.byte_size(), 4);
    assert_eq!(MetadataType::Enum.byte_size(), 4);
    assert_eq!(MetadataType::Int64.byte_size(), 8);
    assert_eq!(MetadataType::Uint64.byte_size(), 8);
    assert_eq!(MetadataType::Float64.byte_size(), 8);
    assert_eq!(MetadataType::String.byte_size(), 0);
}

#[test]
fn metadata_type_equality() {
    assert_eq!(MetadataType::Float32, MetadataType::Float32);
    assert_ne!(MetadataType::Float32, MetadataType::Float64);
}

// ═══════════════════════════════════════════════════════════════════════════════
// MetadataClassProperty extended
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn class_property_scalar_defaults() {
    let prop = MetadataClassProperty::new_scalar("height", MetadataType::Float32);
    assert_eq!(prop.name, "height");
    assert_eq!(prop.value_type, MetadataType::Float32);
    assert_eq!(prop.component_type, MetadataComponentType::Scalar);
    assert!(!prop.array);
    assert!(!prop.required);
    assert!(!prop.normalized);
    assert!(prop.no_data.is_none());
    assert!(prop.default.is_none());
    assert!(prop.offset.is_none());
    assert!(prop.scale.is_none());
    assert!(prop.min.is_none());
    assert!(prop.max.is_none());
    assert!(prop.enum_id.is_none());
    assert!(prop.description.is_none());
}

#[test]
fn class_property_vector() {
    let prop = MetadataClassProperty::new_vector(
        "position",
        MetadataType::Float64,
        MetadataComponentType::Vec3,
    );
    assert_eq!(prop.value_type, MetadataType::Float64);
    assert_eq!(prop.component_type, MetadataComponentType::Vec3);
    assert!(!prop.array);
}

#[test]
fn class_property_with_no_data_and_default() {
    let mut prop = MetadataClassProperty::new_scalar("temperature", MetadataType::Float32);
    prop.no_data = Some(MetadataValue::Float(-9999.0));
    prop.default = Some(MetadataValue::Float(0.0));
    prop.required = true;

    assert_eq!(prop.no_data, Some(MetadataValue::Float(-9999.0)));
    assert_eq!(prop.default, Some(MetadataValue::Float(0.0)));
    assert!(prop.required);
}

#[test]
fn class_property_normalized_with_offset_scale() {
    let mut prop = MetadataClassProperty::new_scalar("color_channel", MetadataType::Uint8);
    prop.normalized = true;
    prop.offset = Some(MetadataValue::Float(0.0));
    prop.scale = Some(MetadataValue::Float(1.0 / 255.0));

    assert!(prop.normalized);
    assert!(prop.offset.is_some());
    assert!(prop.scale.is_some());
}

#[test]
fn class_property_array() {
    let mut prop = MetadataClassProperty::new_scalar("tags", MetadataType::String);
    prop.array = true;
    assert!(prop.array);
}

#[test]
fn class_property_enum() {
    let mut prop = MetadataClassProperty::new_scalar("category", MetadataType::Enum);
    prop.enum_id = Some("building_type".to_string());
    assert_eq!(prop.enum_id, Some("building_type".to_string()));
    assert_eq!(prop.value_type, MetadataType::Enum);
}

#[test]
fn class_property_min_max() {
    let mut prop = MetadataClassProperty::new_scalar("altitude", MetadataType::Float64);
    prop.min = Some(MetadataValue::Float(0.0));
    prop.max = Some(MetadataValue::Float(8848.0));
    assert_eq!(prop.min, Some(MetadataValue::Float(0.0)));
    assert_eq!(prop.max, Some(MetadataValue::Float(8848.0)));
}

// ═══════════════════════════════════════════════════════════════════════════════
// MetadataClass extended
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn metadata_class_add_multiple_properties() {
    let mut class = MetadataClass::new("building");
    class.add_property(MetadataClassProperty::new_scalar("height", MetadataType::Float32));
    class.add_property(MetadataClassProperty::new_scalar("floors", MetadataType::Int32));
    class.add_property(MetadataClassProperty::new_vector(
        "center",
        MetadataType::Float64,
        MetadataComponentType::Vec3,
    ));

    assert_eq!(class.properties.len(), 3);
    assert!(class.get_property("height").is_some());
    assert!(class.get_property("floors").is_some());
    assert!(class.get_property("center").is_some());
    assert!(class.get_property("nonexistent").is_none());
}

#[test]
fn metadata_class_name_description() {
    let mut class = MetadataClass::new("road");
    class.name = Some("Road Feature".to_string());
    class.description = Some("A road segment".to_string());

    assert_eq!(class.name, Some("Road Feature".to_string()));
    assert_eq!(class.description, Some("A road segment".to_string()));
}

// ═══════════════════════════════════════════════════════════════════════════════
// MetadataEnum extended
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn metadata_enum_with_values() {
    let mut e = MetadataEnum::new("land_use", MetadataType::Uint8);
    e.add_value("residential", 0);
    e.add_value("commercial", 1);
    e.add_value("industrial", 2);
    e.add_value("park", 3);

    assert_eq!(e.values.len(), 4);
    assert_eq!(e.values.get("residential"), Some(&0));
    assert_eq!(e.values.get("park"), Some(&3));
}

#[test]
fn metadata_enum_name_for_value() {
    let mut e = MetadataEnum::new("color", MetadataType::Uint8);
    e.add_value("red", 0);
    e.add_value("green", 1);
    e.add_value("blue", 2);

    assert_eq!(e.name_for_value(0), Some("red"));
    assert_eq!(e.name_for_value(1), Some("green"));
    assert_eq!(e.name_for_value(2), Some("blue"));
    assert_eq!(e.name_for_value(99), None);
}

#[test]
fn metadata_enum_int16_type() {
    let e = MetadataEnum::new("large_enum", MetadataType::Int16);
    assert_eq!(e.value_type, MetadataType::Int16);
}

// ═══════════════════════════════════════════════════════════════════════════════
// PropertyTable extended
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn property_table_set_get_values() {
    let class = MetadataClass::new("buildings");
    let mut table = PropertyTable::new(3, class);

    table.set_value("height", 0, MetadataValue::Float(10.5));
    table.set_value("height", 1, MetadataValue::Float(20.0));
    table.set_value("height", 2, MetadataValue::Float(15.3));

    assert_eq!(
        table.get_value("height", 0),
        Some(&MetadataValue::Float(10.5))
    );
    assert_eq!(
        table.get_value("height", 1),
        Some(&MetadataValue::Float(20.0))
    );
    assert_eq!(
        table.get_value("height", 2),
        Some(&MetadataValue::Float(15.3))
    );
}

#[test]
fn property_table_multiple_properties() {
    let class = MetadataClass::new("features");
    let mut table = PropertyTable::new(2, class);

    table.set_value("name", 0, MetadataValue::String("Building A".into()));
    table.set_value("name", 1, MetadataValue::String("Building B".into()));
    table.set_value("height", 0, MetadataValue::Float(30.0));
    table.set_value("height", 1, MetadataValue::Float(45.0));

    assert_eq!(table.property_count(), 2);
    assert_eq!(
        table.get_value("name", 0),
        Some(&MetadataValue::String("Building A".into()))
    );
}

#[test]
fn property_table_out_of_bounds() {
    let class = MetadataClass::new("test");
    let mut table = PropertyTable::new(2, class);
    table.set_value("prop", 0, MetadataValue::Int(42));

    // Out of bounds feature index
    assert_eq!(table.get_value("prop", 99), None);
    // Non-existent property
    assert_eq!(table.get_value("nonexistent", 0), None);
}

#[test]
fn property_table_property_ids() {
    let class = MetadataClass::new("test");
    let mut table = PropertyTable::new(1, class);
    table.set_value("alpha", 0, MetadataValue::Int(1));
    table.set_value("beta", 0, MetadataValue::Int(2));

    let ids = table.property_ids();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&"alpha"));
    assert!(ids.contains(&"beta"));
}

#[test]
fn property_table_name_and_id() {
    let class = MetadataClass::new("test");
    let mut table = PropertyTable::new(5, class);
    table.name = Some("My Table".to_string());
    table.id = Some("table_0".to_string());

    assert_eq!(table.name, Some("My Table".to_string()));
    assert_eq!(table.id, Some("table_0".to_string()));
    assert_eq!(table.count, 5);
}

#[test]
fn property_table_array_values() {
    let class = MetadataClass::new("test");
    let mut table = PropertyTable::new(1, class);
    table.set_value(
        "colors",
        0,
        MetadataValue::Array(vec![
            MetadataValue::Uint(255),
            MetadataValue::Uint(128),
            MetadataValue::Uint(0),
        ]),
    );

    if let Some(MetadataValue::Array(items)) = table.get_value("colors", 0) {
        assert_eq!(items.len(), 3);
    } else {
        panic!("Expected Array value");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// PropertyTexture extended
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn property_texture_with_properties() {
    let class = MetadataClass::new("texture_class");
    let mut texture = PropertyTexture::new(class);
    texture.name = Some("Height Texture".to_string());

    texture.properties.insert(
        "height".to_string(),
        cesium_tileset::structural_metadata::PropertyTextureProperty {
            texture_index: 0,
            tex_coord: 0,
            channels: vec![0, 1],
        },
    );

    assert!(texture.get_property("height").is_some());
    assert!(texture.get_property("nonexistent").is_none());
    let prop = texture.get_property("height").unwrap();
    assert_eq!(prop.texture_index, 0);
    assert_eq!(prop.channels, vec![0, 1]);
}

// ═══════════════════════════════════════════════════════════════════════════════
// PropertyAttribute extended
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn property_attribute_with_properties() {
    let class = MetadataClass::new("attr_class");
    let mut attr = PropertyAttribute::new(class);
    attr.name = Some("Vertex Colors".to_string());

    attr.properties.insert(
        "color".to_string(),
        cesium_tileset::structural_metadata::PropertyAttributeProperty {
            attribute: "_COLOR".to_string(),
        },
    );

    assert!(attr.get_property("color").is_some());
    let prop = attr.get_property("color").unwrap();
    assert_eq!(prop.attribute, "_COLOR");
}

// ═══════════════════════════════════════════════════════════════════════════════
// StructuralMetadata extended
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn structural_metadata_multiple_tables() {
    let mut sm = StructuralMetadata::new();

    let class1 = MetadataClass::new("buildings");
    sm.classes.insert("buildings".to_string(), class1.clone());
    sm.add_property_table(PropertyTable::new(10, class1));

    let class2 = MetadataClass::new("roads");
    sm.classes.insert("roads".to_string(), class2.clone());
    sm.add_property_table(PropertyTable::new(25, class2));

    assert_eq!(sm.property_tables.len(), 2);
    assert_eq!(sm.get_property_table(0).unwrap().count, 10);
    assert_eq!(sm.get_property_table(1).unwrap().count, 25);
    assert!(sm.get_property_table(2).is_none());
}

#[test]
fn structural_metadata_get_class() {
    let mut sm = StructuralMetadata::new();
    let class = MetadataClass::new("bridge");
    sm.classes.insert("bridge".to_string(), class);

    assert!(sm.get_class("bridge").is_some());
    assert!(sm.get_class("tunnel").is_none());
}

#[test]
fn structural_metadata_enums() {
    let mut sm = StructuralMetadata::new();
    let mut e = MetadataEnum::new("material_type", MetadataType::Uint8);
    e.add_value("concrete", 0);
    e.add_value("steel", 1);
    e.add_value("wood", 2);
    sm.enums.insert("material_type".to_string(), e);

    assert!(sm.enums.contains_key("material_type"));
    let stored = &sm.enums["material_type"];
    assert_eq!(stored.values.len(), 3);
}

// ═══════════════════════════════════════════════════════════════════════════════
// MetadataValue extended
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn metadata_value_as_str() {
    let v = MetadataValue::String("hello".to_string());
    assert_eq!(v.as_str(), Some("hello"));
    assert_eq!(MetadataValue::Int(42).as_str(), None);
}

#[test]
fn metadata_value_nested_array() {
    let v = MetadataValue::Array(vec![
        MetadataValue::Array(vec![MetadataValue::Int(1), MetadataValue::Int(2)]),
        MetadataValue::Array(vec![MetadataValue::Int(3), MetadataValue::Int(4)]),
    ]);
    if let MetadataValue::Array(rows) = v {
        assert_eq!(rows.len(), 2);
        if let MetadataValue::Array(row) = &rows[0] {
            assert_eq!(row.len(), 2);
        }
    }
}

#[test]
fn metadata_value_bool_as_f64() {
    assert_eq!(MetadataValue::Bool(true).as_f64(), Some(1.0));
    assert_eq!(MetadataValue::Bool(false).as_f64(), Some(0.0));
}

#[test]
fn metadata_value_uint_as_f64() {
    assert_eq!(MetadataValue::Uint(u64::MAX).as_f64(), Some(u64::MAX as f64));
}
