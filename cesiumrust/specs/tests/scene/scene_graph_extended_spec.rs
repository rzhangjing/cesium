//! Scene/SceneSpec.js (extended) → Rust integration tests.
//!
//! Maps to CesiumJS:
//! - Scene/Scene.js (scene graph traversal, world transforms)
//! - Scene/Primitive.js (renderable collection)
//!
//! A-class tests: world transform propagation, traverse visible-only,
//! collect renderables, remove with descendants, world bounding sphere,
//! multi-level hierarchy, add_child to invalid parent.
//! C-class omitted: WebGL rendering, canvas operations, pick.

use cesium_scene::{RenderableContent, SceneGraph, SceneNode};
use cesium_geospatial::bounding::BoundingSphere;
use glam::{DMat4, DVec3};

// === World transform propagation ===

#[test]
fn world_transform_root_identity() {
    let mut graph = SceneGraph::new();
    let node = SceneNode::new(0)
        .with_transform(DMat4::from_translation(DVec3::new(5.0, 10.0, 15.0)));
    let id = graph.add_node(node);
    graph.update_world_transforms();

    let n = graph.get(id).unwrap();
    let pos = n.world_transform.w_axis.truncate();
    assert!((pos.x - 5.0).abs() < 1e-10);
    assert!((pos.y - 10.0).abs() < 1e-10);
    assert!((pos.z - 15.0).abs() < 1e-10);
}

#[test]
fn world_transform_parent_child_accumulates() {
    let mut graph = SceneGraph::new();
    let parent = SceneNode::new(0)
        .with_transform(DMat4::from_translation(DVec3::new(10.0, 0.0, 0.0)));
    let parent_id = graph.add_node(parent);

    let child = SceneNode::new(0)
        .with_transform(DMat4::from_translation(DVec3::new(0.0, 20.0, 0.0)));
    let child_id = graph.add_child(parent_id, child).unwrap();

    graph.update_world_transforms();

    // Child world = parent(10,0,0) * child(0,20,0) = (10,20,0)
    let c = graph.get(child_id).unwrap();
    let pos = c.world_transform.w_axis.truncate();
    assert!((pos.x - 10.0).abs() < 1e-10);
    assert!((pos.y - 20.0).abs() < 1e-10);
}

#[test]
fn world_transform_three_levels() {
    let mut graph = SceneGraph::new();
    let root = SceneNode::new(0)
        .with_transform(DMat4::from_translation(DVec3::new(1.0, 0.0, 0.0)));
    let root_id = graph.add_node(root);

    let mid = SceneNode::new(0)
        .with_transform(DMat4::from_translation(DVec3::new(0.0, 2.0, 0.0)));
    let mid_id = graph.add_child(root_id, mid).unwrap();

    let leaf = SceneNode::new(0)
        .with_transform(DMat4::from_translation(DVec3::new(0.0, 0.0, 3.0)));
    let leaf_id = graph.add_child(mid_id, leaf).unwrap();

    graph.update_world_transforms();

    let l = graph.get(leaf_id).unwrap();
    let pos = l.world_transform.w_axis.truncate();
    assert!((pos.x - 1.0).abs() < 1e-10);
    assert!((pos.y - 2.0).abs() < 1e-10);
    assert!((pos.z - 3.0).abs() < 1e-10);
}

#[test]
fn world_transform_with_scale() {
    let mut graph = SceneGraph::new();
    let parent = SceneNode::new(0)
        .with_transform(DMat4::from_scale(DVec3::new(2.0, 2.0, 2.0)));
    let parent_id = graph.add_node(parent);

    let child = SceneNode::new(0)
        .with_transform(DMat4::from_translation(DVec3::new(5.0, 0.0, 0.0)));
    let child_id = graph.add_child(parent_id, child).unwrap();

    graph.update_world_transforms();

    // Child world position = parent_scale * child_translation = (10, 0, 0)
    let c = graph.get(child_id).unwrap();
    let pos = c.world_transform.w_axis.truncate();
    assert!((pos.x - 10.0).abs() < 1e-10);
}

// === Traverse ===

#[test]
fn traverse_visits_visible_only() {
    let mut graph = SceneGraph::new();
    let root_id = graph.add_node(SceneNode::new(0).with_name("Root"));

    let mut visible = SceneNode::new(0).with_name("Visible");
    visible.visible = true;
    graph.add_child(root_id, visible);

    let mut hidden = SceneNode::new(0).with_name("Hidden");
    hidden.visible = false;
    let hidden_id = graph.add_child(root_id, hidden).unwrap();

    // Child of hidden node should also not be visited
    let child_of_hidden = SceneNode::new(0).with_name("ChildOfHidden");
    graph.add_child(hidden_id, child_of_hidden);

    let mut names = Vec::new();
    graph.traverse(|node| {
        names.push(node.name.clone().unwrap_or_default());
    });

    assert_eq!(names.len(), 2);
    assert!(names.contains(&"Root".to_string()));
    assert!(names.contains(&"Visible".to_string()));
    assert!(!names.contains(&"Hidden".to_string()));
    assert!(!names.contains(&"ChildOfHidden".to_string()));
}

#[test]
fn traverse_empty_graph() {
    let graph = SceneGraph::new();
    let mut count = 0;
    graph.traverse(|_| count += 1);
    assert_eq!(count, 0);
}

// === Collect renderables ===

#[test]
fn collect_renderable_ids_mixed() {
    let mut graph = SceneGraph::new();

    let with_mesh = SceneNode::new(0).with_renderable(RenderableContent::Mesh {
        mesh_id: 1,
        material_id: 2,
    });
    let id1 = graph.add_node(with_mesh);

    let without = SceneNode::new(0);
    graph.add_node(without);

    let with_model = SceneNode::new(0).with_renderable(RenderableContent::Model {
        model_id: 10,
    });
    let id3 = graph.add_node(with_model);

    let renderables = graph.collect_renderable_ids();
    assert_eq!(renderables.len(), 2);
    assert!(renderables.contains(&id1));
    assert!(renderables.contains(&id3));
}

#[test]
fn collect_renderable_skips_hidden() {
    let mut graph = SceneGraph::new();

    let mut hidden_renderable = SceneNode::new(0).with_renderable(RenderableContent::PointCloud {
        point_cloud_id: 5,
        point_count: 1000,
    });
    hidden_renderable.visible = false;
    graph.add_node(hidden_renderable);

    let renderables = graph.collect_renderable_ids();
    assert_eq!(renderables.len(), 0);
}

// === Remove with descendants ===

#[test]
fn remove_node_cascades_to_grandchildren() {
    let mut graph = SceneGraph::new();
    let root_id = graph.add_node(SceneNode::new(0).with_name("Root"));
    let child_id = graph.add_child(root_id, SceneNode::new(0).with_name("Child")).unwrap();
    let grandchild_id = graph.add_child(child_id, SceneNode::new(0).with_name("Grandchild")).unwrap();

    assert_eq!(graph.node_count(), 3);
    graph.remove_node(child_id);
    assert_eq!(graph.node_count(), 1);
    assert!(graph.get(child_id).is_none());
    assert!(graph.get(grandchild_id).is_none());
    assert!(graph.get(root_id).is_some());
}

#[test]
fn remove_root_node() {
    let mut graph = SceneGraph::new();
    let root_id = graph.add_node(SceneNode::new(0));
    let child_id = graph.add_child(root_id, SceneNode::new(0)).unwrap();

    graph.remove_node(root_id);
    assert_eq!(graph.node_count(), 0);
    assert!(graph.get(child_id).is_none());
    assert!(graph.roots().is_empty());
}

// === Add child to invalid parent ===

#[test]
fn add_child_invalid_parent_returns_none() {
    let mut graph = SceneGraph::new();
    let result = graph.add_child(999, SceneNode::new(0));
    assert!(result.is_none());
    assert_eq!(graph.node_count(), 0);
}

// === World bounding sphere ===

#[test]
fn world_bounding_sphere_translation() {
    let mut node = SceneNode::new(0)
        .with_transform(DMat4::from_translation(DVec3::new(100.0, 0.0, 0.0)))
        .with_bounding_volume(BoundingSphere::new(DVec3::ZERO, 10.0));
    node.world_transform = node.local_transform;

    let ws = node.world_bounding_sphere().unwrap();
    assert!((ws.center.x - 100.0).abs() < 1e-10);
    assert!((ws.radius - 10.0).abs() < 1e-10);
}

#[test]
fn world_bounding_sphere_with_scale() {
    let mut node = SceneNode::new(0)
        .with_transform(DMat4::from_scale(DVec3::new(3.0, 3.0, 3.0)))
        .with_bounding_volume(BoundingSphere::new(DVec3::ZERO, 5.0));
    node.world_transform = node.local_transform;

    let ws = node.world_bounding_sphere().unwrap();
    // Radius scaled by max axis scale (3.0)
    assert!((ws.radius - 15.0).abs() < 1e-10);
}

#[test]
fn world_bounding_sphere_none() {
    let node = SceneNode::new(0);
    assert!(node.world_bounding_sphere().is_none());
}

// === Roots tracking ===

#[test]
fn roots_tracked_correctly() {
    let mut graph = SceneGraph::new();
    let r1 = graph.add_node(SceneNode::new(0));
    let r2 = graph.add_node(SceneNode::new(0));
    let _c = graph.add_child(r1, SceneNode::new(0)).unwrap();

    assert_eq!(graph.roots().len(), 2);
    assert!(graph.roots().contains(&r1));
    assert!(graph.roots().contains(&r2));
}
