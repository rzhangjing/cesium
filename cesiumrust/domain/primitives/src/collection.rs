//! Primitive collection and batching.
//!
//! Maps to CesiumJS:
//! - `Scene/Primitive.js`
//! - `Scene/PrimitiveCollection.js`
//! - Geometry batching for performance

use crate::geometry_instance::{Appearance, GeometryInstance};
use cesium_geospatial::bounding::BoundingSphere;
use glam::DVec3;

/// A primitive that renders geometry instances with an appearance.
///
/// Maps to CesiumJS `Scene/Primitive.js`
#[derive(Debug, Clone)]
pub struct Primitive {
    /// Unique identifier.
    pub id: String,
    /// Geometry instances to render.
    pub instances: Vec<GeometryInstance>,
    /// Appearance for rendering.
    pub appearance: Appearance,
    /// Whether the primitive is shown.
    pub show: bool,
    /// Whether to cull back faces.
    pub cull: bool,
    /// Whether to compress vertices for performance.
    pub compress_vertices: bool,
    /// Computed bounding sphere.
    pub bounding_sphere: Option<BoundingSphere>,
}

impl Primitive {
    /// Creates a new primitive.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            instances: Vec::new(),
            appearance: Appearance::default(),
            show: true,
            cull: true,
            compress_vertices: true,
            bounding_sphere: None,
        }
    }

    /// Adds a geometry instance.
    pub fn add_instance(&mut self, instance: GeometryInstance) {
        self.instances.push(instance);
        self.bounding_sphere = None; // Invalidate
    }

    /// Sets the appearance.
    pub fn with_appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// Computes the combined bounding sphere.
    pub fn compute_bounding_sphere(&mut self) {
        if self.instances.is_empty() {
            self.bounding_sphere = None;
            return;
        }

        // Compute bounding spheres for all instances
        let spheres: Vec<BoundingSphere> = self
            .instances
            .iter()
            .map(|inst| {
                let local_bs = inst.geometry_type.bounding_sphere();
                local_bs.transform(&inst.model_matrix)
            })
            .collect();

        // Compute union
        self.bounding_sphere = Some(compute_bounding_sphere_union(&spheres));
    }

    /// Returns the total vertex count estimate.
    pub fn total_vertex_count(&self) -> u32 {
        self.instances
            .iter()
            .map(|inst| inst.geometry_type.estimated_vertex_count())
            .sum()
    }
}

/// A collection of primitives.
///
/// Maps to CesiumJS `Scene/PrimitiveCollection.js`
#[derive(Debug, Default)]
pub struct PrimitiveCollection {
    /// Primitives in the collection.
    primitives: Vec<Primitive>,
    /// Whether the collection is shown.
    pub show: bool,
}

impl PrimitiveCollection {
    /// Creates a new primitive collection.
    pub fn new() -> Self {
        Self {
            primitives: Vec::new(),
            show: true,
        }
    }

    /// Adds a primitive to the collection.
    pub fn add(&mut self, primitive: Primitive) {
        self.primitives.push(primitive);
    }

    /// Removes a primitive by ID.
    pub fn remove(&mut self, id: &str) -> Option<Primitive> {
        if let Some(idx) = self.primitives.iter().position(|p| p.id == id) {
            Some(self.primitives.remove(idx))
        } else {
            None
        }
    }

    /// Gets a primitive by ID.
    pub fn get(&self, id: &str) -> Option<&Primitive> {
        self.primitives.iter().find(|p| p.id == id)
    }

    /// Gets a mutable primitive by ID.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Primitive> {
        self.primitives.iter_mut().find(|p| p.id == id)
    }

    /// Returns the number of primitives.
    pub fn len(&self) -> usize {
        self.primitives.len()
    }

    /// Returns true if the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.primitives.is_empty()
    }

    /// Returns an iterator over primitives.
    pub fn iter(&self) -> impl Iterator<Item = &Primitive> {
        self.primitives.iter()
    }

    /// Returns a mutable iterator over primitives.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Primitive> {
        self.primitives.iter_mut()
    }

    /// Computes the combined bounding sphere.
    pub fn compute_bounding_sphere(&self) -> Option<BoundingSphere> {
        let spheres: Vec<BoundingSphere> = self
            .primitives
            .iter()
            .filter(|p| p.show)
            .filter_map(|p| {
                let spheres: Vec<BoundingSphere> = p
                    .instances
                    .iter()
                    .map(|inst| {
                        let local_bs = inst.geometry_type.bounding_sphere();
                        local_bs.transform(&inst.model_matrix)
                    })
                    .collect();
                if spheres.is_empty() {
                    None
                } else {
                    Some(compute_bounding_sphere_union(&spheres))
                }
            })
            .collect();

        if spheres.is_empty() {
            None
        } else {
            Some(compute_bounding_sphere_union(&spheres))
        }
    }

    /// Returns visible primitives.
    pub fn visible_primitives(&self) -> impl Iterator<Item = &Primitive> {
        self.primitives.iter().filter(|p| p.show && self.show)
    }
}

/// Computes the union of multiple bounding spheres.
pub fn compute_bounding_sphere_union(spheres: &[BoundingSphere]) -> BoundingSphere {
    if spheres.is_empty() {
        return BoundingSphere::new(DVec3::ZERO, 0.0);
    }

    if spheres.len() == 1 {
        return spheres[0];
    }

    // Compute centroid
    let mut center = DVec3::ZERO;
    for sphere in spheres {
        center += sphere.center;
    }
    center /= spheres.len() as f64;

    // Compute max distance from centroid
    let mut max_radius = 0.0f64;
    for sphere in spheres {
        let dist = (sphere.center - center).length() + sphere.radius;
        max_radius = max_radius.max(dist);
    }

    BoundingSphere::new(center, max_radius)
}

/// Batch configuration for geometry merging.
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Maximum instances per batch.
    pub max_instances_per_batch: usize,
    /// Whether to merge geometries with the same material.
    pub merge_by_material: bool,
    /// Whether to sort by distance for transparency.
    pub sort_by_distance: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_instances_per_batch: 1000,
            merge_by_material: true,
            sort_by_distance: false,
        }
    }
}

/// A batch of geometry instances for efficient rendering.
#[derive(Debug, Clone)]
pub struct GeometryBatch {
    /// Batch ID.
    pub id: u32,
    /// Instances in this batch.
    pub instances: Vec<GeometryInstance>,
    /// Shared appearance.
    pub appearance: Appearance,
    /// Combined bounding sphere.
    pub bounding_sphere: Option<BoundingSphere>,
}

impl GeometryBatch {
    /// Creates a new batch.
    pub fn new(id: u32, appearance: Appearance) -> Self {
        Self {
            id,
            instances: Vec::new(),
            appearance,
            bounding_sphere: None,
        }
    }

    /// Adds an instance to the batch.
    pub fn add(&mut self, instance: GeometryInstance) {
        self.instances.push(instance);
        self.bounding_sphere = None; // Invalidate
    }

    /// Returns true if the batch is full.
    pub fn is_full(&self, config: &BatchConfig) -> bool {
        self.instances.len() >= config.max_instances_per_batch
    }

    /// Computes the batch bounding sphere.
    pub fn compute_bounding_sphere(&mut self) {
        let spheres: Vec<BoundingSphere> = self
            .instances
            .iter()
            .map(|inst| {
                let local_bs = inst.geometry_type.bounding_sphere();
                local_bs.transform(&inst.model_matrix)
            })
            .collect();

        self.bounding_sphere = if spheres.is_empty() {
            None
        } else {
            Some(compute_bounding_sphere_union(&spheres))
        };
    }
}

/// Batches geometry instances for efficient rendering.
pub fn batch_instances(
    instances: Vec<GeometryInstance>,
    appearance: Appearance,
    config: &BatchConfig,
) -> Vec<GeometryBatch> {
    let mut batches = Vec::new();
    let mut current_batch = GeometryBatch::new(0, appearance.clone());

    for instance in instances {
        if current_batch.is_full(config) {
            current_batch.compute_bounding_sphere();
            batches.push(current_batch);
            current_batch = GeometryBatch::new(batches.len() as u32, appearance.clone());
        }
        current_batch.add(instance);
    }

    if !current_batch.instances.is_empty() {
        current_batch.compute_bounding_sphere();
        batches.push(current_batch);
    }

    batches
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry_instance::GeometryType;

    #[test]
    fn test_primitive_creation() {
        let primitive = Primitive::new("test");
        assert_eq!(primitive.id, "test");
        assert!(primitive.show);
        assert!(primitive.instances.is_empty());
    }

    #[test]
    fn test_primitive_add_instance() {
        let mut primitive = Primitive::new("test");
        primitive.add_instance(GeometryInstance::new("inst1", GeometryType::Sphere { radius: 10.0 }));
        assert_eq!(primitive.instances.len(), 1);
    }

    #[test]
    fn test_primitive_bounding_sphere() {
        let mut primitive = Primitive::new("test");
        primitive.add_instance(
            GeometryInstance::new("inst1", GeometryType::Sphere { radius: 10.0 })
                .with_position(DVec3::new(100.0, 0.0, 0.0)),
        );
        primitive.add_instance(
            GeometryInstance::new("inst2", GeometryType::Sphere { radius: 10.0 })
                .with_position(DVec3::new(-100.0, 0.0, 0.0)),
        );

        primitive.compute_bounding_sphere();
        let bs = primitive.bounding_sphere.unwrap();

        // Center should be at origin, radius should cover both spheres
        assert!(bs.center.length() < 1e-10);
        assert!(bs.radius >= 110.0);
    }

    #[test]
    fn test_primitive_total_vertex_count() {
        let mut primitive = Primitive::new("test");
        primitive.add_instance(GeometryInstance::new("box", GeometryType::Box { half_extents: DVec3::ONE }));
        primitive.add_instance(GeometryInstance::new("sphere", GeometryType::Sphere { radius: 1.0 }));

        assert_eq!(primitive.total_vertex_count(), 24 + 1024);
    }

    #[test]
    fn test_primitive_collection() {
        let mut collection = PrimitiveCollection::new();
        assert!(collection.is_empty());

        collection.add(Primitive::new("p1"));
        collection.add(Primitive::new("p2"));

        assert_eq!(collection.len(), 2);
        assert!(!collection.is_empty());
    }

    #[test]
    fn test_primitive_collection_get() {
        let mut collection = PrimitiveCollection::new();
        collection.add(Primitive::new("p1"));
        collection.add(Primitive::new("p2"));

        assert!(collection.get("p1").is_some());
        assert!(collection.get("p3").is_none());
    }

    #[test]
    fn test_primitive_collection_remove() {
        let mut collection = PrimitiveCollection::new();
        collection.add(Primitive::new("p1"));
        collection.add(Primitive::new("p2"));

        let removed = collection.remove("p1");
        assert!(removed.is_some());
        assert_eq!(collection.len(), 1);
    }

    #[test]
    fn test_primitive_collection_visible() {
        let mut collection = PrimitiveCollection::new();
        let mut p1 = Primitive::new("p1");
        p1.show = true;
        let mut p2 = Primitive::new("p2");
        p2.show = false;

        collection.add(p1);
        collection.add(p2);

        let visible: Vec<_> = collection.visible_primitives().collect();
        assert_eq!(visible.len(), 1);
    }

    #[test]
    fn test_bounding_sphere_union() {
        let spheres = vec![
            BoundingSphere::new(DVec3::new(0.0, 0.0, 0.0), 10.0),
            BoundingSphere::new(DVec3::new(100.0, 0.0, 0.0), 10.0),
        ];

        let union = compute_bounding_sphere_union(&spheres);
        assert!(union.center.x > 40.0 && union.center.x < 60.0);
        assert!(union.radius >= 60.0);
    }

    #[test]
    fn test_bounding_sphere_union_empty() {
        let union = compute_bounding_sphere_union(&[]);
        assert_eq!(union.radius, 0.0);
    }

    #[test]
    fn test_bounding_sphere_union_single() {
        let spheres = vec![BoundingSphere::new(DVec3::new(10.0, 20.0, 30.0), 50.0)];
        let union = compute_bounding_sphere_union(&spheres);
        assert_eq!(union.center, DVec3::new(10.0, 20.0, 30.0));
        assert_eq!(union.radius, 50.0);
    }

    #[test]
    fn test_batch_config_default() {
        let config = BatchConfig::default();
        assert_eq!(config.max_instances_per_batch, 1000);
        assert!(config.merge_by_material);
        assert!(!config.sort_by_distance);
    }

    #[test]
    fn test_geometry_batch() {
        let mut batch = GeometryBatch::new(0, Appearance::default());
        batch.add(GeometryInstance::new("inst1", GeometryType::Sphere { radius: 10.0 }));
        batch.add(GeometryInstance::new("inst2", GeometryType::Sphere { radius: 20.0 }));

        assert_eq!(batch.instances.len(), 2);

        batch.compute_bounding_sphere();
        assert!(batch.bounding_sphere.is_some());
    }

    #[test]
    fn test_batch_instances() {
        let instances: Vec<GeometryInstance> = (0..5)
            .map(|i| GeometryInstance::new(format!("inst{}", i), GeometryType::Sphere { radius: 10.0 }))
            .collect();

        let config = BatchConfig {
            max_instances_per_batch: 2,
            ..Default::default()
        };

        let batches = batch_instances(instances, Appearance::default(), &config);

        // 5 instances / 2 per batch = 3 batches (2, 2, 1)
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0].instances.len(), 2);
        assert_eq!(batches[1].instances.len(), 2);
        assert_eq!(batches[2].instances.len(), 1);
    }

    #[test]
    fn test_batch_is_full() {
        let config = BatchConfig {
            max_instances_per_batch: 2,
            ..Default::default()
        };

        let mut batch = GeometryBatch::new(0, Appearance::default());
        assert!(!batch.is_full(&config));

        batch.add(GeometryInstance::new("inst1", GeometryType::Sphere { radius: 10.0 }));
        assert!(!batch.is_full(&config));

        batch.add(GeometryInstance::new("inst2", GeometryType::Sphere { radius: 10.0 }));
        assert!(batch.is_full(&config));
    }
}
