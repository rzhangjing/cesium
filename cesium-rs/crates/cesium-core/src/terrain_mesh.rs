//! Ported from `packages/engine/Source/Core/TerrainMesh.js`.

use crate::bounding_sphere::BoundingSphere;
use crate::cartesian3::Cartesian3;
use crate::oriented_bounding_box::OrientedBoundingBox;
use crate::rectangle::Rectangle;

/// A mesh plus related metadata for a single tile of terrain.
pub struct TerrainMesh {
    /// The center of the tile.
    pub center: Cartesian3,
    /// The vertex data: [X, Y, Z, H, U, V, ...].
    pub vertices: Vec<f32>,
    /// The number of components in each vertex.
    pub stride: usize,
    /// The indices describing how vertices form triangles.
    pub indices: Vec<u32>,
    /// Index count not including skirts.
    pub index_count_without_skirts: usize,
    /// Vertex count not including skirts.
    pub vertex_count_without_skirts: usize,
    /// The lowest height in the tile, in meters.
    pub minimum_height: f64,
    /// The highest height in the tile, in meters.
    pub maximum_height: f64,
    /// The rectangle, in radians, covered by this tile.
    pub rectangle: Rectangle,
    /// A bounding sphere that completely contains the tile.
    pub bounding_sphere_3d: BoundingSphere,
    /// The occludee point for horizon culling.
    pub occludee_point_in_scaled_space: Cartesian3,
    /// A bounding box that completely contains the tile.
    pub oriented_bounding_box: Option<OrientedBoundingBox>,
    /// Edge indices: west (S→N), south (E→W), east (N→S), north (W→E).
    pub west_indices_south_to_north: Vec<u32>,
    pub south_indices_east_to_west: Vec<u32>,
    pub east_indices_north_to_south: Vec<u32>,
    pub north_indices_west_to_east: Vec<u32>,
}
