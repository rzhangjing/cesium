//! Scene culling + scene graph specs
//! Ported from CesiumJS Scene/SceneSpec.js culling logic

use cesium_scene::{
    cull_scene, filter_visible, sort_back_to_front, sort_front_to_back,
    CullResult, CullingContext, NodeId, RenderableContent, SceneGraph, SceneNode,
    VisibilityResult,
};
use cesium_geospatial::bounding::BoundingSphere;
use cesium_geospatial::frustum::PerspectiveFrustum;
use glam::DVec3;
use std::f64::consts::PI;

// ==================== CullResult ====================

#[test]
fn cull_result_is_visible() {
    assert!(CullResult::Inside.is_visible());
    assert!(CullResult::Intersecting.is_visible());
    assert!(!CullResult::Outside.is_visible());
}

// ==================== CullingContext ====================

fn make_test_context() -> CullingContext {
    let frustum = PerspectiveFrustum::new(PI / 3.0, 1.0, 0.1, 1000.0);
    CullingContext::from_perspective_frustum(
        &frustum,
        DVec3::ZERO,
        DVec3::new(0.0, 0.0, -1.0),
        DVec3::Y,
    )
}

#[test]
fn culling_context_sphere_inside() {
    let ctx = make_test_context();
    // Sphere directly in front of camera
    let sphere = BoundingSphere::new(DVec3::new(0.0, 0.0, -10.0), 1.0);
    let result = ctx.test_bounding_sphere(&sphere);
    assert!(result.is_visible());
}

#[test]
fn culling_context_sphere_outside() {
    let ctx = make_test_context();
    // Sphere far behind camera
    let sphere = BoundingSphere::new(DVec3::new(0.0, 0.0, 100.0), 1.0);
    let result = ctx.test_bounding_sphere(&sphere);
    assert_eq!(result, CullResult::Outside);
}

#[test]
fn culling_context_disabled() {
    let mut ctx = make_test_context();
    ctx.enabled = false;
    // When disabled, everything is Inside
    let sphere = BoundingSphere::new(DVec3::new(0.0, 0.0, 100.0), 1.0);
    let result = ctx.test_bounding_sphere(&sphere);
    assert_eq!(result, CullResult::Inside);
}

#[test]
fn culling_context_distance_to() {
    let ctx = make_test_context();
    let sphere = BoundingSphere::new(DVec3::new(0.0, 0.0, -50.0), 5.0);
    let dist = ctx.distance_to(&sphere);
    // distance = |camera - center| - radius = 50 - 5 = 45
    assert!((dist - 45.0).abs() < 1e-10);
}

#[test]
fn culling_context_distance_overlapping() {
    let ctx = make_test_context();
    // Sphere contains camera
    let sphere = BoundingSphere::new(DVec3::ZERO, 100.0);
    let dist = ctx.distance_to(&sphere);
    assert!((dist - 0.0).abs() < 1e-10); // Clamped to 0
}

// ==================== SceneGraph ====================

#[test]
fn scene_graph_add_node() {
    let mut graph = SceneGraph::new();
    let id = graph.add_node(SceneNode::new(0).with_name("root"));
    assert_eq!(graph.node_count(), 1);
    assert!(graph.get(id).is_some());
}

#[test]
fn scene_graph_parent_child() {
    let mut graph = SceneGraph::new();
    let parent = graph.add_node(SceneNode::new(0).with_name("parent"));
    let child = graph.add_child(parent, SceneNode::new(0).with_name("child")).unwrap();

    let parent_node = graph.get(parent).unwrap();
    assert!(parent_node.children.contains(&child));

    let child_node = graph.get(child).unwrap();
    assert_eq!(child_node.parent, Some(parent));
}

#[test]
fn scene_graph_remove_node() {
    let mut graph = SceneGraph::new();
    let id = graph.add_node(SceneNode::new(0));
    assert_eq!(graph.node_count(), 1);
    graph.remove_node(id);
    assert_eq!(graph.node_count(), 0);
}

#[test]
fn scene_node_world_bounding_sphere() {
    let node = SceneNode::new(0)
        .with_bounding_volume(BoundingSphere::new(DVec3::ZERO, 10.0))
        .with_transform(glam::DMat4::from_translation(DVec3::new(100.0, 0.0, 0.0)));

    // world_transform is identity by default (not computed), so use it directly
    let mut node = node;
    node.world_transform = node.local_transform;

    let wbs = node.world_bounding_sphere().unwrap();
    assert!((wbs.center.x - 100.0).abs() < 1e-10);
    assert!((wbs.radius - 10.0).abs() < 1e-10);
}

#[test]
fn scene_node_no_bounding_volume() {
    let node = SceneNode::new(0);
    assert!(node.world_bounding_sphere().is_none());
}

// ==================== Sort + Filter ====================

#[test]
fn sort_front_to_back_order() {
    let mut results = vec![
        make_vis_result(1, 100.0),
        make_vis_result(2, 10.0),
        make_vis_result(3, 50.0),
    ];
    sort_front_to_back(&mut results);
    assert_eq!(results[0].node_id, 2);
    assert_eq!(results[1].node_id, 3);
    assert_eq!(results[2].node_id, 1);
}

#[test]
fn sort_back_to_front_order() {
    let mut results = vec![
        make_vis_result(1, 100.0),
        make_vis_result(2, 10.0),
        make_vis_result(3, 50.0),
    ];
    sort_back_to_front(&mut results);
    assert_eq!(results[0].node_id, 1);
    assert_eq!(results[1].node_id, 3);
    assert_eq!(results[2].node_id, 2);
}

#[test]
fn filter_visible_removes_outside() {
    let results = vec![
        VisibilityResult {
            node_id: 1,
            visible: true,
            distance: 10.0,
            cull_result: CullResult::Inside,
        },
        VisibilityResult {
            node_id: 2,
            visible: false,
            distance: 20.0,
            cull_result: CullResult::Outside,
        },
        VisibilityResult {
            node_id: 3,
            visible: true,
            distance: 30.0,
            cull_result: CullResult::Intersecting,
        },
    ];
    let visible = filter_visible(results);
    assert_eq!(visible.len(), 2);
    assert_eq!(visible[0].node_id, 1);
    assert_eq!(visible[1].node_id, 3);
}

// ==================== Helpers ====================

fn make_vis_result(id: NodeId, distance: f64) -> VisibilityResult {
    VisibilityResult {
        node_id: id,
        visible: true,
        distance,
        cull_result: CullResult::Inside,
    }
}
