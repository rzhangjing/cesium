//! Port of `Core/GeometryInstanceSpec.js`.
use std::collections::HashMap;

use cesium_core::component_datatype::ComponentDatatype;
use cesium_core::geometry_instance::{GeometryInstance, GeometryInstanceGeometry};
use cesium_core::geometry_instance_attribute::GeometryInstanceAttribute;
use cesium_core::matrix4::Matrix4;

#[test]
fn constructor() {
    let attrs = {
        let mut m = HashMap::new();
        m.insert(
            "color".to_string(),
            GeometryInstanceAttribute::new(
                ComponentDatatype::UnsignedByte,
                4,
                Some(true),
                vec![255.0, 255.0, 0.0, 255.0],
            ),
        );
        m
    };

    let instance = GeometryInstance::new(
        GeometryInstanceGeometry::Placeholder,
        Some(Matrix4::IDENTITY.clone()),
        Some("geometry".to_string()),
        Some(attrs),
    );

    assert_eq!(instance.id, Some("geometry".to_string()));
    assert_eq!(instance.model_matrix, Matrix4::IDENTITY);
    assert!(instance.attributes.contains_key("color"));
}

#[test]
fn constructor_defaults() {
    let instance = GeometryInstance::new(
        GeometryInstanceGeometry::Placeholder,
        None,
        None,
        None,
    );
    assert_eq!(instance.id, None);
    assert_eq!(instance.model_matrix, Matrix4::IDENTITY);
    assert!(instance.attributes.is_empty());
}
