//! Ellipsoid terrain mesh generation for globe tiles.
//!
//! Mirrors the CesiumJS `GlobeSurfaceTileProvider` → `TerrainMesh` contract:
//! each rendered quadtree tile carries per-vertex `position3DAndHeight`
//! (vec4: xyz ellipsoid-surface position, w height) and
//! `textureCoordAndEncodedNormals` (vec4: .xy texture coordinates) streams,
//! consumed by `globe_vs.wgsl`.
//!
//! DEVIATION (B4-3): CesiumJS builds the mesh from a heightmap/quantized
//! grid (`createTerrainMeshData` with skirts, encoded normals, water-mask
//! texture coordinates and a tile-center relative encoding). This batch
//! generates a simplified longitude/latitude grid on the ellipsoid surface
//! (heights zero, no skirts, no encoded normals); the vertex stream layout
//! matches the original so the B4-5 terrain upgrade can swap the mesh
//! generator without touching the renderer wiring.
//!
//! LOD seam discipline (cesiumrust pitfall checkpoint): edge vertices are
//! placed exactly on the tile rectangle boundaries from the same longitude/
//! latitude parametrization, so adjacent tiles share identical border
//! positions at every level — no cracks when `maximum_screen_space_error`
//! stops refinement at mixed levels.

use cesium_core::cartographic::Cartographic;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::rectangle::Rectangle;

/// Default grid segments per tile edge. 16 → 17×17 vertices, 512 triangles —
/// close to the CesiumJS heightmap grid density (65×65 quantized) while
/// keeping the per-frame upload trivial for the smoke path.
pub const DEFAULT_GRID_SEGMENTS: u32 = 16;

/// CPU-side geometry for one globe tile, ready for vertex/index upload.
pub struct GlobeTileGeometry {
    /// `position3DAndHeight` stream: 4 f32 per vertex (xyz on the ellipsoid,
    /// w = height = 0 for the ellipsoid terrain).
    pub positions: Vec<f32>,
    /// `textureCoordAndEncodedNormals` stream: 4 f32 per vertex (uv texture
    /// coordinates with v = 0 at the south edge, z/w unused).
    pub texture_coordinates: Vec<f32>,
    /// Triangle indices (CCW front faces viewed from outside the globe).
    pub indices: Vec<u32>,
    /// Number of vertices.
    pub vertex_count: u32,
    /// Number of indices.
    pub index_count: u32,
}

/// Generates a longitude/latitude grid over `rectangle` on `ellipsoid`.
///
/// Texture coordinate convention (matches CesiumJS terrain UVs):
/// `u = 0` at west, `u = 1` at east, `v = 0` at south, `v = 1` at north.
pub fn create_ellipsoid_grid(
    rectangle: &Rectangle,
    ellipsoid: &Ellipsoid,
    segments: u32,
) -> GlobeTileGeometry {
    let segments = segments.max(1);
    let rows = segments as usize;
    let cols = segments as usize;
    let vertex_count = (rows + 1) * (cols + 1);

    let mut positions: Vec<f32> = Vec::with_capacity(vertex_count * 4);
    let mut texture_coordinates: Vec<f32> = Vec::with_capacity(vertex_count * 4);

    for row in 0..=rows {
        let v = row as f64 / rows as f64;
        let latitude = rectangle.south + (rectangle.north - rectangle.south) * v;
        for col in 0..=cols {
            let u = col as f64 / cols as f64;
            let longitude = rectangle.west + (rectangle.east - rectangle.west) * u;

            let cartographic = Cartographic {
                longitude,
                latitude,
                height: 0.0,
            };
            let mut cartesian = cesium_core::cartesian3::Cartesian3::default();
            ellipsoid.cartographic_to_cartesian(&cartographic, &mut cartesian);

            positions.push(cartesian.x as f32);
            positions.push(cartesian.y as f32);
            positions.push(cartesian.z as f32);
            positions.push(0.0); // height (ellipsoid terrain)

            texture_coordinates.push(u as f32);
            texture_coordinates.push(v as f32);
            texture_coordinates.push(0.0);
            texture_coordinates.push(0.0);
        }
    }

    // Two triangles per grid cell. Winding is CCW when viewed from outside
    // the globe (north-west → south-west → north-east, then south-west →
    // south-east → north-east), matching the default `FrontFace::Ccw` +
    // back-face culling render state.
    let stride = (cols + 1) as u32;
    let mut indices: Vec<u32> = Vec::with_capacity(rows * cols * 6);
    for row in 0..rows as u32 {
        for col in 0..cols as u32 {
            let north_west = (row + 1) * stride + col;
            let south_west = row * stride + col;
            let north_east = (row + 1) * stride + col + 1;
            let south_east = row * stride + col + 1;
            indices.extend_from_slice(&[north_west, south_west, north_east]);
            indices.extend_from_slice(&[south_west, south_east, north_east]);
        }
    }

    GlobeTileGeometry {
        positions,
        texture_coordinates,
        indices,
        vertex_count: vertex_count as u32,
        index_count: (rows * cols * 6) as u32,
    }
}
