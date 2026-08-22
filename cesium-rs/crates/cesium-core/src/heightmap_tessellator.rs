//! Ported from `packages/engine/Source/Core/HeightmapTessellator.js`.
//!
//! Creates a mesh from a heightmap. Full implementation deferred to a later milestone
//! because it depends on `TerrainMesh`, `TerrainEncoding`, and `Ellipsoid` tessellation logic.

use crate::ellipsoid::Ellipsoid;
use crate::rectangle::Rectangle;

/// Tessellates heightmap data into a terrain mesh.
pub struct HeightmapTessellator;

impl HeightmapTessellator {
    /// Creates a terrain mesh from heightmap data.
    ///
    /// This is a skeleton; the full implementation requires `TerrainEncoding` encode/decode,
    /// skirt generation, and normal computation.
    pub fn fill_vertices(
        _rectangle: &Rectangle,
        _heights: &[f64],
        _width: i32,
        _height: i32,
        _skirt_height: f64,
        _ellipsoid: &Ellipsoid,
        _vertices: &mut [f32],
        _stride: usize,
    ) {
        // TODO: full implementation
    }

    /// Computes the bounding sphere for a terrain mesh.
    pub fn compute_bounding_sphere(
        _rectangle: &Rectangle,
        _vertices: &[f32],
        _stride: usize,
        _ellipsoid: &Ellipsoid,
    ) -> crate::bounding_sphere::BoundingSphere {
        // TODO: full implementation
        crate::bounding_sphere::BoundingSphere::default()
    }
}
