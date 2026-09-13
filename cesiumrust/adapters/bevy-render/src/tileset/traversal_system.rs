use bevy::prelude::*;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_tileset::lod_selection::{
    CameraState, LodSelectionContext, SelectedTile,
};
use cesium_tileset::traversal::{TraversalContext, TraversalStrategy};

use super::loader::LoadedTileset;

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

    selection.clear();
    selection.frame_number += 1;

    let tileset_json = match &loaded.tileset_json {
        Some(ts) => ts,
        None => return,
    };

    let camera_state = match get_camera_state(&camera_query, &window_query) {
        Some(cs) => cs,
        None => return,
    };

    let mut ctx = TraversalContext::default();
    ctx.lod_context = LodSelectionContext {
        maximum_screen_space_error: loaded.state.maximum_screen_space_error,
        cull_with_frustum: true,
        skip_level_of_detail: false,
    };
    ctx.strategy = TraversalStrategy::Base;

    let ellipsoid = Ellipsoid::WGS84;
    let result = cesium_tileset::traversal::traverse(
        &tileset_json.root,
        &camera_state,
        &ctx,
        &ellipsoid,
    );

    let prev_tiles: Vec<Vec<usize>> = selection
        .selected_tiles
        .iter()
        .map(|t| t.path.clone())
        .collect();

    let new_tile_paths: Vec<Vec<usize>> = result
        .selected_tiles
        .iter()
        .map(|t| t.path.clone())
        .collect();

    for tile in &result.selected_tiles {
        if !prev_tiles.contains(&tile.path) {
            selection.tiles_to_load.push(tile.path.clone());
        }
    }

    for prev in &prev_tiles {
        if !new_tile_paths.contains(prev) {
            selection.tiles_to_unload.push(prev.clone());
        }
    }

    selection.selected_tiles = result.selected_tiles;
}

fn get_camera_state(
    camera_query: &Query<(&Camera, &GlobalTransform, &Projection)>,
    window_query: &Query<&Window>,
) -> Option<CameraState> {
    let window = window_query.get_single().ok()?;
    let (_camera, transform, projection) = camera_query.get_single().ok()?;

    let viewport_height = window.physical_height() as f64;

    let position = glam::DVec3::new(
        transform.translation().x as f64,
        transform.translation().y as f64,
        transform.translation().z as f64,
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
