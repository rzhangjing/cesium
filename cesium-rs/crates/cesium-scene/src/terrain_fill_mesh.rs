//! Ported from `packages/engine/Source/Scene/TerrainFillMesh.js`.
//!
//! A mesh used to fill gaps in terrain data where no actual terrain tiles are available.

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::rectangle::Rectangle;

/// A mesh used to fill gaps in terrain data where no actual terrain tiles are available.
///
/// Terrain fill meshes are generated when a tile has no terrain data but its
/// children do, providing a smooth visual transition.
pub struct TerrainFillMesh {
    /// The rectangle covered by this fill mesh.
    pub rectangle: Rectangle,
    /// The bounding sphere of this fill mesh.
    pub bounding_sphere: BoundingSphere,
    /// The vertex positions.
    pub positions: Vec<f64>,
    /// The texture coordinates.
    pub texture_coordinates: Vec<f64>,
    /// The indices.
    pub indices: Vec<u32>,
    /// The normal vectors.
    pub normals: Vec<f64>,
    /// The minimum height of the fill.
    pub minimum_height: f64,
    /// The maximum height of the fill.
    pub maximum_height: f64,
}

impl TerrainFillMesh {
    /// Creates a new TerrainFillMesh.
    pub fn new(rectangle: Rectangle) -> Self {
        Self {
            rectangle,
            bounding_sphere: BoundingSphere::default(),
            positions: Vec::new(),
            texture_coordinates: Vec::new(),
            indices: Vec::new(),
            normals: Vec::new(),
            minimum_height: 0.0,
            maximum_height: 0.0,
        }
    }
}

impl Default for TerrainFillMesh {
    fn default() -> Self {
        Self::new(Rectangle::default())
    }
}
