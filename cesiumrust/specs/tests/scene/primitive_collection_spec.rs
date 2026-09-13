//! PrimitiveCollection + GeometryInstance + GeometryBatch specs
//! Ported from CesiumJS Scene/PrimitiveSpec.js + PrimitiveCollectionSpec.js

use cesium_primitives::{
    batch_instances, compute_bounding_sphere_union, Appearance, BatchConfig, GeometryBatch,
    GeometryInstance, GeometryType, Primitive, PrimitiveCollection,
};
use cesium_geospatial::bounding::BoundingSphere;
use glam::{DMat4, DVec3};

// ==================== GeometryInstance ====================

#[test]
fn geometry_instance_new_defaults() {
    let inst = GeometryInstance::new("test", GeometryType::Sphere { radius: 10.0 });
    assert_eq!(inst.id, "test");
    assert!(inst.show);
    assert_eq!(inst.color, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(inst.model_matrix, DMat4::IDENTITY);
    assert!(inst.bounding_sphere.is_none());
}

#[test]
fn geometry_instance_with_position() {
    let inst = GeometryInstance::new("s", GeometryType::Sphere { radius: 5.0 })
        .with_position(DVec3::new(100.0, 200.0, 300.0));
    let translation = inst.model_matrix.w_axis.truncate();
    assert_eq!(translation, DVec3::new(100.0, 200.0, 300.0));
}

#[test]
fn geometry_instance_with_color() {
    let inst = GeometryInstance::new("c", GeometryType::Sphere { radius: 1.0 })
        .with_color([1.0, 0.0, 0.0, 0.5]);
    assert_eq!(inst.color, [1.0, 0.0, 0.0, 0.5]);
}

#[test]
fn geometry_instance_compute_bounding_sphere() {
    let mut inst = GeometryInstance::new("bs", GeometryType::Sphere { radius: 10.0 })
        .with_position(DVec3::new(50.0, 0.0, 0.0));
    inst.compute_bounding_sphere();
    let bs = inst.bounding_sphere.unwrap();
    assert!((bs.center.x - 50.0).abs() < 1e-10);
    assert!((bs.radius - 10.0).abs() < 1e-10);
}

// ==================== GeometryType bounding spheres ====================

#[test]
fn geometry_type_sphere_bounding_sphere() {
    let gt = GeometryType::Sphere { radius: 25.0 };
    let bs = gt.bounding_sphere();
    assert_eq!(bs.center, DVec3::ZERO);
    assert!((bs.radius - 25.0).abs() < 1e-10);
}

#[test]
fn geometry_type_box_bounding_sphere() {
    let gt = GeometryType::Box {
        half_extents: DVec3::new(3.0, 4.0, 0.0),
    };
    let bs = gt.bounding_sphere();
    assert!((bs.radius - 5.0).abs() < 1e-10); // sqrt(9+16) = 5
}

#[test]
fn geometry_type_cylinder_bounding_sphere() {
    let gt = GeometryType::Cylinder {
        top_radius: 3.0,
        bottom_radius: 4.0,
        height: 6.0,
    };
    let bs = gt.bounding_sphere();
    // max_radius=4, half_height=3, radius=sqrt(16+9)=5
    assert!((bs.radius - 5.0).abs() < 1e-10);
}

// ==================== Primitive ====================

#[test]
fn primitive_new_defaults() {
    let p = Primitive::new("prim1");
    assert_eq!(p.id, "prim1");
    assert!(p.show);
    assert!(p.cull);
    assert!(p.compress_vertices);
    assert!(p.instances.is_empty());
    assert!(p.bounding_sphere.is_none());
}

#[test]
fn primitive_add_instance_invalidates_bs() {
    let mut p = Primitive::new("test");
    p.bounding_sphere = Some(BoundingSphere::new(DVec3::ZERO, 1.0));
    p.add_instance(GeometryInstance::new("i1", GeometryType::Sphere { radius: 5.0 }));
    assert!(p.bounding_sphere.is_none()); // Invalidated
}

#[test]
fn primitive_total_vertex_count() {
    let mut p = Primitive::new("test");
    p.add_instance(GeometryInstance::new("i1", GeometryType::Sphere { radius: 5.0 }));
    p.add_instance(GeometryInstance::new("i2", GeometryType::Sphere { radius: 10.0 }));
    let count = p.total_vertex_count();
    assert!(count > 0);
}

#[test]
fn primitive_compute_bounding_sphere() {
    let mut p = Primitive::new("test");
    p.add_instance(
        GeometryInstance::new("i1", GeometryType::Sphere { radius: 10.0 })
            .with_position(DVec3::new(100.0, 0.0, 0.0)),
    );
    p.add_instance(
        GeometryInstance::new("i2", GeometryType::Sphere { radius: 10.0 })
            .with_position(DVec3::new(-100.0, 0.0, 0.0)),
    );
    p.compute_bounding_sphere();
    let bs = p.bounding_sphere.unwrap();
    // Union should encompass both spheres
    assert!(bs.radius >= 110.0);
}

// ==================== PrimitiveCollection ====================

#[test]
fn primitive_collection_add_remove() {
    let mut coll = PrimitiveCollection::new();
    assert!(coll.is_empty());

    coll.add(Primitive::new("p1"));
    coll.add(Primitive::new("p2"));
    assert_eq!(coll.len(), 2);

    let removed = coll.remove("p1");
    assert!(removed.is_some());
    assert_eq!(coll.len(), 1);
    assert!(coll.get("p1").is_none());
    assert!(coll.get("p2").is_some());
}

#[test]
fn primitive_collection_remove_nonexistent() {
    let mut coll = PrimitiveCollection::new();
    coll.add(Primitive::new("p1"));
    assert!(coll.remove("nonexistent").is_none());
    assert_eq!(coll.len(), 1);
}

#[test]
fn primitive_collection_get_mut() {
    let mut coll = PrimitiveCollection::new();
    coll.add(Primitive::new("p1"));
    if let Some(p) = coll.get_mut("p1") {
        p.show = false;
    }
    assert!(!coll.get("p1").unwrap().show);
}

#[test]
fn primitive_collection_visible_primitives() {
    let mut coll = PrimitiveCollection::new();
    let mut p1 = Primitive::new("visible");
    p1.show = true;
    let mut p2 = Primitive::new("hidden");
    p2.show = false;
    coll.add(p1);
    coll.add(p2);

    let visible: Vec<_> = coll.visible_primitives().collect();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].id, "visible");
}

// ==================== compute_bounding_sphere_union ====================

#[test]
fn bounding_sphere_union_empty() {
    let bs = compute_bounding_sphere_union(&[]);
    assert_eq!(bs.center, DVec3::ZERO);
    assert!((bs.radius - 0.0).abs() < 1e-10);
}

#[test]
fn bounding_sphere_union_single() {
    let spheres = vec![BoundingSphere::new(DVec3::new(1.0, 2.0, 3.0), 5.0)];
    let bs = compute_bounding_sphere_union(&spheres);
    assert_eq!(bs.center, DVec3::new(1.0, 2.0, 3.0));
    assert!((bs.radius - 5.0).abs() < 1e-10);
}

#[test]
fn bounding_sphere_union_two() {
    let spheres = vec![
        BoundingSphere::new(DVec3::new(-10.0, 0.0, 0.0), 5.0),
        BoundingSphere::new(DVec3::new(10.0, 0.0, 0.0), 5.0),
    ];
    let bs = compute_bounding_sphere_union(&spheres);
    // Centroid = (0,0,0), max_dist = 10 + 5 = 15
    assert!((bs.center.x - 0.0).abs() < 1e-10);
    assert!((bs.radius - 15.0).abs() < 1e-10);
}

// ==================== GeometryBatch + batch_instances ====================

#[test]
fn geometry_batch_new() {
    let batch = GeometryBatch::new(0, Appearance::default());
    assert_eq!(batch.id, 0);
    assert!(batch.instances.is_empty());
    assert!(batch.bounding_sphere.is_none());
}

#[test]
fn geometry_batch_is_full() {
    let config = BatchConfig {
        max_instances_per_batch: 2,
        ..Default::default()
    };
    let mut batch = GeometryBatch::new(0, Appearance::default());
    assert!(!batch.is_full(&config));
    batch.add(GeometryInstance::new("i1", GeometryType::Sphere { radius: 1.0 }));
    assert!(!batch.is_full(&config));
    batch.add(GeometryInstance::new("i2", GeometryType::Sphere { radius: 1.0 }));
    assert!(batch.is_full(&config));
}

#[test]
fn batch_instances_splits() {
    let config = BatchConfig {
        max_instances_per_batch: 3,
        ..Default::default()
    };
    let instances: Vec<GeometryInstance> = (0..7)
        .map(|i| GeometryInstance::new(format!("i{}", i), GeometryType::Sphere { radius: 1.0 }))
        .collect();

    let batches = batch_instances(instances, Appearance::default(), &config);
    assert_eq!(batches.len(), 3); // 3 + 3 + 1
    assert_eq!(batches[0].instances.len(), 3);
    assert_eq!(batches[1].instances.len(), 3);
    assert_eq!(batches[2].instances.len(), 1);
}

#[test]
fn batch_instances_empty() {
    let config = BatchConfig::default();
    let batches = batch_instances(vec![], Appearance::default(), &config);
    assert!(batches.is_empty());
}
