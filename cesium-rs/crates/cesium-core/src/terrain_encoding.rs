//! Ported from `packages/engine/Source/Core/TerrainEncoding.js`.
//!
//! Encodes and decodes terrain mesh vertices.

/// Information about how a terrain mesh is encoded.
pub struct TerrainEncoding {
    /// Whether the encoding includes vertex normals.
    pub has_vertex_normals: bool,
    /// Whether the encoding includes water mask.
    pub has_water_mask: bool,
    /// The vertical exaggeration scale.
    pub exaggeration: f64,
    /// The height relative to which terrain is exaggerated.
    pub exaggeration_relative_height: f64,
    /// The stride (number of components per vertex).
    pub stride: usize,
}

impl TerrainEncoding {
    /// Creates a new TerrainEncoding.
    pub fn new(
        has_vertex_normals: bool,
        has_water_mask: bool,
        exaggeration: f64,
        exaggeration_relative_height: f64,
    ) -> Self {
        // Base stride: X, Y, Z, H, U, V = 6
        // + 3 for normals if has_vertex_normals
        // + 1 for water mask if has_water_mask
        let mut stride = 6;
        if has_vertex_normals {
            stride += 3;
        }
        if has_water_mask {
            stride += 1;
        }

        Self {
            has_vertex_normals,
            has_water_mask,
            exaggeration,
            exaggeration_relative_height,
            stride,
        }
    }
}
