//! 3D Tiles 的 Detail-of-Detail (LOD) 选择。
//!
//! 实现屏幕空间误差（SSE）计算与瓦片遍历，
//! 以选择要渲染哪些瓦片。

use crate::tile::{Tile, TileRefine};
use cesium_geospatial::ellipsoid::Ellipsoid;
use glam::DVec3;

/// 用于 LOD 计算的相机状态。
#[derive(Debug, Clone)]
pub struct CameraState {
    /// 相机位置，ECEF 坐标。
    pub position: DVec3,

    /// 相机视线方向（已归一化）。
    pub direction: DVec3,

    /// 相机上方方向（已归一化）。
    pub up: DVec3,

    /// 垂直视野角，弧度。
    pub fov_y: f64,

    /// 视口高度，像素。
    pub viewport_height: f64,
}

impl CameraState {
    /// 创建一个新的相机状态。
    pub fn new(position: DVec3, direction: DVec3, up: DVec3, fov_y: f64, viewport_height: f64) -> Self {
        Self {
            position,
            direction: direction.normalize(),
            up: up.normalize(),
            fov_y,
            viewport_height,
        }
    }

    /// 为给定的几何误差和距离计算屏幕空间误差。
    ///
    /// SSE = (geometricError * viewportHeight) / (distance * 2 * tan(fovY / 2))
    pub fn compute_screen_space_error(&self, geometric_error: f64, distance: f64) -> f64 {
        // 零距离（相机在瓦片内）时误差视为无穷大，强制细化
        if distance <= 0.0 {
            return f64::MAX;
        }

        // 分母中的 2*tan(fovY/2) 为视锥纵向半张角的正切项
        let sse_denominator = 2.0 * (self.fov_y / 2.0).tan();
        (geometric_error * self.viewport_height) / (distance * sse_denominator)
    }
}

/// 单个瓦片的选择结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileSelectionResult {
    /// 应渲染该瓦片（内容已加载）。
    Render,
    /// 应细化该瓦片（需考虑子瓦片）。
    Refine,
    /// 该瓦片被剔除（不可见）。
    Culled,
}

/// 一个被选中的瓦片及其选择结果。
#[derive(Debug, Clone)]
pub struct SelectedTile {
    /// 瓦片在树中的路径（从根开始的索引）。
    pub path: Vec<usize>,

    /// 选择结果。
    pub result: TileSelectionResult,

    /// 本瓦片的屏幕空间误差。
    pub screen_space_error: f64,

    /// 到瓦片的相机距离。
    pub distance_to_camera: f64,
}

/// LOD 选择上下文。
#[derive(Debug, Clone)]
pub struct LodSelectionContext {
    /// 最大屏幕空间误差阈值。
    pub maximum_screen_space_error: f64,

    /// 是否剔除视锥体外的瓦片。
    pub cull_with_frustum: bool,

    /// 是否跳过已被细化的瓦片。
    pub skip_level_of_detail: bool,
}

impl Default for LodSelectionContext {
    /// 默认上下文：最大 SSE 为 16.0，启用视锥剔除，不跳过 LOD。
    fn default() -> Self {
        Self {
            maximum_screen_space_error: 16.0,
            cull_with_frustum: true,
            skip_level_of_detail: false,
        }
    }
}

/// 计算相机到瓦片包围体的距离。
pub fn compute_distance_to_tile(
    camera: &CameraState,
    tile: &Tile,
    ellipsoid: &Ellipsoid,
) -> f64 {
    // 距离取相机位置到瓦片包围体的最短距离
    tile.bounding_volume.distance_to(camera.position, ellipsoid)
}

/// 计算瓦片的屏幕空间误差。
pub fn compute_tile_sse(
    camera: &CameraState,
    tile: &Tile,
    ellipsoid: &Ellipsoid,
) -> f64 {
    // 先算相机距离，再结合瓦片几何误差得出 SSE
    let distance = compute_distance_to_tile(camera, tile, ellipsoid);
    camera.compute_screen_space_error(tile.geometric_error, distance)
}

/// 根据瓦片的屏幕空间误差判断它是否应被细化。
///
/// 若瓦片的 SSE 超过最大阈值且有子瓦片，则应被细化。
pub fn should_refine_tile(
    sse: f64,
    max_sse: f64,
    has_children: bool,
) -> bool {
    // 仅当 SSE 超阈值且存在子瓦片时才细化
    has_children && sse > max_sse
}

/// 使用简单的遍历算法选择要渲染的瓦片。
///
/// 实现一个基本的自上而下遍历，它会：
/// 1. 为每个瓦片计算 SSE
/// 2. 若 SSE <= 阈值，选中该瓦片渲染
/// 3. 若 SSE > 阈值且有子瓦片，细化到子瓦片
///
/// # 参数
/// * `root` - 瓦片集的根瓦片
/// * `camera` - 用于 SSE 计算的相机状态
/// * `context` - LOD 选择上下文
/// * `ellipsoid` - 用于坐标转换的椭球体
///
/// # 返回
/// 被选中瓦片及其选择结果的列表
pub fn select_tiles(
    root: &Tile,
    camera: &CameraState,
    context: &LodSelectionContext,
    ellipsoid: &Ellipsoid,
) -> Vec<SelectedTile> {
    // 从根开始自上而下递归遍历，初始父级细化模式为 Replace
    let mut selected = Vec::new();
    select_tiles_recursive(
        root,
        camera,
        context,
        ellipsoid,
        TileRefine::Replace,
        &[],
        &mut selected,
    );
    selected
}

/// 递归瓦片选择辅助函数。
#[allow(clippy::too_many_arguments)]
fn select_tiles_recursive(
    tile: &Tile,
    camera: &CameraState,
    context: &LodSelectionContext,
    ellipsoid: &Ellipsoid,
    parent_refine: TileRefine,
    path: &[usize],
    selected: &mut Vec<SelectedTile>,
) {
    let distance = compute_distance_to_tile(camera, tile, ellipsoid);
    let sse = camera.compute_screen_space_error(tile.geometric_error, distance);

    let refine_mode = tile.effective_refine(parent_refine);
    let has_children = !tile.children.is_empty();
    let should_refine = should_refine_tile(sse, context.maximum_screen_space_error, has_children);

    if should_refine {
        // 细化：遍历子瓦片
        for (i, child) in tile.children.iter().enumerate() {
            let mut child_path = path.to_vec();
            child_path.push(i);
            select_tiles_recursive(
                child,
                camera,
                context,
                ellipsoid,
                refine_mode,
                &child_path,
                selected,
            );
        }

        // 对于 ADD 细化，同时也渲染父瓦片
        if refine_mode == TileRefine::Add && tile.has_content() {
            selected.push(SelectedTile {
                path: path.to_vec(),
                result: TileSelectionResult::Render,
                screen_space_error: sse,
                distance_to_camera: distance,
            });
        }
    } else {
        // 渲染本瓦片
        if tile.has_content() {
            selected.push(SelectedTile {
                path: path.to_vec(),
                result: TileSelectionResult::Render,
                screen_space_error: sse,
                distance_to_camera: distance,
            });
        } else if has_children {
            // 空瓦片带子瓦片：仍然细化
            for (i, child) in tile.children.iter().enumerate() {
                let mut child_path = path.to_vec();
                child_path.push(i);
                select_tiles_recursive(
                    child,
                    camera,
                    context,
                    ellipsoid,
                    refine_mode,
                    &child_path,
                    selected,
                );
            }
        }
    }
}

/// 根据瓦片在树中的路径获取它。
pub fn get_tile_by_path<'a>(root: &'a Tile, path: &[usize]) -> Option<&'a Tile> {
    let mut current = root;
    // 逐层按索引下钻，任一索引越界即视为路径无效
    for &index in path {
        current = current.children.get(index)?;
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bounding_volume::BoundingVolume;
    use crate::tile::TileContent;

    fn create_test_camera() -> CameraState {
        CameraState::new(
            DVec3::new(0.0, 0.0, 1000.0),
            DVec3::new(0.0, 0.0, -1.0),
            DVec3::new(0.0, 1.0, 0.0),
            std::f64::consts::FRAC_PI_4, // 45 度
            1080.0,
        )
    }

    fn create_test_tile(geometric_error: f64, has_children: bool) -> Tile {
        let children = if has_children {
            vec![
                Tile {
                    bounding_volume: BoundingVolume::from_sphere(DVec3::ZERO, 50.0),
                    geometric_error: geometric_error / 2.0,
                    refine: None,
                    transform: None,
                    content: Some(TileContent {
                        uri: "child.b3dm".to_string(),
                        bounding_volume: None,
                        group: None,
                    }),
                    contents: None,
                    children: vec![],
                    viewer_request_volume: None,
                    extras: None,
                },
            ]
        } else {
            vec![]
        };

        Tile {
            bounding_volume: BoundingVolume::from_sphere(DVec3::ZERO, 100.0),
            geometric_error,
            refine: Some(TileRefine::Replace),
            transform: None,
            content: Some(TileContent {
                uri: "parent.b3dm".to_string(),
                bounding_volume: None,
                group: None,
            }),
            contents: None,
            children,
            viewer_request_volume: None,
            extras: None,
        }
    }

    #[test]
    /// 验证 SSE 公式的计算结果与手算一致。
    fn test_screen_space_error_computation() {
        let camera = create_test_camera();

        // SSE = (geometricError * viewportHeight) / (distance * 2 * tan(fovY / 2))
        // SSE = (100 * 1080) / (1000 * 2 * tan(22.5°))
        let sse = camera.compute_screen_space_error(100.0, 1000.0);
        let expected = (100.0 * 1080.0) / (1000.0 * 2.0 * (std::f64::consts::FRAC_PI_4 / 2.0).tan());
        assert!((sse - expected).abs() < 1e-10);
    }

    #[test]
    /// 验证几何误差越大 SSE 越大。
    fn test_sse_increases_with_geometric_error() {
        let camera = create_test_camera();

        let sse_small = camera.compute_screen_space_error(10.0, 1000.0);
        let sse_large = camera.compute_screen_space_error(100.0, 1000.0);

        assert!(sse_large > sse_small);
    }

    #[test]
    /// 验证相机越近 SSE 越大。
    fn test_sse_increases_with_proximity() {
        let camera = create_test_camera();

        let sse_far = camera.compute_screen_space_error(100.0, 10000.0);
        let sse_near = camera.compute_screen_space_error(100.0, 1000.0);

        assert!(sse_near > sse_far);
    }

    #[test]
    /// 验证细化判定：需同时满足 SSE 超阈值与有子瓦片。
    fn test_should_refine_tile() {
        assert!(should_refine_tile(20.0, 16.0, true)); // SSE > 阈值，有子瓦片
        assert!(!should_refine_tile(10.0, 16.0, true)); // SSE < 阈值
        assert!(!should_refine_tile(20.0, 16.0, false)); // 无子瓦片
    }

    #[test]
    /// 验证低误差时直接渲染根瓦片不细化。
    fn test_select_tiles_no_refinement() {
        let root = create_test_tile(10.0, false); // 低误差，无子瓦片
        let camera = create_test_camera();
        let context = LodSelectionContext::default();

        let selected = select_tiles(&root, &camera, &context, &Ellipsoid::WGS84);

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].result, TileSelectionResult::Render);
    }

    #[test]
    fn test_select_tiles_with_refinement() {
        let root = create_test_tile(1000.0, true); // 高误差，有子瓦片
        let camera = create_test_camera();
        let context = LodSelectionContext::default();

        let selected = select_tiles(&root, &camera, &context, &Ellipsoid::WGS84);

        // 应细化到子瓦片
        assert!(selected.iter().any(|t| t.path == vec![0]));
    }

    #[test]
    fn test_get_tile_by_path() {
        let root = create_test_tile(100.0, true);

        // 根拥有空路径
        let root_tile = get_tile_by_path(&root, &[]);
        assert!(root_tile.is_some());
        assert_eq!(root_tile.unwrap().geometric_error, 100.0);

        // 子瓦片拥有路径 [0]
        let child_tile = get_tile_by_path(&root, &[0]);
        assert!(child_tile.is_some());
        assert_eq!(child_tile.unwrap().geometric_error, 50.0);

        // 无效路径
        let invalid = get_tile_by_path(&root, &[1]);
        assert!(invalid.is_none());
    }

    #[test]
    fn test_distance_to_tile() {
        let camera = create_test_camera();
        let tile = Tile {
            bounding_volume: BoundingVolume::from_sphere(DVec3::ZERO, 100.0),
            geometric_error: 10.0,
            refine: None,
            transform: None,
            content: None,
            contents: None,
            children: vec![],
            viewer_request_volume: None,
            extras: None,
        };

        let distance = compute_distance_to_tile(&camera, &tile, &Ellipsoid::WGS84);
        // 相机在 (0, 0, 1000)，球体位于原点，半径 100
        // 距离 = 1000 - 100 = 900
        assert!((distance - 900.0).abs() < 1e-10);
    }

    #[test]
    fn test_add_refinement_mode() {
        let mut root = create_test_tile(1000.0, true);
        root.refine = Some(TileRefine::Add);

        let camera = create_test_camera();
        let context = LodSelectionContext::default();

        let selected = select_tiles(&root, &camera, &context, &Ellipsoid::WGS84);

        // 采用 ADD 细化时，父瓦片和子瓦片都应渲染
        assert!(selected.iter().any(|t| t.path.is_empty())); // 父瓦片
        assert!(selected.iter().any(|t| t.path == vec![0])); // 子瓦片
    }
}
