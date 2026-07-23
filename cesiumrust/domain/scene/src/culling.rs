//! Frustum culling and visibility determination.
//!
//! Maps to CesiumJS `Scene/Scene.js` culling logic and
//! `Core/CullingVolume.js`

use cesium_geospatial::bounding::BoundingSphere;
use cesium_geospatial::frustum::{CullingVolume, PerspectiveFrustum};
use cesium_geospatial::ray::Intersect;
use glam::DVec3;

use crate::scene_graph::{NodeId, SceneGraph, SceneNode};

/// Result of a culling test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CullResult {
    /// The object is completely outside the frustum.
    Outside,
    /// The object intersects the frustum boundary.
    Intersecting,
    /// The object is completely inside the frustum.
    Inside,
}

impl CullResult {
    /// Returns true if the object is at least partially visible.
    pub fn is_visible(&self) -> bool {
        !matches!(self, CullResult::Outside)
    }
}

/// Frustum culling context for a frame.
#[derive(Debug, Clone)]
pub struct CullingContext {
    /// The culling volume (6 planes).
    pub culling_volume: CullingVolume,

    /// Camera position for distance calculations.
    pub camera_position: DVec3,

    /// Whether culling is enabled.
    pub enabled: bool,
}

impl CullingContext {
    /// Creates a culling context from a perspective frustum.
    pub fn from_perspective_frustum(
        frustum: &PerspectiveFrustum,
        position: DVec3,
        direction: DVec3,
        up: DVec3,
    ) -> Self {
        let culling_volume = frustum.compute_culling_volume(position, direction, up);
        Self {
            culling_volume,
            camera_position: position,
            enabled: true,
        }
    }

    /// Tests a bounding sphere against the frustum.
    pub fn test_bounding_sphere(&self, sphere: &BoundingSphere) -> CullResult {
        if !self.enabled {
            return CullResult::Inside;
        }
        match self.culling_volume.visibility(sphere) {
            Intersect::Outside => CullResult::Outside,
            Intersect::Intersecting => CullResult::Intersecting,
            Intersect::Inside => CullResult::Inside,
        }
    }

    /// Computes the distance from the camera to a bounding sphere.
    pub fn distance_to(&self, sphere: &BoundingSphere) -> f64 {
        let dist = self.camera_position.distance(sphere.center) - sphere.radius;
        dist.max(0.0)
    }
}

/// Result of visibility determination for a node.
#[derive(Debug, Clone)]
pub struct VisibilityResult {
    /// The node ID.
    pub node_id: NodeId,

    /// Whether the node is visible.
    pub visible: bool,

    /// Distance from camera (for sorting).
    pub distance: f64,

    /// The cull result.
    pub cull_result: CullResult,
}

/// Performs frustum culling on the scene graph.
///
/// Returns a list of visible node IDs with their distances.
pub fn cull_scene(
    scene: &SceneGraph,
    context: &CullingContext,
) -> Vec<VisibilityResult> {
    let mut results = Vec::new();

    scene.traverse(|node| {
        let result = cull_node(node, context);
        results.push(result);
    });

    results
}

/// Performs culling test on a single node.
fn cull_node(node: &SceneNode, context: &CullingContext) -> VisibilityResult {
    // If node has no bounding volume, assume visible
    let world_bv = match node.world_bounding_sphere() {
        Some(bv) => bv,
        None => {
            return VisibilityResult {
                node_id: node.id,
                visible: true,
                distance: 0.0,
                cull_result: CullResult::Inside,
            };
        }
    };

    let cull_result = context.test_bounding_sphere(&world_bv);
    let distance = context.distance_to(&world_bv);

    VisibilityResult {
        node_id: node.id,
        visible: cull_result.is_visible(),
        distance,
        cull_result,
    }
}

/// Sorts visibility results by distance (front-to-back).
pub fn sort_front_to_back(results: &mut [VisibilityResult]) {
    results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));
}

/// Sorts visibility results by distance (back-to-front) for transparency.
pub fn sort_back_to_front(results: &mut [VisibilityResult]) {
    results.sort_by(|a, b| b.distance.partial_cmp(&a.distance).unwrap_or(std::cmp::Ordering::Equal));
}

/// Filters results to only visible nodes.
pub fn filter_visible(results: Vec<VisibilityResult>) -> Vec<VisibilityResult> {
    results.into_iter().filter(|r| r.visible).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene_graph::SceneNode;
    use std::f64::consts::FRAC_PI_4;

    fn create_test_frustum() -> PerspectiveFrustum {
        PerspectiveFrustum::new(FRAC_PI_4, 16.0 / 9.0, 0.1, 10000.0)
    }

    fn create_test_context() -> CullingContext {
        let frustum = create_test_frustum();
        CullingContext::from_perspective_frustum(
            &frustum,
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(0.0, 0.0, -1.0),
            DVec3::new(0.0, 1.0, 0.0),
        )
    }

    #[test]
    fn test_cull_result_visibility() {
        assert!(CullResult::Inside.is_visible());
        assert!(CullResult::Intersecting.is_visible());
        assert!(!CullResult::Outside.is_visible());
    }

    #[test]
    fn test_sphere_in_frustum() {
        let context = create_test_context();

        // Sphere in front of camera
        let sphere = BoundingSphere::new(DVec3::new(0.0, 0.0, -100.0), 10.0);
        let result = context.test_bounding_sphere(&sphere);
        assert!(result.is_visible());
    }

    #[test]
    fn test_sphere_behind_camera() {
        let context = create_test_context();

        // Sphere behind camera
        let sphere = BoundingSphere::new(DVec3::new(0.0, 0.0, 100.0), 10.0);
        let result = context.test_bounding_sphere(&sphere);
        assert!(!result.is_visible());
    }

    #[test]
    fn test_distance_calculation() {
        let context = create_test_context();

        let sphere = BoundingSphere::new(DVec3::new(0.0, 0.0, -100.0), 10.0);
        let distance = context.distance_to(&sphere);
        assert!((distance - 90.0).abs() < 1e-10); // 100 - 10 = 90
    }

    #[test]
    fn test_cull_scene() {
        let mut scene = SceneGraph::new();

        // Visible node in front
        let visible_node = SceneNode::new(0)
            .with_bounding_volume(BoundingSphere::new(DVec3::new(0.0, 0.0, -100.0), 10.0));
        scene.add_node(visible_node);

        // Hidden node behind
        let hidden_node = SceneNode::new(0)
            .with_bounding_volume(BoundingSphere::new(DVec3::new(0.0, 0.0, 100.0), 10.0));
        scene.add_node(hidden_node);

        scene.update_world_transforms();

        let context = create_test_context();
        let results = cull_scene(&scene, &context);

        assert_eq!(results.len(), 2);

        let visible_results = filter_visible(results);
        assert_eq!(visible_results.len(), 1);
    }

    #[test]
    fn test_sort_front_to_back() {
        let mut results = vec![
            VisibilityResult {
                node_id: 1,
                visible: true,
                distance: 100.0,
                cull_result: CullResult::Inside,
            },
            VisibilityResult {
                node_id: 2,
                visible: true,
                distance: 50.0,
                cull_result: CullResult::Inside,
            },
            VisibilityResult {
                node_id: 3,
                visible: true,
                distance: 200.0,
                cull_result: CullResult::Inside,
            },
        ];

        sort_front_to_back(&mut results);

        assert_eq!(results[0].node_id, 2);
        assert_eq!(results[1].node_id, 1);
        assert_eq!(results[2].node_id, 3);
    }

    #[test]
    fn test_sort_back_to_front() {
        let mut results = vec![
            VisibilityResult {
                node_id: 1,
                visible: true,
                distance: 100.0,
                cull_result: CullResult::Inside,
            },
            VisibilityResult {
                node_id: 2,
                visible: true,
                distance: 50.0,
                cull_result: CullResult::Inside,
            },
        ];

        sort_back_to_front(&mut results);

        assert_eq!(results[0].node_id, 1);
        assert_eq!(results[1].node_id, 2);
    }

    #[test]
    fn test_culling_disabled() {
        let mut context = create_test_context();
        context.enabled = false;

        // Even a sphere behind camera should be "visible" when culling is disabled
        let sphere = BoundingSphere::new(DVec3::new(0.0, 0.0, 100.0), 10.0);
        let result = context.test_bounding_sphere(&sphere);
        assert!(result.is_visible());
    }
}
