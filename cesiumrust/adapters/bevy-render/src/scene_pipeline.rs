//! Scene rendering pipeline integration.
//!
//! Bridges the domain SceneGraph → Culling → DrawCommand system
//! to Bevy's ECS rendering.
//!
//! Maps to CesiumJS `Scene/Scene.js` render loop.

use bevy::prelude::*;
use cesium_geospatial::frustum::PerspectiveFrustum;
use cesium_scene::culling::{self, CullingContext, VisibilityResult};
use cesium_scene::draw_command::{DrawCommand, FrameStatistics, RenderCommandList, RenderPass};
use cesium_scene::scene_graph::{NodeId, RenderableContent, SceneGraph};
use glam::DVec3;

/// Resource holding the scene graph.
#[derive(Resource, Default)]
pub struct SceneGraphResource {
    /// The domain scene graph.
    pub scene: SceneGraph,
}

/// Resource holding frame statistics.
#[derive(Resource, Default)]
pub struct FrameStatsResource {
    /// Statistics for the current frame.
    pub stats: FrameStatistics,
}

/// Component linking a Bevy entity to a scene graph node.
#[derive(Component)]
pub struct SceneNodeLink {
    /// The scene graph node ID.
    pub node_id: NodeId,
}

/// Component marking an entity as culled (hidden) this frame.
#[derive(Component)]
pub struct CulledThisFrame;

/// Configuration for the scene pipeline.
#[derive(Resource)]
pub struct ScenePipelineConfig {
    /// Vertical field of view in radians.
    pub fov_y: f64,
    /// Aspect ratio (width / height).
    pub aspect_ratio: f64,
    /// Near clipping plane distance.
    pub near: f64,
    /// Far clipping plane distance.
    pub far: f64,
    /// Whether frustum culling is enabled.
    pub culling_enabled: bool,
}

impl Default for ScenePipelineConfig {
    fn default() -> Self {
        Self {
            fov_y: std::f64::consts::FRAC_PI_4,
            aspect_ratio: 16.0 / 9.0,
            near: 0.1,
            far: 1e12, // Very far for globe-scale rendering
            culling_enabled: true,
        }
    }
}

/// Converts a domain DMat4 (f64) to a Bevy Transform (f32).
///
/// This is the precision boundary where f64 domain transforms
/// become f32 GPU-ready transforms.
pub fn dmat4_to_transform(mat: glam::DMat4) -> Transform {
    // Extract translation
    let translation = mat.w_axis.truncate();

    // Extract rotation (upper 3x3, normalized)
    let col0 = mat.x_axis.truncate();
    let col1 = mat.y_axis.truncate();
    let col2 = mat.z_axis.truncate();

    let scale_x = col0.length();
    let scale_y = col1.length();
    let scale_z = col2.length();

    // Build rotation matrix from normalized columns
    let rot = if scale_x > 1e-10 && scale_y > 1e-10 && scale_z > 1e-10 {
        let r0 = (col0 / scale_x).as_vec3();
        let r1 = (col1 / scale_y).as_vec3();
        let r2 = (col2 / scale_z).as_vec3();
        let mat3 = glam::Mat3::from_cols(r0, r1, r2);
        Quat::from_mat3(&mat3)
    } else {
        Quat::IDENTITY
    };

    Transform {
        translation: Vec3::new(translation.x as f32, translation.y as f32, translation.z as f32),
        rotation: rot,
        scale: Vec3::new(scale_x as f32, scale_y as f32, scale_z as f32),
    }
}

/// Performs frustum culling on the scene graph and returns visible nodes.
///
/// # Arguments
/// * `scene` - The scene graph to cull
/// * `camera_position` - Camera position in ECEF
/// * `camera_direction` - Camera look direction (normalized)
/// * `camera_up` - Camera up direction (normalized)
/// * `config` - Pipeline configuration
///
/// # Returns
/// Visibility results for all traversed nodes
pub fn perform_culling(
    scene: &SceneGraph,
    camera_position: DVec3,
    camera_direction: DVec3,
    camera_up: DVec3,
    config: &ScenePipelineConfig,
) -> Vec<VisibilityResult> {
    let frustum = PerspectiveFrustum::new(
        config.fov_y,
        config.aspect_ratio,
        config.near,
        config.far,
    );

    let context = CullingContext::from_perspective_frustum(
        &frustum,
        camera_position,
        camera_direction,
        camera_up,
    );

    let mut context = context;
    context.enabled = config.culling_enabled;

    culling::cull_scene(scene, &context)
}

/// Generates draw commands from visible scene nodes.
///
/// # Arguments
/// * `scene` - The scene graph
/// * `visibility` - Visibility results from culling
///
/// # Returns
/// A sorted render command list
pub fn generate_draw_commands(
    scene: &SceneGraph,
    visibility: &[VisibilityResult],
) -> RenderCommandList {
    let mut command_list = RenderCommandList::new();

    for vis in visibility {
        if !vis.visible {
            continue;
        }

        let node = match scene.get(vis.node_id) {
            Some(n) => n,
            None => continue,
        };

        let renderable = match &node.renderable {
            Some(r) => r,
            None => continue,
        };

        let (geometry_id, material_id, pass) = match renderable {
            RenderableContent::Mesh { mesh_id, material_id } => {
                (*mesh_id, *material_id, RenderPass::Opaque)
            }
            RenderableContent::Model { model_id } => {
                (*model_id, 0, RenderPass::Cesium3DTile)
            }
            RenderableContent::PointCloud { point_cloud_id, .. } => {
                (*point_cloud_id, 0, RenderPass::Opaque)
            }
            RenderableContent::DebugWireframe { .. } => {
                (0, 0, RenderPass::Overlay)
            }
        };

        let cmd = DrawCommand::new(geometry_id, material_id)
            .with_pass(pass)
            .with_model_matrix(node.world_transform)
            .with_sort_key(vis.distance);

        command_list.push(cmd);
    }

    command_list.sort();
    command_list
}

/// Converts a RenderCommandList into frame statistics.
pub fn compute_frame_statistics(command_list: &RenderCommandList, culled_count: usize) -> FrameStatistics {
    FrameStatistics {
        draw_calls: command_list.len(),
        culled_objects: culled_count,
        ..Default::default()
    }
}

/// Full pipeline: cull → generate commands → statistics.
///
/// This is the main entry point for the scene rendering pipeline.
pub fn execute_scene_pipeline(
    scene: &SceneGraph,
    camera_position: DVec3,
    camera_direction: DVec3,
    camera_up: DVec3,
    config: &ScenePipelineConfig,
) -> (RenderCommandList, FrameStatistics) {
    // Step 1: Cull
    let visibility = perform_culling(scene, camera_position, camera_direction, camera_up, config);

    // Step 2: Count culled
    let total = visibility.len();
    let visible_count = visibility.iter().filter(|v| v.visible).count();
    let culled_count = total - visible_count;

    // Step 3: Generate draw commands
    let command_list = generate_draw_commands(scene, &visibility);

    // Step 4: Statistics
    let stats = compute_frame_statistics(&command_list, culled_count);

    (command_list, stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_geospatial::bounding::BoundingSphere;
    use cesium_scene::scene_graph::SceneNode;

    #[test]
    fn test_dmat4_to_transform_identity() {
        let transform = dmat4_to_transform(glam::DMat4::IDENTITY);
        assert!((transform.translation.x).abs() < 1e-6);
        assert!((transform.translation.y).abs() < 1e-6);
        assert!((transform.translation.z).abs() < 1e-6);
        assert!((transform.scale.x - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_dmat4_to_transform_translation() {
        let mat = glam::DMat4::from_translation(DVec3::new(100.0, 200.0, 300.0));
        let transform = dmat4_to_transform(mat);
        assert!((transform.translation.x - 100.0).abs() < 1e-4);
        assert!((transform.translation.y - 200.0).abs() < 1e-4);
        assert!((transform.translation.z - 300.0).abs() < 1e-4);
    }

    #[test]
    fn test_dmat4_to_transform_scale() {
        let mat = glam::DMat4::from_scale(DVec3::new(2.0, 3.0, 4.0));
        let transform = dmat4_to_transform(mat);
        assert!((transform.scale.x - 2.0).abs() < 1e-6);
        assert!((transform.scale.y - 3.0).abs() < 1e-6);
        assert!((transform.scale.z - 4.0).abs() < 1e-6);
    }

    #[test]
    fn test_perform_culling() {
        let mut scene = SceneGraph::new();

        // Node in front of camera
        let visible = SceneNode::new(0)
            .with_bounding_volume(BoundingSphere::new(DVec3::new(0.0, 0.0, -100.0), 10.0));
        scene.add_node(visible);

        // Node behind camera
        let hidden = SceneNode::new(0)
            .with_bounding_volume(BoundingSphere::new(DVec3::new(0.0, 0.0, 100.0), 10.0));
        scene.add_node(hidden);

        scene.update_world_transforms();

        let config = ScenePipelineConfig::default();
        let results = perform_culling(
            &scene,
            DVec3::ZERO,
            DVec3::new(0.0, 0.0, -1.0),
            DVec3::new(0.0, 1.0, 0.0),
            &config,
        );

        assert_eq!(results.len(), 2);
        let visible_count = results.iter().filter(|r| r.visible).count();
        assert_eq!(visible_count, 1);
    }

    #[test]
    fn test_generate_draw_commands() {
        let mut scene = SceneGraph::new();

        let node = SceneNode::new(0).with_renderable(RenderableContent::Mesh {
            mesh_id: 42,
            material_id: 7,
        });
        let node_id = scene.add_node(node);
        scene.update_world_transforms();

        let visibility = vec![VisibilityResult {
            node_id,
            visible: true,
            distance: 50.0,
            cull_result: culling::CullResult::Inside,
        }];

        let commands = generate_draw_commands(&scene, &visibility);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands.commands_for_pass(RenderPass::Opaque).len(), 1);
    }

    #[test]
    fn test_execute_scene_pipeline() {
        let mut scene = SceneGraph::new();

        // Visible node with renderable
        let node = SceneNode::new(0)
            .with_bounding_volume(BoundingSphere::new(DVec3::new(0.0, 0.0, -100.0), 10.0))
            .with_renderable(RenderableContent::Mesh {
                mesh_id: 1,
                material_id: 1,
            });
        scene.add_node(node);

        // Hidden node behind camera
        let hidden = SceneNode::new(0)
            .with_bounding_volume(BoundingSphere::new(DVec3::new(0.0, 0.0, 100.0), 10.0))
            .with_renderable(RenderableContent::Mesh {
                mesh_id: 2,
                material_id: 2,
            });
        scene.add_node(hidden);

        scene.update_world_transforms();

        let config = ScenePipelineConfig::default();
        let (commands, stats) = execute_scene_pipeline(
            &scene,
            DVec3::ZERO,
            DVec3::new(0.0, 0.0, -1.0),
            DVec3::new(0.0, 1.0, 0.0),
            &config,
        );

        // Only the visible node should produce a draw command
        assert_eq!(commands.len(), 1);
        assert_eq!(stats.draw_calls, 1);
        assert_eq!(stats.culled_objects, 1);
    }

    #[test]
    fn test_culling_disabled_pipeline() {
        let mut scene = SceneGraph::new();

        // Node behind camera
        let node = SceneNode::new(0)
            .with_bounding_volume(BoundingSphere::new(DVec3::new(0.0, 0.0, 100.0), 10.0))
            .with_renderable(RenderableContent::Mesh {
                mesh_id: 1,
                material_id: 1,
            });
        scene.add_node(node);
        scene.update_world_transforms();

        let config = ScenePipelineConfig {
            culling_enabled: false,
            ..Default::default()
        };

        let (commands, stats) = execute_scene_pipeline(
            &scene,
            DVec3::ZERO,
            DVec3::new(0.0, 0.0, -1.0),
            DVec3::new(0.0, 1.0, 0.0),
            &config,
        );

        // With culling disabled, even the behind-camera node renders
        assert_eq!(commands.len(), 1);
        assert_eq!(stats.culled_objects, 0);
    }
}
