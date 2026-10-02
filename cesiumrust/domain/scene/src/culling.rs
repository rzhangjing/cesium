//! 视锥剔除与可见性判定。
//!
//! 本模块提供三类能力：
//! - 以 [`CullingContext`] 承载一帧的剔除体与相机位置；
//! - 以 [`CullResult`] 表达包围球与视锥的相交关系；
//! - 以 [`cull_scene`] 遍历场景图逐节点判定可见性并计算距离。
//!
//! 剔除仅做保守判定：只要包围球与任一视锥平面相交即视为可见，
//! 宁可多画不可漏画，交由后续精确裁剪处理边界情况。

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
        // 由相机位置、朝向与 up 向量推出六面剔除体，再连同位置组装为上下文
        let culling_volume = frustum.compute_culling_volume(position, direction, up);
        Self {
            culling_volume,
            camera_position: position,
            enabled: true,
        }
    }

    /// 测试一个包围球与视锥的关系。
    pub fn test_bounding_sphere(&self, sphere: &BoundingSphere) -> CullResult {
        // 剔除关闭时一律视为完全在内，保持调用方无需分支判断
        if !self.enabled {
            return CullResult::Inside;
        }
        // 将几何求交的三态枚举映射为领域内的剔除结论
        match self.culling_volume.visibility(sphere) {
            Intersect::Outside => CullResult::Outside,
            Intersect::Intersecting => CullResult::Intersecting,
            Intersect::Inside => CullResult::Inside,
        }
    }

    /// 计算从相机到包围球的距离。
    pub fn distance_to(&self, sphere: &BoundingSphere) -> f64 {
        // 以球心距减去半径得到球面最近距离，负值（相机在球内）钳制为 0
        let dist = self.camera_position.distance(sphere.center) - sphere.radius;
        dist.max(0.0)
    }
}

/// 一个节点的可见性判定结果。
#[derive(Debug, Clone)]
pub struct VisibilityResult {
    /// 被判定节点的唯一 ID。
    pub node_id: NodeId,

    /// 节点是否至少部分可见（等于 cull_result.is_visible()）。
    pub visible: bool,

    /// 到相机最近面的距离（单位同世界坐标，用于排序）。
    pub distance: f64,

    /// 与视锥的原始相交结果（Outside/Intersecting/Inside）。
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

    // 深度优先遍历整棵场景图，逐节点跑一次剔除并累积结果
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
    // 不透明物体先画近的，可尽早写入深度以剔除后续被遮挡者
    results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));
}

/// 按距离对可见性结果排序（从后到前），用于透明渲染。
pub fn sort_back_to_front(results: &mut [VisibilityResult]) {
    // 透明物体需先画远的再画近的，才能正确叠加颜色混合
    results.sort_by(|a, b| b.distance.partial_cmp(&a.distance).unwrap_or(std::cmp::Ordering::Equal));
}

/// 过滤结果，仅保留可见节点。
pub fn filter_visible(results: Vec<VisibilityResult>) -> Vec<VisibilityResult> {
    // 丢弃完全在视锥之外的项，得到真正参与绘制的候选集
    results.into_iter().filter(|r| r.visible).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene_graph::SceneNode;
    use std::f64::consts::FRAC_PI_4;

    fn create_test_frustum() -> PerspectiveFrustum {
        // 45° 视场、16:9 宽高比、近裁剪 0.1、远裁剪 10000 的常用透视参数
        PerspectiveFrustum::new(FRAC_PI_4, 16.0 / 9.0, 0.1, 10000.0)
    }

    fn create_test_context() -> CullingContext {
        // 相机置于原点朝 -Z 看向，构造一个标准右手坐标下的剔除上下文
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
        // Inside 与 Intersecting 均属至少部分可见，仅 Outside 不可见
        assert!(CullResult::Inside.is_visible());
        assert!(CullResult::Intersecting.is_visible());
        assert!(!CullResult::Outside.is_visible());
    }

    #[test]
    fn test_sphere_in_frustum() {
        let context = create_test_context();

        // 相机前方的球体
        // -Z 位于看向方向内且落在远裁剪之前，应判为可见
        let sphere = BoundingSphere::new(DVec3::new(0.0, 0.0, -100.0), 10.0);
        let result = context.test_bounding_sphere(&sphere);
        assert!(result.is_visible());
    }

    #[test]
    fn test_sphere_behind_camera() {
        let context = create_test_context();

        // 相机后方的球体
        // +Z 位于看向（-Z）的反向，应被近裁剪面剔除
        let sphere = BoundingSphere::new(DVec3::new(0.0, 0.0, 100.0), 10.0);
        let result = context.test_bounding_sphere(&sphere);
        assert!(!result.is_visible());
    }

    #[test]
    fn test_distance_calculation() {
        let context = create_test_context();

        // 球心距相机 100、半径 10，最近面距离应为 90
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

        // 世界变换需先更新，包围球才能落入正确的世界坐标供剔除使用
        scene.update_world_transforms();

        let context = create_test_context();
        let results = cull_scene(&scene, &context);

        // 两个节点均被遍历到，但只有前方那个通过可见性过滤
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

        // 从前到后排序：按距离升序 50(节点2) < 100(节点1) < 200(节点3)
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

        // 从后到前排序：远的(100)排在近的(50)之前
        assert_eq!(results[0].node_id, 1);
        assert_eq!(results[1].node_id, 2);
    }

    #[test]
    fn test_culling_disabled() {
        let mut context = create_test_context();
        context.enabled = false;

        // 即使禁用到除后，相机后方的球体也应当“可见”
        // enabled=false 短路了剔除体求交，test_bounding_sphere 无条件返回 Inside
        let sphere = BoundingSphere::new(DVec3::new(0.0, 0.0, 100.0), 10.0);
        let result = context.test_bounding_sphere(&sphere);
        assert!(result.is_visible());
    }
}
