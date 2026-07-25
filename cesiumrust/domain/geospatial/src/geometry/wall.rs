//! Wall geometry - a vertical curtain extruded between top and bottom heights.
//!
//! Faithful port of CesiumJS `WallGeometryLibrary.js`, `WallGeometry.js` and
//! `WallOutlineGeometry.js`. A wall is defined by a series of surface positions
//! that extrude vertically between a minimum and maximum height. The arc between
//! consecutive positions is subdivided along the geodesic (see
//! [`crate::polyline_pipeline`]).

use crate::bounding::BoundingSphere;
use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;
use crate::geometry::{GeometryData, PrimitiveType, VertexFormat};
use crate::math_utils::EPSILON10;
use crate::polyline_pipeline::{generate_arc, ArcOptions};
use glam::DVec3;

/// Default granularity: one degree in radians.
const DEFAULT_GRANULARITY: f64 = std::f64::consts::PI / 180.0;

/// Options describing a wall.
#[derive(Debug, Clone)]
pub struct WallOptions {
    /// The surface positions defining the wall path (at least 2).
    pub positions: Vec<DVec3>,
    /// Maximum (top) height at each position. `None` uses each position's height.
    pub maximum_heights: Option<Vec<f64>>,
    /// Minimum (bottom) height at each position. `None` uses 0.
    pub minimum_heights: Option<Vec<f64>>,
    /// Angular granularity in radians.
    pub granularity: f64,
    /// The reference ellipsoid.
    pub ellipsoid: Ellipsoid,
}

impl WallOptions {
    /// Creates a wall from constant top/bottom heights (mirrors
    /// `WallGeometry.fromConstantHeights`).
    pub fn from_constant_heights(
        positions: Vec<DVec3>,
        minimum_height: Option<f64>,
        maximum_height: Option<f64>,
        ellipsoid: Ellipsoid,
    ) -> Self {
        let length = positions.len();
        let minimum_heights = minimum_height.map(|h| vec![h; length]);
        let maximum_heights = maximum_height.map(|h| vec![h; length]);
        Self {
            positions,
            maximum_heights,
            minimum_heights,
            granularity: DEFAULT_GRANULARITY,
            ellipsoid,
        }
    }
}

impl Default for WallOptions {
    fn default() -> Self {
        Self {
            positions: Vec::new(),
            maximum_heights: None,
            minimum_heights: None,
            granularity: DEFAULT_GRANULARITY,
            ellipsoid: Ellipsoid::WGS84,
        }
    }
}

/// Cleaned positions after duplicate removal.
struct CleanedPositions {
    positions: Vec<DVec3>,
    top_heights: Vec<f64>,
    bottom_heights: Vec<f64>,
}

fn lat_lon_equals(c0: &Cartographic, c1: &Cartographic) -> bool {
    (c0.latitude - c1.latitude).abs() <= EPSILON10
        && (c0.longitude - c1.longitude).abs() <= EPSILON10
}

fn cartesian_equals_epsilon(a: DVec3, b: DVec3) -> bool {
    (a.x - b.x).abs() <= EPSILON10
        && (a.y - b.y).abs() <= EPSILON10
        && (a.z - b.z).abs() <= EPSILON10
}

/// Removes consecutive duplicate positions (and merges heights for positions
/// sharing the same longitude/latitude).
///
/// Maps to `WallGeometryLibrary`'s private `removeDuplicates`.
fn remove_duplicates(
    ellipsoid: &Ellipsoid,
    positions: &[DVec3],
    top_heights: Option<&[f64]>,
    bottom_heights: Option<&[f64]>,
) -> Option<CleanedPositions> {
    // arrayRemoveDuplicates: drop consecutive exactly-equal positions.
    let mut deduped: Vec<DVec3> = Vec::with_capacity(positions.len());
    for &p in positions {
        if deduped.last().is_none_or(|&last| !cartesian_equals_epsilon(last, p)) {
            deduped.push(p);
        }
    }

    let length = deduped.len();
    if length < 2 {
        return None;
    }

    let has_bottom = bottom_heights.is_some();
    let has_top = top_heights.is_some();

    let mut cleaned_positions: Vec<DVec3> = Vec::with_capacity(length);
    let mut cleaned_top: Vec<f64> = Vec::with_capacity(length);
    let mut cleaned_bottom: Vec<f64> = Vec::with_capacity(length);

    let v0 = deduped[0];
    cleaned_positions.push(v0);

    let mut c0 = ellipsoid.cartesian_to_cartographic(v0).unwrap_or_default();
    if has_top {
        c0.height = top_heights.unwrap()[0];
    }
    cleaned_top.push(c0.height);
    cleaned_bottom.push(if has_bottom { bottom_heights.unwrap()[0] } else { 0.0 });

    let start_top = cleaned_top[0];
    let start_bottom = cleaned_bottom[0];
    let mut has_all_same_heights = (start_top - start_bottom).abs() < f64::EPSILON;

    for (i, &v1) in deduped.iter().enumerate().skip(1) {
        let mut c1 = ellipsoid.cartesian_to_cartographic(v1).unwrap_or_default();
        if has_top {
            c1.height = top_heights.unwrap()[i];
        }
        has_all_same_heigths_check(&mut has_all_same_heights, c1.height);

        if !lat_lon_equals(&c0, &c1) {
            cleaned_positions.push(v1);
            cleaned_top.push(c1.height);
            cleaned_bottom.push(if has_bottom { bottom_heights.unwrap()[i] } else { 0.0 });
            let idx = cleaned_top.len() - 1;
            has_all_same_heights =
                has_all_same_heights && (cleaned_top[idx] - cleaned_bottom[idx]).abs() < f64::EPSILON;
            c0 = c1;
        } else if c0.height < c1.height {
            // Adjacent positions share lon/lat: keep the greater top height.
            let idx = cleaned_top.len() - 1;
            cleaned_top[idx] = c1.height;
        }
    }

    if has_all_same_heights || cleaned_positions.len() < 2 {
        return None;
    }

    Some(CleanedPositions {
        positions: cleaned_positions,
        top_heights: cleaned_top,
        bottom_heights: cleaned_bottom,
    })
}

#[inline]
fn has_all_same_heigths_check(flag: &mut bool, height: f64) {
    *flag = *flag && height.abs() < f64::EPSILON;
}

/// Result of [`compute_positions`].
struct WallPositions {
    top_positions: Vec<DVec3>,
    bottom_positions: Vec<DVec3>,
    num_corners: usize,
}

/// Computes the subdivided top and bottom position arrays for the wall.
///
/// Maps to `WallGeometryLibrary.computePositions`. When `duplicate_corners` is
/// true (filled geometry) each segment is subdivided independently so corners
/// are duplicated for correct per-face normals; when false (outline) the whole
/// path is subdivided as a single arc.
fn compute_positions(
    ellipsoid: &Ellipsoid,
    wall_positions: &[DVec3],
    maximum_heights: Option<&[f64]>,
    minimum_heights: Option<&[f64]>,
    granularity: f64,
    duplicate_corners: bool,
) -> Option<WallPositions> {
    let cleaned = remove_duplicates(ellipsoid, wall_positions, maximum_heights, minimum_heights)?;

    let wall_positions = cleaned.positions;
    let maximum_heights = cleaned.top_heights;
    let minimum_heights = cleaned.bottom_heights;

    let length = wall_positions.len();
    let num_corners = length - 2;

    let (top_positions, bottom_positions) = if duplicate_corners {
        let mut top_positions: Vec<DVec3> = Vec::new();
        let mut bottom_positions: Vec<DVec3> = Vec::new();

        for i in 0..length - 1 {
            let seg_positions = [wall_positions[i], wall_positions[i + 1]];

            let top_heights = [maximum_heights[i], maximum_heights[i + 1]];
            let top_opts = ArcOptions {
                positions: &seg_positions,
                heights: Some(&top_heights),
                granularity,
                ellipsoid,
            };
            let top = generate_arc(&top_opts);

            let bottom_heights = [minimum_heights[i], minimum_heights[i + 1]];
            let bottom_opts = ArcOptions {
                positions: &seg_positions,
                heights: Some(&bottom_heights),
                granularity,
                ellipsoid,
            };
            let bottom = generate_arc(&bottom_opts);

            top_positions.extend_from_slice(&top);
            bottom_positions.extend_from_slice(&bottom);
        }
        (top_positions, bottom_positions)
    } else {
        let top_opts = ArcOptions {
            positions: &wall_positions,
            heights: Some(&maximum_heights),
            granularity,
            ellipsoid,
        };
        let bottom_opts = ArcOptions {
            positions: &wall_positions,
            heights: Some(&minimum_heights),
            granularity,
            ellipsoid,
        };
        (generate_arc(&top_opts), generate_arc(&bottom_opts))
    };

    Some(WallPositions {
        top_positions,
        bottom_positions,
        num_corners,
    })
}

/// Generates a filled wall geometry.
///
/// Maps to CesiumJS `WallGeometry.createGeometry`.
pub fn wall_geometry(options: &WallOptions, vf: VertexFormat) -> GeometryData {
    let ellipsoid = &options.ellipsoid;
    let pos = compute_positions(
        ellipsoid,
        &options.positions,
        options.maximum_heights.as_deref(),
        options.minimum_heights.as_deref(),
        options.granularity,
        true,
    );

    let Some(pos) = pos else {
        return empty_geometry(PrimitiveType::Triangles);
    };

    let top_positions = &pos.top_positions;
    let bottom_positions = &pos.bottom_positions;
    let num_corners = pos.num_corners;
    let length = top_positions.len();

    // Interleave bottom (even) and top (odd) positions.
    let mut positions: Vec<[f64; 3]> = Vec::with_capacity(length * 2);
    let mut normals: Option<Vec<[f64; 3]>> = if vf.normal { Some(Vec::new()) } else { None };
    let mut tangents: Option<Vec<[f64; 3]>> = if vf.tangent { Some(Vec::new()) } else { None };
    let mut bitangents: Option<Vec<[f64; 3]>> = if vf.bitangent { Some(Vec::new()) } else { None };
    let mut tex_coords: Option<Vec<[f64; 2]>> = if vf.st { Some(Vec::new()) } else { None };

    let mut normal = DVec3::ZERO;
    let mut tangent = DVec3::ZERO;
    let mut bitangent = DVec3::ZERO;
    let mut recompute_normal = true;
    let mut s = 0.0f64;
    let ds = if length > num_corners + 1 {
        1.0 / (length - num_corners - 1) as f64
    } else {
        0.0
    };

    for i in 0..length {
        let top_position = top_positions[i];
        let bottom_position = bottom_positions[i];

        positions.push([bottom_position.x, bottom_position.y, bottom_position.z]);
        positions.push([top_position.x, top_position.y, top_position.z]);

        if let Some(ref mut st) = tex_coords {
            st.push([s, 0.0]);
            st.push([s, 1.0]);
        }

        if normals.is_some() || tangents.is_some() || bitangents.is_some() {
            let mut next_top = DVec3::ZERO;
            let surface_normal = ellipsoid
                .geodetic_surface_normal(top_position)
                .unwrap_or(DVec3::Z);
            let ground_position = top_position - surface_normal;
            if i + 1 < length {
                next_top = top_positions[i + 1];
            }

            if recompute_normal {
                let scaled_next = next_top - top_position;
                let scaled_ground = ground_position - top_position;
                normal = scaled_ground.cross(scaled_next).normalize_or(DVec3::Z);
                recompute_normal = false;
            }

            if cartesian_equals_epsilon(top_position, next_top) {
                recompute_normal = true;
            } else {
                s += ds;
                if tangents.is_some() {
                    tangent = (next_top - top_position).normalize_or(DVec3::X);
                }
                if bitangents.is_some() {
                    bitangent = normal.cross(tangent).normalize_or(DVec3::Y);
                }
            }

            if let Some(ref mut n) = normals {
                n.push([normal.x, normal.y, normal.z]);
                n.push([normal.x, normal.y, normal.z]);
            }
            if let Some(ref mut t) = tangents {
                t.push([tangent.x, tangent.y, tangent.z]);
                t.push([tangent.x, tangent.y, tangent.z]);
            }
            if let Some(ref mut b) = bitangents {
                b.push([bitangent.x, bitangent.y, bitangent.z]);
                b.push([bitangent.x, bitangent.y, bitangent.z]);
            }
        }
    }

    // Two triangles per wall quad.
    let num_vertices = positions.len();
    let mut indices: Vec<u32> = Vec::new();
    let mut i = 0usize;
    while i + 2 < num_vertices {
        let ll = i;
        let lr = i + 2;
        let pl = DVec3::from(positions[ll]);
        let pr = DVec3::from(positions[lr]);
        if cartesian_equals_epsilon(pl, pr) {
            i += 2;
            continue;
        }
        let ul = i + 1;
        let ur = i + 3;
        indices.extend_from_slice(&[ul as u32, ll as u32, ur as u32]);
        indices.extend_from_slice(&[ur as u32, ll as u32, lr as u32]);
        i += 2;
    }

    let bounding_sphere = BoundingSphere::from_points(
        &positions.iter().map(|p| DVec3::new(p[0], p[1], p[2])).collect::<Vec<_>>(),
    );

    GeometryData {
        positions,
        normals,
        tex_coords,
        tangents,
        bitangents,
        indices,
        bounding_sphere,
        primitive_type: PrimitiveType::Triangles,
    }
}

/// Generates a wall outline geometry (line segments).
///
/// Maps to CesiumJS `WallOutlineGeometry.createGeometry`.
pub fn wall_outline_geometry(options: &WallOptions) -> GeometryData {
    let ellipsoid = &options.ellipsoid;
    let pos = compute_positions(
        ellipsoid,
        &options.positions,
        options.maximum_heights.as_deref(),
        options.minimum_heights.as_deref(),
        options.granularity,
        false,
    );

    let Some(pos) = pos else {
        return empty_geometry(PrimitiveType::Lines);
    };

    let top_positions = &pos.top_positions;
    let bottom_positions = &pos.bottom_positions;
    let length = top_positions.len();

    // Interleave bottom (even) and top (odd).
    let mut positions: Vec<[f64; 3]> = Vec::with_capacity(length * 2);
    for i in 0..length {
        let bp = bottom_positions[i];
        let tp = top_positions[i];
        positions.push([bp.x, bp.y, bp.z]);
        positions.push([tp.x, tp.y, tp.z]);
    }

    let num_vertices = positions.len();
    let mut indices: Vec<u32> = Vec::new();
    let mut i = 0usize;
    while i + 2 < num_vertices {
        let ll = i;
        let lr = i + 2;
        let pl = DVec3::from(positions[ll]);
        let pr = DVec3::from(positions[lr]);
        if cartesian_equals_epsilon(pl, pr) {
            i += 2;
            continue;
        }
        let ul = i + 1;
        let ur = i + 3;
        // Vertical left edge, top edge, bottom edge.
        indices.extend_from_slice(&[ul as u32, ll as u32]);
        indices.extend_from_slice(&[ul as u32, ur as u32]);
        indices.extend_from_slice(&[ll as u32, lr as u32]);
        i += 2;
    }
    // Final vertical edge.
    if num_vertices >= 2 {
        indices.push((num_vertices - 2) as u32);
        indices.push((num_vertices - 1) as u32);
    }

    let bounding_sphere = BoundingSphere::from_points(
        &positions.iter().map(|p| DVec3::new(p[0], p[1], p[2])).collect::<Vec<_>>(),
    );

    GeometryData {
        positions,
        normals: None,
        tex_coords: None,
        tangents: None,
        bitangents: None,
        indices,
        bounding_sphere,
        primitive_type: PrimitiveType::Lines,
    }
}

fn empty_geometry(primitive_type: PrimitiveType) -> GeometryData {
    GeometryData {
        positions: Vec::new(),
        normals: None,
        tex_coords: None,
        tangents: None,
        bitangents: None,
        indices: Vec::new(),
        bounding_sphere: BoundingSphere::default(),
        primitive_type,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wall_options() -> WallOptions {
        let ell = Ellipsoid::WGS84;
        let positions = vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(19.0, 47.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(19.0, 48.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(20.0, 48.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(20.0, 47.0, 0.0)),
        ];
        WallOptions::from_constant_heights(positions, Some(0.0), Some(10000.0), ell)
    }

    #[test]
    fn test_wall_geometry_basic() {
        let geo = wall_geometry(&wall_options(), VertexFormat::ALL);
        assert!(!geo.positions.is_empty());
        assert_eq!(geo.primitive_type, PrimitiveType::Triangles);
        assert_eq!(geo.indices.len() % 3, 0);
        // Positions interleaved bottom/top => even count.
        assert_eq!(geo.positions.len() % 2, 0);
        assert_eq!(geo.normals.as_ref().unwrap().len(), geo.positions.len());
        assert_eq!(geo.tex_coords.as_ref().unwrap().len(), geo.positions.len());
    }

    #[test]
    fn test_wall_heights_correct() {
        let ell = Ellipsoid::WGS84;
        let geo = wall_geometry(&wall_options(), VertexFormat::POSITION_ONLY);
        // Even indices are bottom (~0 m), odd are top (~10000 m).
        for (i, p) in geo.positions.iter().enumerate() {
            let c = ell.cartesian_to_cartographic(DVec3::new(p[0], p[1], p[2])).unwrap();
            if i % 2 == 0 {
                assert!(c.height.abs() < 1.0, "bottom height {}", c.height);
            } else {
                assert!((c.height - 10000.0).abs() < 1.0, "top height {}", c.height);
            }
        }
    }

    #[test]
    fn test_wall_outline_basic() {
        let geo = wall_outline_geometry(&wall_options());
        assert!(!geo.positions.is_empty());
        assert_eq!(geo.primitive_type, PrimitiveType::Lines);
        assert_eq!(geo.indices.len() % 2, 0);
        // All indices in range.
        let n = geo.positions.len() as u32;
        for &idx in &geo.indices {
            assert!(idx < n);
        }
    }

    #[test]
    fn test_wall_degenerate_all_zero_heights() {
        // When all top heights are 0, CesiumJS considers the wall degenerate.
        let ell = Ellipsoid::WGS84;
        let positions = vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 0.0, 0.0)),
        ];
        let opts = WallOptions::from_constant_heights(positions, None, None, ell);
        let geo = wall_geometry(&opts, VertexFormat::POSITION_ONLY);
        assert!(geo.positions.is_empty());
    }

    #[test]
    fn test_wall_too_few_positions() {
        let ell = Ellipsoid::WGS84;
        let positions = vec![ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0))];
        let opts = WallOptions::from_constant_heights(positions, Some(0.0), Some(100.0), ell);
        let geo = wall_geometry(&opts, VertexFormat::POSITION_ONLY);
        assert!(geo.positions.is_empty());
    }
}
