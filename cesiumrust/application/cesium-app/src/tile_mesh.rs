//! Per-tile ellipsoid mesh generation.
//!
//! Implements the CesiumJS `HeightmapTessellator.computeVertices` approach:
//! each tile (x, y, z) in a Web Mercator tiling scheme gets its own mesh patch
//! on the WGS84 ellipsoid surface, with UV coordinates normalized to [0,1]
//! within the tile's geographic extent.

use bevy::prelude::*;
use cesium_bevy_render::METERS_PER_RENDER_UNIT;

/// WGS84 semi-major axis (meters).
const EARTH_RADIUS: f64 = 6378137.0;

/// Component identifying a globe tile entity by its tiling scheme coordinates.
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlobeTile {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

/// Geographic rectangle in radians (west, south, east, north).
#[derive(Debug, Clone, Copy)]
pub struct GeoRectangle {
    pub west: f64,
    pub south: f64,
    pub east: f64,
    pub north: f64,
}

/// Computes the geographic extent of a Web Mercator tile.
///
/// Corresponds to CesiumJS `WebMercatorTilingScheme.tileXYToRectangle`:
/// - Global Mercator range: [-PI*R, PI*R] in both axes
/// - Tile width/height in meters = 2*PI*R / 2^z
/// - Unproject: lon = x_m / R, lat = PI/2 - 2*atan(exp(-y_m / R))
pub fn tile_xy_to_rectangle(x: u32, y: u32, z: u32) -> GeoRectangle {
    let num_tiles = 1u64 << z;
    let tile_size_meters = 2.0 * std::f64::consts::PI * EARTH_RADIUS / num_tiles as f64;

    // Native rectangle in meters (Web Mercator coordinates)
    let west_m = -std::f64::consts::PI * EARTH_RADIUS + x as f64 * tile_size_meters;
    let east_m = west_m + tile_size_meters;
    let north_m = std::f64::consts::PI * EARTH_RADIUS - y as f64 * tile_size_meters;
    let south_m = north_m - tile_size_meters;

    // Unproject from Web Mercator meters to geographic radians
    let one_over_r = 1.0 / EARTH_RADIUS;
    let west = west_m * one_over_r;
    let east = east_m * one_over_r;
    let north = std::f64::consts::FRAC_PI_2 - 2.0 * (-north_m * one_over_r).exp().atan();
    let south = std::f64::consts::FRAC_PI_2 - 2.0 * (-south_m * one_over_r).exp().atan();

    GeoRectangle {
        west,
        south,
        east,
        north,
    }
}

/// Fraction of a tile's extent by which the mesh overlaps its neighbors.
/// The overlap hides sub-pixel cracks between adjacent tiles; skirt vertices
/// are tucked slightly below the surface so the overlap never z-fights with
/// the neighbor's main surface.
const TILE_MARGIN: f64 = 0.012;
/// Radial tuck factor applied to skirt (overlap) vertices.
const SKIRT_TUCK: f64 = 0.9992;

/// Generates a Bevy Mesh for a single tile on the WGS84 ellipsoid surface.
///
/// Follows CesiumJS `HeightmapTessellator.computeVertices`:
/// - Vertices distributed across the tile's geographic extent
/// - UV: u = (lon - west) / (east - west), v = (lat - south) / (north - south)
/// - Positions: cartographic_to_cartesian on WGS84 ellipsoid
/// - Normals: geodetic surface normal (normalized position on ellipsoid)
/// - Triangle winding: counter-clockwise from outside (outward normals)
/// - A one-vertex overlap skirt per side (tucked below the surface) hides
///   rasterization cracks between neighboring tiles
///
/// # Arguments
/// * `x`, `y`, `z` - Tile coordinates in Web Mercator tiling scheme
/// * `segments` - Number of subdivisions per axis within the true tile extent
/// * `base_tuck` - Radial scale (< 1.0) placing this tile below finer LOD
///   levels so overlapping levels never z-fight (1.0 for the finest level)
pub fn create_tile_mesh(x: u32, y: u32, z: u32, segments: u32, base_tuck: f64) -> Mesh {
    let rect = tile_xy_to_rectangle(x, y, z);

    let width = rect.east - rect.west;
    let height = rect.north - rect.south;

    // One extra vertex ring per side forms the overlap skirt.
    let verts_per_side = segments + 3;
    let vertex_count = (verts_per_side * verts_per_side) as usize;

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(vertex_count);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(vertex_count);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(vertex_count);

    // WGS84 radii
    let a = EARTH_RADIUS; // semi-major axis
    let b = 6356752.314245_f64; // semi-minor axis
    let e2 = 1.0 - (b * b) / (a * a);

    for row in 0..verts_per_side {
        // v_norm spans [-MARGIN, 1+MARGIN]; 0..1 is the true tile extent.
        let v_norm = -TILE_MARGIN
            + (1.0 + 2.0 * TILE_MARGIN) * row as f64 / (verts_per_side - 1) as f64;
        let lat = rect.south + v_norm.clamp(-0.05, 1.05) * height;

        let cos_lat = lat.cos();
        let sin_lat = lat.sin();

        for col in 0..verts_per_side {
            // u_norm spans [-MARGIN, 1+MARGIN]; 0..1 is the true tile extent.
            let u_norm = -TILE_MARGIN
                + (1.0 + 2.0 * TILE_MARGIN) * col as f64 / (verts_per_side - 1) as f64;
            let lon = rect.west + u_norm * width;

            let cos_lon = lon.cos();
            let sin_lon = lon.sin();

            // Geodetic surface normal direction
            let nx = cos_lat * cos_lon;
            let ny = cos_lat * sin_lon;
            let nz = sin_lat;

            // Position on ellipsoid surface (cartographic to cartesian):
            // N = a / sqrt(1 - e2*sin^2(lat)); x = N*cos(lat)*cos(lon), etc.
            let n_val = a / (1.0 - e2 * sin_lat * sin_lat).sqrt();

            // Skirt vertices (outside the true tile extent) are tucked just
            // below the surface so overlapping neighbors never z-fight.
            let skirt = if u_norm < 0.0 || u_norm > 1.0 || v_norm < 0.0 || v_norm > 1.0 {
                SKIRT_TUCK
            } else {
                1.0
            };
            let tuck = base_tuck * skirt;

            let px = n_val * cos_lat * cos_lon * tuck;
            let py = n_val * cos_lat * sin_lon * tuck;
            let pz = n_val * (1.0 - e2) * sin_lat * tuck;

            positions.push([px as f32, py as f32, pz as f32]);
            normals.push([nx as f32, ny as f32, nz as f32]);
            // Bevy samples UV v=0 at the TOP of the image (row 0 = north for
            // map tiles). UVs clamp to [0,1] so the skirt reuses edge texels.
            uvs.push([
                u_norm.clamp(0.0, 1.0) as f32,
                (1.0 - v_norm.clamp(0.0, 1.0)) as f32,
            ]);
        }
    }

    // Generate triangle indices (counter-clockwise winding from outside)
    let quads = verts_per_side - 1;
    let mut indices: Vec<u32> = Vec::with_capacity((quads * quads * 6) as usize);
    for row in 0..quads {
        for col in 0..quads {
            let a = row * verts_per_side + col;
            let b = a + verts_per_side;

            // Two triangles per quad, CCW from outside
            indices.push(a);
            indices.push(a + 1);
            indices.push(b);
            indices.push(a + 1);
            indices.push(b + 1);
            indices.push(b);
        }
    }

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));

    mesh
}

/// Generates a smooth unit-radius UV sphere (used for the base sphere safety
/// net, whose silhouette is visible at the horizon). Winding matches
/// `create_tile_mesh` (counter-clockwise from outside, rows south -> north).
pub fn create_uv_sphere(longitude_segments: u32, latitude_rings: u32) -> Mesh {
    let verts_x = longitude_segments + 1; // last column duplicates the seam
    let verts_y = latitude_rings + 1;

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity((verts_x * verts_y) as usize);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity((verts_x * verts_y) as usize);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity((verts_x * verts_y) as usize);

    for row in 0..verts_y {
        // Rows iterate south (-90 deg) -> north (+90 deg), like tile meshes.
        let lat = -std::f64::consts::FRAC_PI_2
            + std::f64::consts::PI * row as f64 / latitude_rings as f64;
        let cos_lat = lat.cos();
        let sin_lat = lat.sin();
        for col in 0..verts_x {
            let lon = 2.0 * std::f64::consts::PI * col as f64 / longitude_segments as f64;
            let nx = cos_lat * lon.cos();
            let ny = cos_lat * lon.sin();
            let nz = sin_lat;
            positions.push([nx as f32, ny as f32, nz as f32]);
            normals.push([nx as f32, ny as f32, nz as f32]);
            uvs.push([
                col as f32 / longitude_segments as f32,
                row as f32 / latitude_rings as f32,
            ]);
        }
    }

    let mut indices: Vec<u32> =
        Vec::with_capacity((longitude_segments * latitude_rings * 6) as usize);
    for row in 0..latitude_rings {
        for col in 0..longitude_segments {
            let a = row * verts_x + col;
            let b = a + verts_x;
            indices.push(a);
            indices.push(a + 1);
            indices.push(b);
            indices.push(a + 1);
            indices.push(b + 1);
            indices.push(b);
        }
    }

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));

    mesh
}

/// Returns the render-scale factor for converting meters to render units.
pub fn render_scale() -> f32 {
    (1.0 / METERS_PER_RENDER_UNIT) as f32
}

/// Web Mercator maximum latitude (radians): atan(sinh(PI)) = 85.05112877980659 deg.
/// Web Mercator tiles only cover [-MAX_LAT, MAX_LAT]; the polar regions beyond
/// this latitude are not covered by any tile, so we cap them separately.
/// (Hardcoded because exp/atan are not const-fn; equals PI/2 - 2*atan(exp(-PI)).)
const MAX_MERCATOR_LAT: f64 = 1.4844222297453322;

/// Generates a polar cap mesh covering the region from the Web Mercator
/// maximum latitude (85.051129 deg) to the pole (90 deg).
///
/// This fills the hole left by Web Mercator tiling (which cannot represent
/// the poles). The cap is rendered as a solid color (ice white), matching
/// CesiumJS where terrain covers the poles and imagery is only draped over
/// the tiled region.
///
/// # Arguments
/// * `north` - true for the north polar cap, false for the south
/// * `segments` - number of longitudinal subdivisions around the pole
pub fn create_polar_cap(north: bool, segments: u32) -> Mesh {
    let a = EARTH_RADIUS;
    let b = 6356752.314245_f64;
    let e2 = 1.0 - (b * b) / (a * a);

    // Ring latitude: matches the outermost tile row boundary exactly so the
    // cap blends seamlessly with the adjacent tile mesh.
    let ring_lat = if north { MAX_MERCATOR_LAT } else { -MAX_MERCATOR_LAT };
    let sin_ring = ring_lat.sin();
    let cos_ring = ring_lat.cos();
    let n_ring = a / (1.0 - e2 * sin_ring * sin_ring).sqrt();

    // Pole vertex on the ellipsoid surface.
    let pole_z = if north { b } else { -b };

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity((segments + 2) as usize);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity((segments + 2) as usize);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity((segments + 2) as usize);

    // Tuck the cap just below the tile surface so the (overlapping) tile
    // skirts cover the seam without z-fighting.
    const CAP_TUCK: f64 = 0.9995;

    // Center vertex at the pole (normal points along the polar axis, outward).
    positions.push([0.0, 0.0, (pole_z * CAP_TUCK) as f32]);
    normals.push([0.0, 0.0, if north { 1.0 } else { -1.0 }]);
    uvs.push([0.5, 0.5]);

    // Ring of vertices at the maximum Mercator latitude.
    for i in 0..=segments {
        let lon = 2.0 * std::f64::consts::PI * i as f64 / segments as f64;
        let cos_lon = lon.cos();
        let sin_lon = lon.sin();

        let px = n_ring * cos_ring * cos_lon;
        let py = n_ring * cos_ring * sin_lon;
        let pz = n_ring * (1.0 - e2) * sin_ring;

        positions.push([
            (px * CAP_TUCK) as f32,
            (py * CAP_TUCK) as f32,
            (pz * CAP_TUCK) as f32,
        ]);
        normals.push([(cos_ring * cos_lon) as f32, (cos_ring * sin_lon) as f32, sin_ring as f32]);
        uvs.push([0.5, 0.5]);
    }

    // Triangle fan from the pole to the ring.
    // Winding chosen so normals point outward (away from the globe center).
    let mut indices: Vec<u32> = Vec::with_capacity(segments as usize * 3);
    for i in 0..segments {
        let ring_a = 1 + i;
        let ring_b = 1 + i + 1;
        if north {
            // Counter-clockwise viewed from above (+z) -> normal +z (outward).
            indices.push(0);
            indices.push(ring_a);
            indices.push(ring_b);
        } else {
            // Clockwise viewed from above -> normal -z (outward at south pole).
            indices.push(0);
            indices.push(ring_b);
            indices.push(ring_a);
        }
    }

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));

    mesh
}
