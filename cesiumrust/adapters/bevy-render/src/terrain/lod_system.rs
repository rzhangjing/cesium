use bevy::prelude::*;
use cesium_geospatial::bounding::BoundingSphere;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_quadtree::traversal::{QuadtreeConfig, QuadtreePrimitive, QuadtreeTile, TileState};

use crate::components::CesiumTerrainTile;
use crate::resources::GlobeConfig;

#[derive(Resource, Default)]
pub struct TerrainSelection {
    pub tiles_to_load: Vec<(u32, u32, u32)>,
    pub tiles_to_unload: Vec<(u32, u32, u32)>,
    pub active_tiles: Vec<(u32, u32, u32)>,
    pub frame_number: u64,
}

impl TerrainSelection {
    pub fn clear(&mut self) {
        self.tiles_to_load.clear();
        self.tiles_to_unload.clear();
        self.active_tiles.clear();
    }
}

fn create_root_tiles() -> Vec<QuadtreeTile> {
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

fn tile_sphere(ellipsoid: &Ellipsoid, x: u32, y: u32, level: u32) -> BoundingSphere {
    let n = 2u32.pow(level.max(1)) as f64;
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
    let radius = (ne - sw).length() / 2.0;
    BoundingSphere::new(center, radius)
}

pub fn terrain_lod_system(
    camera_query: Query<(&Camera, &GlobalTransform, &Projection)>,
    window_query: Query<&Window>,
    config: Option<Res<GlobeConfig>>,
    terrain_query: Query<&CesiumTerrainTile>,
    mut selection: ResMut<TerrainSelection>,
) {
    let config = match config {
        Some(c) => c,
        None => return,
    };
    selection.clear();
    selection.frame_number += 1;

    let window = match window_query.get_single() {
        Ok(w) => w,
        Err(_) => return,
    };
    let viewport_height = window.physical_height() as f64;

    let (_camera, transform, projection) = match camera_query.get_single() {
        Ok(c) => c,
        Err(_) => return,
    };

    let camera_position = glam::DVec3::new(
        transform.translation().x as f64,
        transform.translation().y as f64,
        transform.translation().z as f64,
    );

    let fov_y = match projection {
        Projection::Perspective(p) => p.fov as f64,
        _ => std::f64::consts::FRAC_PI_4,
    };

    let roots = create_root_tiles();
    let ellipsoid = config.ellipsoid;

    let quadtree = QuadtreePrimitive::new(
        roots,
        QuadtreeConfig {
            maximum_screen_space_error: 16.0,
            maximum_level: 18,
            minimum_level: 0,
            ..Default::default()
        },
    );

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

    let existing: Vec<(u32, u32, u32)> = terrain_query
        .iter()
        .map(|t| (t.x, t.y, t.level))
        .collect();

    let new_tiles: Vec<(u32, u32, u32)> = result
        .tiles_to_render
        .iter()
        .map(|t| (t.x, t.y, t.level))
        .collect();

    for tile in &result.tiles_to_render {
        let key = (tile.x, tile.y, tile.level);
        if !existing.contains(&key) {
            selection.tiles_to_load.push(key);
        }
    }

    for &existing_tile in &existing {
        if !new_tiles.contains(&existing_tile) {
            selection.tiles_to_unload.push(existing_tile);
        }
    }

    selection.active_tiles = new_tiles;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_selection_clear() {
        let mut s = TerrainSelection::default();
        s.tiles_to_load.push((0, 0, 0));
        s.active_tiles.push((0, 0, 0));
        s.clear();
        assert!(s.tiles_to_load.is_empty());
        assert!(s.active_tiles.is_empty());
    }

    #[test]
    fn test_tile_sphere_level_0() {
        let ellipsoid = Ellipsoid::WGS84;
        let sphere = tile_sphere(&ellipsoid, 0, 0, 0);
        assert!(sphere.radius > 0.0);
    }

    #[test]
    fn test_tile_sphere_level_1() {
        let ellipsoid = Ellipsoid::WGS84;
        let sphere0 = tile_sphere(&ellipsoid, 0, 0, 1);
        let sphere1 = tile_sphere(&ellipsoid, 0, 0, 0);
        assert!(sphere0.radius < sphere1.radius);
    }

    #[test]
    fn test_create_root_tiles() {
        let roots = create_root_tiles();
        assert_eq!(roots.len(), 2);
        assert_eq!(roots[0].level, 0);
        assert_eq!(roots[1].level, 0);
    }
}
