//! Ported from `packages/engine/Source/Core/TerrainProvider.js`.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::ellipsoid::Ellipsoid;

/// Provides terrain or other geometry for the surface of an ellipsoid.
pub trait TerrainProvider {
    /// Gets the tiling scheme used by the provider.
    fn tiling_scheme(&self) -> &dyn crate::tiling_scheme::TilingScheme;

    /// Gets a value indicating whether or not the provider includes a water mask.
    fn has_water_mask(&self) -> bool;

    /// Gets a value indicating whether or not the requested tiles include vertex normals.
    fn has_vertex_normals(&self) -> bool;

    /// Gets the maximum geometric error allowed in a tile at a given level.
    fn get_level_maximum_geometric_error(&self, level: i32) -> f64;

    /// Determines whether data for a tile is available.
    fn get_tile_data_available(&self, x: i32, y: i32, level: i32) -> Option<bool>;
}

/// Quality of terrain created from heightmaps.
pub const HEIGHTMAP_TERRAIN_QUALITY: f64 = 0.25;

/// Determines an appropriate geometric error estimate for heightmap terrain.
pub fn get_estimated_level_zero_geometric_error_for_a_heightmap(
    ellipsoid: &Ellipsoid,
    tile_image_width: f64,
    number_of_tiles_at_level_zero: i32,
) -> f64 {
    (ellipsoid.maximum_radius()
        * 2.0
        * std::f64::consts::PI
        * HEIGHTMAP_TERRAIN_QUALITY)
        / (tile_image_width * number_of_tiles_at_level_zero as f64)
}

/// Edge indices for a regular grid.
pub struct RegularGridEdgeIndices {
    pub west_indices_south_to_north: Vec<i32>,
    pub south_indices_east_to_west: Vec<i32>,
    pub east_indices_north_to_south: Vec<i32>,
    pub north_indices_west_to_east: Vec<i32>,
}

/// Regular grid indices and edge indices.
pub struct RegularGridIndicesAndEdgeIndices {
    pub indices: Vec<u32>,
    pub west_indices_south_to_north: Vec<i32>,
    pub south_indices_east_to_west: Vec<i32>,
    pub east_indices_north_to_south: Vec<i32>,
    pub north_indices_west_to_east: Vec<i32>,
}

/// Regular grid indices with skirts and edge indices.
pub struct RegularGridAndSkirtIndicesAndEdgeIndices {
    pub indices: Vec<u32>,
    pub west_indices_south_to_north: Vec<i32>,
    pub south_indices_east_to_west: Vec<i32>,
    pub east_indices_north_to_south: Vec<i32>,
    pub north_indices_west_to_east: Vec<i32>,
    pub index_count_without_skirts: usize,
}

static REGULAR_GRID_INDICES_CACHE: Mutex<Option<HashMap<(i32, i32), Vec<u32>>>> =
    Mutex::new(None);

/// Gets a list of indices for a triangle mesh representing a regular grid.
pub fn get_regular_grid_indices(width: i32, height: i32) -> Vec<u32> {
    let mut cache = REGULAR_GRID_INDICES_CACHE.lock().unwrap();
    let cache_map = cache.get_or_insert_with(HashMap::new);
    let key = (width, height);

    if let Some(indices) = cache_map.get(&key) {
        return indices.clone();
    }

    let count = ((width - 1) * (height - 1) * 6) as usize;
    let mut indices = vec![0u32; count];
    add_regular_grid_indices(width, height, &mut indices, 0);
    cache_map.insert(key, indices.clone());
    indices
}

static REGULAR_GRID_AND_EDGE_CACHE: Mutex<
    Option<HashMap<(i32, i32), RegularGridIndicesAndEdgeIndices>>,
> = Mutex::new(None);

/// Gets regular grid indices and edge indices.
pub fn get_regular_grid_indices_and_edge_indices(
    width: i32,
    height: i32,
) -> RegularGridIndicesAndEdgeIndices {
    let mut cache = REGULAR_GRID_AND_EDGE_CACHE.lock().unwrap();
    let cache_map = cache.get_or_insert_with(HashMap::new);
    let key = (width, height);

    if let Some(result) = cache_map.get(&key) {
        return RegularGridIndicesAndEdgeIndices {
            indices: result.indices.clone(),
            west_indices_south_to_north: result.west_indices_south_to_north.clone(),
            south_indices_east_to_west: result.south_indices_east_to_west.clone(),
            east_indices_north_to_south: result.east_indices_north_to_south.clone(),
            north_indices_west_to_east: result.north_indices_west_to_east.clone(),
        };
    }

    let indices = get_regular_grid_indices(width, height);
    let edge_indices = get_edge_indices(width, height);

    let result = RegularGridIndicesAndEdgeIndices {
        indices,
        west_indices_south_to_north: edge_indices.west_indices_south_to_north.clone(),
        south_indices_east_to_west: edge_indices.south_indices_east_to_west.clone(),
        east_indices_north_to_south: edge_indices.east_indices_north_to_south.clone(),
        north_indices_west_to_east: edge_indices.north_indices_west_to_east.clone(),
    };

    cache_map.insert(
        key,
        RegularGridIndicesAndEdgeIndices {
            indices: result.indices.clone(),
            west_indices_south_to_north: result.west_indices_south_to_north.clone(),
            south_indices_east_to_west: result.south_indices_east_to_west.clone(),
            east_indices_north_to_south: result.east_indices_north_to_south.clone(),
            north_indices_west_to_east: result.north_indices_west_to_east.clone(),
        },
    );

    result
}

/// Calculates the number of skirt vertices.
pub fn get_skirt_vertex_count(
    west: &[i32],
    south: &[i32],
    east: &[i32],
    north: &[i32],
) -> usize {
    west.len() + south.len() + east.len() + north.len()
}

/// Computes the number of skirt indices.
pub fn get_skirt_index_count(skirt_vertex_count: usize) -> usize {
    (skirt_vertex_count - 4) * 2 * 3
}

/// Computes the number of skirt indices with filled corners.
pub fn get_skirt_index_count_with_filled_corners(skirt_vertex_count: usize) -> usize {
    ((skirt_vertex_count - 4) * 2 + 4) * 3
}

/// Adds skirt indices.
pub fn add_skirt_indices(
    west_indices: &[i32],
    south_indices: &[i32],
    east_indices: &[i32],
    north_indices: &[i32],
    vertex_count: usize,
    indices: &mut [u32],
    offset: usize,
) {
    let mut vertex_index = vertex_count;
    let mut off = add_skirt_indices_for_edge(west_indices, vertex_index, indices, offset);
    vertex_index += west_indices.len();
    off = add_skirt_indices_for_edge(south_indices, vertex_index, indices, off);
    vertex_index += south_indices.len();
    off = add_skirt_indices_for_edge(east_indices, vertex_index, indices, off);
    vertex_index += east_indices.len();
    add_skirt_indices_for_edge(north_indices, vertex_index, indices, off);
}

fn get_edge_indices(width: i32, height: i32) -> RegularGridEdgeIndices {
    let mut west = vec![0i32; height as usize];
    let mut south = vec![0i32; width as usize];
    let mut east = vec![0i32; height as usize];
    let mut north = vec![0i32; width as usize];

    for i in 0..width {
        north[i as usize] = i;
        south[i as usize] = width * height - 1 - i;
    }

    for i in 0..height {
        east[i as usize] = (i + 1) * width - 1;
        west[i as usize] = (height - i - 1) * width;
    }

    RegularGridEdgeIndices {
        west_indices_south_to_north: west,
        south_indices_east_to_west: south,
        east_indices_north_to_south: east,
        north_indices_west_to_east: north,
    }
}

fn add_regular_grid_indices(width: i32, height: i32, indices: &mut [u32], offset: usize) {
    let mut index = 0i32;
    let mut off = offset;
    for _j in 0..height - 1 {
        for _i in 0..width - 1 {
            let upper_left = index;
            let lower_left = upper_left + width;
            let lower_right = lower_left + 1;
            let upper_right = upper_left + 1;

            indices[off] = upper_left as u32;
            indices[off + 1] = lower_left as u32;
            indices[off + 2] = upper_right as u32;
            indices[off + 3] = upper_right as u32;
            indices[off + 4] = lower_left as u32;
            indices[off + 5] = lower_right as u32;
            off += 6;
            index += 1;
        }
        index += 1;
    }
}

fn add_skirt_indices_for_edge(
    edge_indices: &[i32],
    vertex_index: usize,
    indices: &mut [u32],
    offset: usize,
) -> usize {
    let mut off = offset;
    let mut prev = edge_indices[0] as u32;
    let mut vi = vertex_index;

    for i in 1..edge_indices.len() {
        let idx = edge_indices[i] as u32;
        indices[off] = prev;
        indices[off + 1] = idx;
        indices[off + 2] = vi as u32;
        indices[off + 3] = vi as u32;
        indices[off + 4] = idx;
        indices[off + 5] = (vi + 1) as u32;
        off += 6;
        prev = idx;
        vi += 1;
    }
    off
}
