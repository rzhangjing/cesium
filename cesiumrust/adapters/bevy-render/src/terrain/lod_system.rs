// legacy CesiumJS-port style debt (deferred.md #18); revisit at M13 lint-cleanup 或本文件在其里程碑被重写时
//! 地形 LOD 选择系统：基于相机与视口做四叉树遍历，得出本帧应加载/
//! 卸载/渲染的瓦片集合。
//!
//! [`terrain_lod_system`] 每帧把相机变换从 render 单位换算到 ECEF 米，
//! 构造四叉树并遍历，根据屏幕空间误差(SSE)细分； [`TerrainSelection`]
//! 作为输出资源供下游加载/卸载系统消费。
#![allow(clippy::unnecessary_cast)]
use bevy::prelude::*;
use cesium_geospatial::bounding::BoundingSphere;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_quadtree::traversal::{QuadtreeConfig, QuadtreePrimitive, QuadtreeTile, TileState};

use crate::components::CesiumTerrainTile;
use crate::resources::{GlobeConfig, METERS_PER_RENDER_UNIT};

/// 本帧地形瓦片选择结果（供加载/卸载系统消费）。
#[derive(Resource, Default)]
pub struct TerrainSelection {
    /// 需新加载的瓦片坐标。
    pub tiles_to_load: Vec<(u32, u32, u32)>,
    /// 需卸载的瓦片坐标。
    pub tiles_to_unload: Vec<(u32, u32, u32)>,
    /// 本帧最终应渲染的瓦片坐标。
    pub active_tiles: Vec<(u32, u32, u32)>,
    /// 帧计数（递增）。
    pub frame_number: u64,
}

impl TerrainSelection {
    /// 清空三个瓦片列表（不改 frame_number）。
    pub fn clear(&mut self) {
        self.tiles_to_load.clear();
        self.tiles_to_unload.clear();
        self.active_tiles.clear();
    }
}

/// 构造根瓦片（两个半球：+X 与 -X）。
///
/// # 返回
/// 含两个根瓦片的列表。
fn create_root_tiles() -> Vec<QuadtreeTile> {
    // 取椭球三轴最大半径作为包围球半径。
    let ellipsoid = Ellipsoid::WGS84;
    let radii = ellipsoid.radii();
    let rx = radii[0];
    let ry = radii[1];
    let rz = radii[2];
    let max_radius = rx.max(ry).max(rz);

    vec![
        QuadtreeTile::new(
            0,
            0,
            0,
            BoundingSphere::new(glam::DVec3::new(rx as f64, 0.0, 0.0), max_radius as f64),
            500000.0,
        ),
        QuadtreeTile::new(
            1,
            0,
            0,
            BoundingSphere::new(glam::DVec3::new(-rx as f64, 0.0, 0.0), max_radius as f64),
            500000.0,
        ),
    ]
}

/// 计算瓦片 (x,y,level) 的包围球（由经纬度矩形角点得到）。
///
/// # 参数
/// - `ellipsoid`：参考椭球
/// - `x`/`y`/`level`：瓦片坐标与层级
///
/// # 返回
/// 以对角线一半为半径、中心为中点经纬的包围球。
fn tile_sphere(ellipsoid: &Ellipsoid, x: u32, y: u32, level: u32) -> BoundingSphere {
    // 四叉树（CesiumJS）语义：level L 有 2^(L+1) 列 x 2^L 行，
    // 所以每 level 分辨率翻倍。指数被钳位到 31 以
    // 保护 `u32` 移位：仅靠 `saturating_add`仍会让 level >= 31
    // 溢出移位（debug panic / release 回绕到 0 → 除零 NaN）。
    // `.max(2)` 使 level-0 基础网格非退化（n=1 → 零半径）。
    let n = 2u32.pow(level.saturating_add(1).min(31)).max(2) as f64;
    let west = (x as f64 / n) * 360.0 - 180.0;
    let east = ((x as f64 + 1.0) / n) * 360.0 - 180.0;
    let south = (y as f64 / n) * 180.0 - 90.0;
    let north = ((y as f64 + 1.0) / n) * 180.0 - 90.0;

    let sw = ellipsoid.cartographic_to_cartesian(
        &cesium_geospatial::cartographic::Cartographic::from_degrees(west, south, 0.0),
    );
    let ne = ellipsoid.cartographic_to_cartesian(
        &cesium_geospatial::cartographic::Cartographic::from_degrees(east, north, 0.0),
    );
    let center_w = cesium_geospatial::cartographic::Cartographic::from_degrees(
        (west + east) / 2.0,
        (south + north) / 2.0,
        0.0,
    );
    let center = ellipsoid.cartographic_to_cartesian(&center_w);
    // 由角点西南/东北计算对角线长度的一半作为半径。
    let radius = (ne - sw).length() / 2.0;
    BoundingSphere::new(center, radius)
}

/// 地形 LOD 主系统：遍历四叉树并写入 [`TerrainSelection`]。
///
/// # 参数
/// - `camera_query`：相机、全局变换与投影
/// - `window_query`：窗口（提供视口高度）
/// - `config`：地球配置（可选，缺失则直接返回）
/// - `terrain_query`：现有地形瓦片组件
/// - `selection`：选择结果（可写，每帧重置）
pub fn terrain_lod_system(
    camera_query: Query<(&Camera, &GlobalTransform, &Projection)>,
    window_query: Query<&Window>,
    config: Option<Res<GlobeConfig>>,
    terrain_query: Query<&CesiumTerrainTile>,
    mut selection: ResMut<TerrainSelection>,
) {
    // 无配置时无法计算，直接返回。
    let config = match config {
        Some(c) => c,
        None => return,
    };
    // 每帧重置选择列表并递增帧号。
    selection.clear();
    selection.frame_number += 1;

    let window = match window_query.get_single() {
        Ok(w) => w,
        Err(_) => return,
    };
    // 取视口像素高度作为 SSE 计算基准。
    let viewport_height = window.physical_height() as f64;

    let (_camera, transform, projection) = match camera_query.get_single() {
        Ok(c) => c,
        Err(_) => return,
    };

    // 相机 transform 以 render 单位表示（一个地球半径的 globe 约为 ~1）；下面的
    // tile 包围球以 ECEF 米表示（~6.4e6）。把相机转换
    // 到米，以便相机-tile 距离——从而屏幕空间误差——是
    // 有意义的。否则 ~6.4e6 的偏移会主导距离，SSE 约为
    // ~常量，四叉树从不细化到超过根 tile（terrain
    // “only loaded=2”症状）。镜像 tileset/traversal_system.rs。
    let t = transform.translation();
    let camera_position = glam::DVec3::new(
        t.x as f64 * METERS_PER_RENDER_UNIT,
        t.y as f64 * METERS_PER_RENDER_UNIT,
        t.z as f64 * METERS_PER_RENDER_UNIT,
    );

    // 垂直视场角（仅透视投影有效，否则用默认 45°）。
    let fov_y = match projection {
        Projection::Perspective(p) => p.fov as f64,
        _ => std::f64::consts::FRAC_PI_4,
    };

    let roots = create_root_tiles();
    let ellipsoid = config.ellipsoid;

    // 用当前配置构造四叉树（固定最大 SSE=16、最大层级=18）。
    let quadtree = QuadtreePrimitive::new(
        roots,
        QuadtreeConfig {
            maximum_screen_space_error: 16.0,
            maximum_level: 18,
            minimum_level: 0,
            ..Default::default()
        },
    );

    // 逐瓦片回调：计算包围球并按层级衰减几何误差。
    let result = quadtree.traverse(camera_position, viewport_height, fov_y, &|x, y, level| {
        let sphere = tile_sphere(&ellipsoid, x, y, level);
        Some(QuadtreeTile {
            x,
            y,
            level,
            bounding_sphere: sphere,
            geometric_error: 500000.0 / (2u64.pow(level) as f64 + 1.0),
            has_content: true,
            refineable: level < 18,
            state: TileState::Unloaded,
        })
    });

    // 现有瓦片集合（用于求新增/卸载差集）。
    let existing: Vec<(u32, u32, u32)> = terrain_query
        .iter()
        .map(|t| (t.x, t.y, t.level))
        .collect();

    // 本帧需渲染的新瓦片集合。
    let new_tiles: Vec<(u32, u32, u32)> = result
        .tiles_to_render
        .iter()
        .map(|t| (t.x, t.y, t.level))
        .collect();

    // 新增：在渲染集但不在现有集中的瓦片需加载。
    for tile in &result.tiles_to_render {
        let key = (tile.x, tile.y, tile.level);
        if !existing.contains(&key) {
            selection.tiles_to_load.push(key);
        }
    }

    // 卸载：在现有集但不在新渲染集中的瓦片需卸载。
    for &existing_tile in &existing {
        if !new_tiles.contains(&existing_tile) {
            selection.tiles_to_unload.push(existing_tile);
        }
    }

    // 最终活跃集即本帧新渲染集。
    selection.active_tiles = new_tiles;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 验证 clear 会清空列表。
    fn test_terrain_selection_clear() {
        let mut s = TerrainSelection::default();
        s.tiles_to_load.push((0, 0, 0));
        s.active_tiles.push((0, 0, 0));
        s.clear();
        assert!(s.tiles_to_load.is_empty());
        assert!(s.active_tiles.is_empty());
    }

    #[test]
    /// 验证 level-0 瓦片包围球半径为正。
    fn test_tile_sphere_level_0() {
        let ellipsoid = Ellipsoid::WGS84;
        let sphere = tile_sphere(&ellipsoid, 0, 0, 0);
        assert!(sphere.radius > 0.0);
    }

    #[test]
    /// 验证层级越高（更细分）包围球半径越小。
    fn test_tile_sphere_level_1() {
        let ellipsoid = Ellipsoid::WGS84;
        let sphere0 = tile_sphere(&ellipsoid, 0, 0, 1);
        let sphere1 = tile_sphere(&ellipsoid, 0, 0, 0);
        assert!(sphere0.radius < sphere1.radius);
    }

    #[test]
    /// 验证根瓦片为两个且均为 level-0。
    fn test_create_root_tiles() {
        let roots = create_root_tiles();
        assert_eq!(roots.len(), 2);
        assert_eq!(roots[0].level, 0);
        assert_eq!(roots[1].level, 0);
    }
}
