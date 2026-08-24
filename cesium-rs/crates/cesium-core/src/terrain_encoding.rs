//! Ported from `packages/engine/Source/Core/TerrainEncoding.js`.
//!
//! Encodes and decodes terrain mesh vertices.

use crate::cartesian2::Cartesian2;

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
    ///
    /// Vertex layout: `[X, Y, Z, H, U, V]` followed, when
    /// `has_vertex_normals`, by the oct-encoded normal pair (`NX, NY`,
    /// mirroring the JS `NORMAL` attribute's 2 components).
    pub fn new(
        has_vertex_normals: bool,
        has_water_mask: bool,
        exaggeration: f64,
        exaggeration_relative_height: f64,
    ) -> Self {
        // Base stride: X, Y, Z, H, U, V = 6
        // + 2 for the oct-encoded normal pair if has_vertex_normals
        // + 1 for water mask if has_water_mask
        let mut stride = 6;
        if has_vertex_normals {
            stride += 2;
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

    /// Decodes the height of a vertex stored in a packed vertex buffer.
    ///
    /// Mirrors `TerrainEncoding.prototype.decodeHeight`
    /// (`buffer[index * stride + 3]`; the height slot follows the XYZ
    /// position components).
    pub fn decode_height(&self, vertices: &[f32], index: usize) -> f64 {
        vertices[index * self.stride + 3] as f64
    }

    /// Decodes the texture coordinates (u, v) of a vertex stored in a packed
    /// vertex buffer.
    ///
    /// Mirrors `TerrainEncoding.prototype.decodeTextureCoordinates`
    /// (`buffer[index * stride + 4]` / `+ 5`).
    pub fn decode_texture_coordinates<'a>(
        &self,
        vertices: &[f32],
        index: usize,
        result: &'a mut Cartesian2,
    ) -> &'a mut Cartesian2 {
        result.x = vertices[index * self.stride + 4] as f64;
        result.y = vertices[index * self.stride + 5] as f64;
        result
    }
}
