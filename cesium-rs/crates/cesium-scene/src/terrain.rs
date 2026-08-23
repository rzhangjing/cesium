//! Ported from `packages/engine/Source/Scene/Terrain.js`.
//!
//! Terrain data for a tile, including heightmap/mesh data.

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::rectangle::Rectangle;

/// Terrain data for a tile, including heightmap/mesh data.
///
/// In CesiumJS, Terrain holds the vertex data, indices, and metadata for a
/// single terrain tile. It is produced by the TerrainProvider and consumed
/// by GlobeSurfaceTileProvider for rendering.
pub struct Terrain {
    /// The rectangle covered by this terrain.
    pub rectangle: Rectangle,
    /// The bounding sphere of this terrain.
    pub bounding_sphere: BoundingSphere,
    /// The oriented bounding box (if computed).
    pub oriented_bounding_box: Option<()>, // Placeholder for OrientedBoundingBox
    /// The vertex positions (x, y, z interleaved).
    pub vertices: Vec<f64>,
    /// The index buffer.
    pub indices: Vec<u32>,
    /// The normal vectors (nx, ny, nz interleaved).
    pub normals: Vec<f64>,
    /// The minimum height in this terrain.
    pub minimum_height: f64,
    /// The maximum height in this terrain.
    pub maximum_height: f64,
    /// Whether this terrain has a water mask.
    pub has_water_mask: bool,
    /// The water mask data.
    pub water_mask: Option<Vec<u8>>,
    /// Whether vertex normals are included.
    pub has_vertex_normals: bool,
    /// Whether this terrain includes skirts (vertical extensions for seam hiding).
    pub has_skirts: bool,
}

impl Terrain {
    /// Creates a new empty Terrain.
    pub fn new(rectangle: Rectangle) -> Self {
        Self {
            rectangle,
            bounding_sphere: BoundingSphere::default(),
            oriented_bounding_box: None,
            vertices: Vec::new(),
            indices: Vec::new(),
            normals: Vec::new(),
            minimum_height: 0.0,
            maximum_height: 0.0,
            has_water_mask: false,
            water_mask: None,
            has_vertex_normals: false,
            has_skirts: false,
        }
    }

    /// Returns the number of vertices.
    pub fn vertex_count(&self) -> usize {
        if self.vertices.is_empty() { 0 } else { self.vertices.len() / 3 }
    }

    /// Returns the number of triangles.
    pub fn triangle_count(&self) -> usize {
        if self.indices.is_empty() { 0 } else { self.indices.len() / 3 }
    }
}

impl Default for Terrain {
    fn default() -> Self {
        Self::new(Rectangle::default())
    }
}
