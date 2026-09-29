use bevy::prelude::*;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_tileset::lod_selection::{
    CameraState, LodSelectionContext, SelectedTile,
};
use cesium_tileset::traversal::{TraversalContext, TraversalStrategy};

use super::loader::LoadedTileset;
use crate::resources::METERS_PER_RENDER_UNIT;

#[derive(Resource, Default)]
pub struct TileSelection {
    pub tiles_to_load: Vec<Vec<usize>>,
    pub tiles_to_unload: Vec<Vec<usize>>,
    pub selected_tiles: Vec<SelectedTile>,
    pub frame_number: u64,
}

impl TileSelection {
    pub fn clear(&mut self) {
        self.tiles_to_load.clear();
        self.tiles_to_unload.clear();
        self.selected_tiles.clear();
    }

    /// 开始一帧：快照上一帧的选择集，*然后*清空
    /// 逐帧队列并递增帧计数。
    ///
    /// 顺序很重要：在 `clear()` 之后读 `selected_tiles` 会得到一个
    /// 空集，于是每个可见 tile 都会在每帧看起来是新被选中的，从而被
    /// 永久重新请求（本函数所取代的那个无限重载 bug）。
    pub fn begin_frame(&mut self) -> Vec<Vec<usize>> {
        let prev_tiles = self
            .selected_tiles
            .iter()
            .map(|t| t.path.clone())
            .collect();

        self.clear();
        self.frame_number += 1;

        prev_tiles
    }

    /// 把本帧的 `selected` tile 与 `prev_tiles` 做 diff，并只把差异填入
    /// load/unload 队列。
    pub fn finish_frame(&mut self, prev_tiles: &[Vec<usize>], selected: Vec<SelectedTile>) {
        let new_tile_paths: Vec<Vec<usize>> =
            selected.iter().map(|t| t.path.clone()).collect();

        for tile in &selected {
            if !prev_tiles.contains(&tile.path) {
                self.tiles_to_load.push(tile.path.clone());
            }
        }

        for prev in prev_tiles {
            if !new_tile_paths.contains(prev) {
                self.tiles_to_unload.push(prev.clone());
            }
        }

        self.selected_tiles = selected;
    }
}

pub fn tileset_traversal_system(
    camera_query: Query<(&Camera, &GlobalTransform, &Projection)>,
    window_query: Query<&Window>,
    loaded: Option<Res<LoadedTileset>>,
    mut selection: ResMut<TileSelection>,
) {
    let loaded = match loaded {
        Some(l) => l,
        None => return,
    };

    // 在清空之前快照上一帧的选择集；下面的早返回
    // 保留了原先“总在帧首清空”的行为。
    let prev_tiles = selection.begin_frame();

    let tileset_json = match &loaded.tileset_json {
        Some(ts) => ts,
        None => return,
    };

    let camera_state = match get_camera_state(&camera_query, &window_query) {
        Some(cs) => cs,
        None => return,
    };

    let ctx = TraversalContext {
        lod_context: LodSelectionContext {
            maximum_screen_space_error: loaded.state.maximum_screen_space_error,
            cull_with_frustum: true,
            skip_level_of_detail: false,
        },
        strategy: TraversalStrategy::Base,
        ..Default::default()
    };

    let ellipsoid = Ellipsoid::WGS84;
    let result = cesium_tileset::traversal::traverse(
        &tileset_json.root,
        &camera_state,
        &ctx,
        &ellipsoid,
    );

    selection.finish_frame(&prev_tiles, result.selected_tiles);
}

fn get_camera_state(
    camera_query: &Query<(&Camera, &GlobalTransform, &Projection)>,
    window_query: &Query<&Window>,
) -> Option<CameraState> {
    let window = window_query.get_single().ok()?;
    let (_camera, transform, projection) = camera_query.get_single().ok()?;

    let viewport_height = window.physical_height() as f64;

    // 相机位置以 render unit 为单位；转为 ECEF 米以供
    // 期望包围体以米为单位的领域 traversal。
    let t = transform.translation();
    let position = glam::DVec3::new(
        t.x as f64 * METERS_PER_RENDER_UNIT,
        t.y as f64 * METERS_PER_RENDER_UNIT,
        t.z as f64 * METERS_PER_RENDER_UNIT,
    );

    let forward = transform.forward();
    let direction = glam::DVec3::new(
        forward.x as f64,
        forward.y as f64,
        forward.z as f64,
    );

    let up = transform.up();
    let up_dir = glam::DVec3::new(up.x as f64, up.y as f64, up.z as f64);

    let fov_y = match projection {
        Projection::Perspective(persp) => persp.fov as f64,
        _ => std::f64::consts::FRAC_PI_4,
    };

    Some(CameraState::new(position, direction, up_dir, fov_y, viewport_height))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_tileset::tileset::TilesetJson;
    use glam::DVec3;

    fn create_test_tileset() -> TilesetJson {
        let json = r#"{
            "asset": { "version": "1.0" },
            "geometricError": 1000,
            "root": {
                "boundingVolume": { "sphere": [0, 0, 0, 6378137] },
                "geometricError": 500,
                "content": { "uri": "root.b3dm" },
                "children": [
                    {
                        "boundingVolume": { "sphere": [1000000, 0, 0, 100000] },
                        "geometricError": 100,
                        "content": { "uri": "child0.b3dm" }
                    },
                    {
                        "boundingVolume": { "sphere": [-1000000, 0, 0, 100000] },
                        "geometricError": 100,
                        "content": { "uri": "child1.b3dm" }
                    }
                ]
            }
        }"#;
        TilesetJson::from_json(json).unwrap()
    }

    #[test]
    fn test_parse_tileset_with_children() {
        let tileset = create_test_tileset();
        assert_eq!(tileset.root.children.len(), 2);
        assert_eq!(tileset.root.geometric_error, 500.0);
    }

    #[test]
    fn test_tile_selection_clear() {
        let mut selection = TileSelection::default();
        selection.selected_tiles.push(SelectedTile {
            path: vec![0],
            result: cesium_tileset::lod_selection::TileSelectionResult::Render,
            screen_space_error: 10.0,
            distance_to_camera: 1000.0,
        });
        selection.clear();
        assert!(selection.selected_tiles.is_empty());
        assert!(selection.tiles_to_load.is_empty());
    }

    fn selected(path: Vec<usize>) -> SelectedTile {
        SelectedTile {
            path,
            result: cesium_tileset::lod_selection::TileSelectionResult::Render,
            screen_space_error: 10.0,
            distance_to_camera: 1000.0,
        }
    }

    /// 针对无限重载 bug 的回归测试：上一帧的快照此前被在
    /// `clear()` *之后*读取，所以 `prev_tiles` 总是为空且
    /// 每个可见 tile 在每帧都看起来是新被选中的。
    #[test]
    fn test_prev_tiles_snapshot_before_clear() {
        let mut selection = TileSelection::default();

        // 帧 1：此前没有任何东西被选中，所以一切皆为新增。
        let prev = selection.begin_frame();
        assert!(prev.is_empty(), "frame 1 has no previous selection");
        assert_eq!(selection.frame_number, 1);
        selection.finish_frame(&prev, vec![selected(vec![0]), selected(vec![0, 1])]);
        assert_eq!(selection.tiles_to_load, vec![vec![0], vec![0, 1]]);
        assert!(selection.tiles_to_unload.is_empty());

        // loader 是 `tiles_to_load` 的唯一消费者；模拟这一点。
        selection.tiles_to_load.clear();

        // 帧 2：相同的选择集。快照必须仍报告上一帧
        // 的 tile，即使 `begin_frame` 将它们清空，否则每个
        // tile 都会被永久重新请求。
        let prev = selection.begin_frame();
        assert_eq!(
            prev,
            vec![vec![0], vec![0, 1]],
            "snapshot must be taken before clear()"
        );
        selection.finish_frame(&prev, vec![selected(vec![0]), selected(vec![0, 1])]);
        assert!(
            selection.tiles_to_load.is_empty(),
            "unchanged selection must not trigger a reload"
        );
        assert!(selection.tiles_to_unload.is_empty());

        // 帧 3：一个 tile 离开选择集 -> 恰好那个 tile 被卸载。
        let prev = selection.begin_frame();
        selection.finish_frame(&prev, vec![selected(vec![0, 1])]);
        assert!(selection.tiles_to_load.is_empty());
        assert_eq!(selection.tiles_to_unload, vec![vec![0]]);
        assert_eq!(selection.selected_tiles.len(), 1);
    }

    #[test]
    fn test_camera_state_computes_sse() {
        let camera = CameraState::new(
            DVec3::new(0.0, 0.0, 1000.0),
            DVec3::new(0.0, 0.0, -1.0),
            DVec3::new(0.0, 1.0, 0.0),
            std::f64::consts::FRAC_PI_4,
            1080.0,
        );

        let sse = camera.compute_screen_space_error(100.0, 500.0);
        assert!(sse > 0.0);
        assert!(sse < 1000.0);
    }
}
