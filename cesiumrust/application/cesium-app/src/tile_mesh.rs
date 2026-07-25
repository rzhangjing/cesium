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

/// Generates a Bevy Mesh for a single tile on the WGS84 ellipsoid surface.
///
/// Follows CesiumJS `HeightmapTessellator.computeVertices`:
/// - Vertices distributed across the tile's geographic extent
/// - UV: u = (lon - west) / (east - west), v = (lat - south) / (north - south)
/// - Positions: cartographic_to_cartesian on WGS84 ellipsoid
/// - Normals: geodetic surface normal (normalized position on ellipsoid)
/// - Triangle winding: counter-clockwise from outside (outward normals)
///
/// # Arguments
/// * `x`, `y`, `z` - Tile coordinates in Web Mercator tiling scheme
/// * `segments` - Number of subdivisions per axis (e.g., 16 means 16x16 quads = 17x17 vertices)
pub fn create_tile_mesh(x: u32, y: u32, z: u32, segments: u32) -> Mesh {
    let rect = tile_xy_to_rectangle(x, y, z);

    let width = rect.east - rect.west;
    let height = rect.north - rect.south;

    let verts_per_side = segments + 1;
    let vertex_count = (verts_per_side * verts_per_side) as usize;

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(vertex_count);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(vertex_count);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(vertex_count);

    // WGS84 radii
    let a = EARTH_RADIUS; // semi-major axis
    let b = 6356752.314245_f64; // semi-minor axis

    for row in 0..verts_per_side {
        // v goes from 0 (south) to 1 (north)
        let v = row as f64 / segments as f64;
        let lat = rect.south + v * height;

        let cos_lat = lat.cos();
        let sin_lat = lat.sin();

        for col in 0..verts_per_side {
            // u goes from 0 (west) to 1 (east)
            let u = col as f64 / segments as f64;
            let lon = rect.west + u * width;

            let cos_lon = lon.cos();
            let sin_lon = lon.sin();

            // Geodetic surface normal direction
            let nx = cos_lat * cos_lon;
            let ny = cos_lat * sin_lon;
            let nz = sin_lat;

            // Position on ellipsoid surface (cartographic to cartesian)
            // Using the standard formula:
            // N = a / sqrt(cos^2(lat) + (b/a)^2 * sin^2(lat))
            // x = N * cos(lat) * cos(lon)
            // y = N * cos(lat) * sin(lon)
            // z = N * (b/a)^2 * sin(lat)
            let e2 = 1.0 - (b * b) / (a * a);
            let n_val = a / (1.0 - e2 * sin_lat * sin_lat).sqrt();

            let px = n_val * cos_lat * cos_lon;
            let py = n_val * cos_lat * sin_lon;
            let pz = n_val * (1.0 - e2) * sin_lat;

            positions.push([px as f32, py as f32, pz as f32]);
            normals.push([nx as f32, ny as f32, nz as f32]);
            // Bevy samples UV v=0 at the TOP of the image (row 0 = north for map
            // tiles). Our loop iterates south (row 0) -> north, so flip v so that
            // v=0 corresponds to north, matching the texture orientation.
            uvs.push([u as f32, (1.0 - v) as f32]);
        }
    }

    // Generate triangle indices (counter-clockwise winding from outside)
    let mut indices: Vec<u32> = Vec::with_capacity((segments * segments * 6) as usize);
    for row in 0..segments {
        for col in 0..segments {
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

    // Center vertex at the pole (normal points along the polar axis, outward).
    positions.push([0.0, 0.0, pole_z as f32]);
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

        positions.push([px as f32, py as f32, pz as f32]);
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
