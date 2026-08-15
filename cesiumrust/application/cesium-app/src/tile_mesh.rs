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
/// WGS84 semi-minor axis (meters).
const EARTH_RADIUS_MINOR: f64 = 6356752.314245;
/// First eccentricity squared of the WGS84 ellipsoid.
const E2: f64 =
    1.0 - (EARTH_RADIUS_MINOR * EARTH_RADIUS_MINOR) / (EARTH_RADIUS * EARTH_RADIUS);

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

/// Radial drop factor of the hanging skirt ring for a tile at level `z`.
/// Skirt vertices sit at the EXACT tile boundary (same lat/lon as the edge
/// row, so neighbors share bit-identical edge vertices) but at this fraction
/// of the surface radius: a vertical wall hanging below the surface,
/// CesiumJS `EllipsoidTessellator`-style. LOD levels render near the same
/// radius (REPLACE refinement + camera-adaptive radial tuck via entity
/// scale), so the wall only fills sub-pixel rasterization cracks and the
/// chord-sagitta crease at LOD boundaries; the depth covers the coarsest
/// live level's sagitta with margin while staying a small fraction of the
/// tile size.
fn skirt_drop(z: u32) -> f64 {
    1.0 - 3.0 * tuck_step(z)
}

/// Level-scaled radial step: 15% of the level's tile arc, clamped so coarse
/// levels keep a usable skirt and deep levels never degenerate. The floor
/// must exceed the MAX runtime LOD cliff: `adaptive_tuck_step` caps at
/// 1.5e-4 per level and fast zooms/drag leave live neighbors up to ~4 levels
/// apart (cliff ~6e-4); a shallower skirt wall leaves a see-through crack
/// at LOD boundaries that reads as thin stripes during fast motion.
/// Only drives [`skirt_drop`] now that LOD tuck is applied via entity scale.
fn tuck_step(z: u32) -> f64 {
    let arc = 2.0 * std::f64::consts::PI / (1u64 << z.min(24)) as f64;
    (arc * 0.15).clamp(5.0e-4, 6.0e-4)
}

/// Generates a Bevy Mesh for a single tile on the WGS84 ellipsoid surface.
///
/// Follows CesiumJS `HeightmapTessellator.computeVertices`:
/// - Vertices distributed across the tile's geographic extent
/// - UV: u = (lon - west) / (east - west), v = (lat - south) / (north - south)
/// - Positions: cartographic_to_cartesian on WGS84 ellipsoid
/// - Normals: geodetic surface normal (normalized position on ellipsoid)
/// - Triangle winding: counter-clockwise from outside (outward normals)
/// - A hanging skirt ring (boundary loop duplicated at `SKIRT_DROP` radius)
///   fills rasterization cracks between neighboring tiles without ever
///   overlapping a neighbor's surface
///
/// The mesh is built at the exact ellipsoid radius (no radial offset):
/// every tile — coarse or fine — sits at its true position, exactly like
/// CesiumJS terrain meshes. Overlap/z-fighting is impossible because the
/// render partition (`sync_visibility`) never draws a parent together with
/// its children, so no radial tuck is needed and no LOD-boundary fin can
/// form.
///
/// # Arguments
/// * `x`, `y`, `z` - Tile coordinates in Web Mercator tiling scheme
/// * `segments` - Number of subdivisions per axis within the true tile extent
pub fn create_tile_mesh(x: u32, y: u32, z: u32, segments: u32) -> Mesh {
    create_tile_mesh_uv(x, y, z, segments, [0.0, 0.0, 1.0, 1.0])
}

/// Same as [`create_tile_mesh`] but remaps UVs into `uv_rect` =
/// [u0, v0, u1, v1], a sub-rectangle of the texture. Used by no-data tiles
/// that inherit an ancestor's image: the child samples exactly its own
/// quadrant region of the ancestor texture (CesiumJS-style upsampling
/// fallback), so no-imagery regions blend seamlessly with real coverage.
pub fn create_tile_mesh_uv(
    x: u32,
    y: u32,
    z: u32,
    segments: u32,
    uv_rect: [f32; 4],
) -> Mesh {
    let rect = tile_xy_to_rectangle(x, y, z);

    let width = rect.east - rect.west;
    let height = rect.north - rect.south;

    // Exact [0,1] grid: adjacent tiles share bit-identical edge vertices
    // (same lat/lon formulas -> same f32 positions), so neighbors meet
    // without overlap; the hanging skirt ring below fills the remaining
    // sub-pixel cracks.
    let verts_per_side = segments + 1;
    let grid_count = (verts_per_side * verts_per_side) as usize;
    let perimeter_count = (4 * segments) as usize;

    let ring_verts = 2 * perimeter_count;
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(grid_count + ring_verts);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(grid_count + ring_verts);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(grid_count + ring_verts);

    for row in 0..verts_per_side {
        // v_norm spans [0,1] exactly: the true tile extent, no overlap.
        let v_norm = row as f64 / (verts_per_side - 1) as f64;
        let lat = rect.south + v_norm * height;

        let cos_lat = lat.cos();
        let sin_lat = lat.sin();

        for col in 0..verts_per_side {
            let u_norm = col as f64 / (verts_per_side - 1) as f64;
            let lon = rect.west + u_norm * width;

            let cos_lon = lon.cos();
            let sin_lon = lon.sin();

            // Geodetic surface normal direction
            let nx = cos_lat * cos_lon;
            let ny = cos_lat * sin_lon;
            let nz = sin_lat;

            // Position on ellipsoid surface (cartographic to cartesian):
            // N = a / sqrt(1 - e2*sin^2(lat)); x = N*cos(lat)*cos(lon), etc.
            let n_val = EARTH_RADIUS / (1.0 - E2 * sin_lat * sin_lat).sqrt();

            positions.push([
                (n_val * nx) as f32,
                (n_val * ny) as f32,
                (n_val * (1.0 - E2) * sin_lat) as f32,
            ]);
            normals.push([nx as f32, ny as f32, nz as f32]);
            // Bevy samples UV v=0 at the TOP of the image (row 0 = north for
            // map tiles).
            let u = u_norm as f32;
            let v = (1.0 - v_norm) as f32;
            uvs.push([
                uv_rect[0] + u * (uv_rect[2] - uv_rect[0]),
                uv_rect[1] + v * (uv_rect[3] - uv_rect[1]),
            ]);
        }
    }

    // Hanging skirt: duplicate the boundary loop at the level-scaled drop
    // radius. The wall hangs straight down from the tile edge, so it can
    // only peek through cracks, never over a neighbor. Drop alternates
    // slightly with tile parity so coincident walls of same-level neighbors
    // cannot z-fight in the cracks.
    let base_drop = skirt_drop(z);
    let drop = if (x + y + z) & 1 == 0 {
        base_drop
    } else {
        base_drop - 0.1 * tuck_step(z)
    };
    let mut perim: Vec<u32> = Vec::with_capacity(perimeter_count);
    // Per-edge CONSTANT uv for the skirt wall: the wall is only ever seen
    // through sub-pixel cracks, so any uv variation along it would squeeze
    // the edge column sideways into stripe fins. Both wall rings therefore
    // carry a single mid-edge texel so an exposed wall reads as a plain
    // edge continuation.
    let mid_u = |u: f32, v: f32| {
        [
            uv_rect[0] + u * (uv_rect[2] - uv_rect[0]),
            uv_rect[1] + v * (uv_rect[3] - uv_rect[1]),
        ]
    };
    let south_uv = mid_u(0.5, 1.0);
    let east_uv = mid_u(1.0, 0.5);
    let north_uv = mid_u(0.5, 0.0);
    let west_uv = mid_u(0.0, 0.5);
    let mut perim_uv: Vec<[f32; 2]> = Vec::with_capacity(perimeter_count);
    let last = verts_per_side - 1;
    for col in 0..verts_per_side {
        perim.push(col); // south edge, west -> east
        perim_uv.push(south_uv);
    }
    for row in 1..verts_per_side {
        perim.push(row * verts_per_side + last); // east edge, south -> north
        perim_uv.push(east_uv);
    }
    for col in (0..last).rev() {
        perim.push(last * verts_per_side + col); // north edge, east -> west
        perim_uv.push(north_uv);
    }
    for row in (1..last).rev() {
        perim.push(row * verts_per_side); // west edge, north -> south
        perim_uv.push(west_uv);
    }
    // Two dedicated wall rings: a TOP ring coincident with the surface edge
    // (its own vertices because the grid edge vertices must keep their
    // varying surface uvs) and a BOTTOM ring at the drop radius. Interpolating
    // constant->constant keeps the whole wall a flat color; interpolating
    // from the grid edge's varying uvs (as a shared-vertex wall would) is
    // exactly what produced the stripe fins.
    for (i, &g) in perim.iter().enumerate() {
        let p = positions[g as usize];
        positions.push(p); // top ring: coincident with edge, constant uv
        normals.push(normals[g as usize]);
        uvs.push(perim_uv[i]);
        positions.push([p[0] * drop as f32, p[1] * drop as f32, p[2] * drop as f32]);
        normals.push(normals[g as usize]);
        uvs.push(perim_uv[i]);
    }

    // Generate triangle indices (counter-clockwise winding from outside)
    let quads = verts_per_side - 1;
    let mut indices: Vec<u32> =
        Vec::with_capacity((quads * quads * 6) as usize + perimeter_count * 6);
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
    // Skirt wall strip between the two dedicated rings (double-sided
    // material, winding irrelevant). Each perimeter point contributed a
    // top/bottom vertex pair, so ring indices interleave: top = 2*i,
    // bottom = 2*i + 1 (offset by the grid). Never reference the grid edge
    // vertices directly: their uvs vary along the edge and would re-create
    // the stripe fins.
    for i in 0..perimeter_count {
        let j = (i + 1) % perimeter_count;
        let t0 = grid_count as u32 + 2 * i as u32;
        let b0 = t0 + 1;
        let t1 = grid_count as u32 + 2 * j as u32;
        let b1 = t1 + 1;
        indices.push(t0);
        indices.push(b0);
        indices.push(b1);
        indices.push(t0);
        indices.push(b1);
        indices.push(t1);
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
