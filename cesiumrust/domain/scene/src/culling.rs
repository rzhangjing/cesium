//! 视锥剔除与可见性判定。
//!
//! 映射到 CesiumJS `Scene/Scene.js` 的剔除逻辑与
//! `Core/CullingVolume.js`

use cesium_geospatial::bounding::BoundingSphere;
use cesium_geospatial::frustum::{CullingVolume, PerspectiveFrustum};
use cesium_geospatial::ray::Intersect;
use glam::DVec3;

use crate::scene_graph::{NodeId, SceneGraph, SceneNode};

/// 一次剔除测试的结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CullResult {
    /// 对象完全在视锥之外。
    Outside,
    /// 对象与视锥边界相交。
    Intersecting,
    /// 对象完全在视锥之内。
    Inside,
}

impl CullResult {
    /// 若对象至少部分可见则返回 true。
    pub fn is_visible(&self) -> bool {
        !matches!(self, CullResult::Outside)
    }
}

/// 一帧的视锥剔除上下文。
#[derive(Debug, Clone)]
pub struct CullingContext {
    /// 剔除体（6 个平面）。
    pub culling_volume: CullingVolume,

    /// 用于距离计算的相机位置。
    pub camera_position: DVec3,

    /// 是否启用剔除。
    pub enabled: bool,
}

impl CullingContext {
    /// 从透视视锥创建一个剔除上下文。
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

    /// 测试一个包围球与视锥的关系。
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

    /// 计算从相机到包围球的距离。
    pub fn distance_to(&self, sphere: &BoundingSphere) -> f64 {
        let dist = self.camera_position.distance(sphere.center) - sphere.radius;
        dist.max(0.0)
    }
}

/// 一个节点的可见性判定结果。
#[derive(Debug, Clone)]
pub struct VisibilityResult {
    /// 节点 ID。
    pub node_id: NodeId,

    /// 节点是否可见。
    pub visible: bool,

    /// 到相机的距离（用于排序）。
    pub distance: f64,

    /// 剔除结果。
    pub cull_result: CullResult,
}

/// 对场景图执行视锥剔除。
///
/// 返回可见节点 ID 及其距离的列表。
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

/// 对单个节点执行剔除测试。
fn cull_node(node: &SceneNode, context: &CullingContext) -> VisibilityResult {
    // 若节点没有包围体，则假定其可见
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

/// 按距离对可见性结果排序（从前到后）。
pub fn sort_front_to_back(results: &mut [VisibilityResult]) {
    results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));
}

/// 按距离对可见性结果排序（从后到前），用于透明渲染。
pub fn sort_back_to_front(results: &mut [VisibilityResult]) {
    results.sort_by(|a, b| b.distance.partial_cmp(&a.distance).unwrap_or(std::cmp::Ordering::Equal));
}

/// 过滤结果，仅保留可见节点。
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

        // 相机前方的球体
        let sphere = BoundingSphere::new(DVec3::new(0.0, 0.0, -100.0), 10.0);
        let result = context.test_bounding_sphere(&sphere);
        assert!(result.is_visible());
    }

    #[test]
    fn test_sphere_behind_camera() {
        let context = create_test_context();

        // 相机后方的球体
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

        // 前方可见的节点
        let visible_node = SceneNode::new(0)
            .with_bounding_volume(BoundingSphere::new(DVec3::new(0.0, 0.0, -100.0), 10.0));
        scene.add_node(visible_node);

        // 后方隐藏的节点
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

        // 即使禁用到除后，相机后方的球体也应当“可见”
        let sphere = BoundingSphere::new(DVec3::new(0.0, 0.0, 100.0), 10.0);
        let result = context.test_bounding_sphere(&sphere);
        assert!(result.is_visible());
    }
}
