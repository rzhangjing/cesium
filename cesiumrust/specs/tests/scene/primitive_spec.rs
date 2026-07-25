//! Scene/PrimitiveSpec.js, PrimitiveCollectionSpec.js → Rust integration tests

use cesium_primitives::{PrimitiveCollection, GeometryType, CullMode};
use glam::DVec3;

// === GeometryType ===

#[test]
fn test_geometry_type_variants() {
    let _box = GeometryType::Box { half_extents: DVec3::new(1.0, 1.0, 1.0) };
    let _sphere = GeometryType::Sphere { radius: 1.0 };
    let _cylinder = GeometryType::Cylinder { top_radius: 0.5, bottom_radius: 1.0, height: 2.0 };
}

// === CullMode ===

#[test]
fn test_cull_mode_default() {
    let mode = CullMode::default();
    assert_eq!(mode, CullMode::Back);
}

// === PrimitiveCollection ===

#[test]
fn test_primitive_collection_new() {
    let collection = PrimitiveCollection::new();
    assert_eq!(collection.len(), 0);
    assert!(collection.is_empty());
}

#[test]
fn test_primitive_collection_show() {
    let mut collection = PrimitiveCollection::new();
    assert!(collection.show);
    collection.show = false;
    assert!(!collection.show);
}
