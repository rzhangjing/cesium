//! Polyline geometry - a ribbon of constant width along a geodesic arc.
//!
//! Faithful adaptation of CesiumJS `PolylineGeometry.js`. CesiumJS uses
//! GPU-side expansion (prevPosition/nextPosition/expandAndWidth attributes);
//! here we generate a CPU-expanded triangle-strip ribbon in world space which
//! is directly renderable with Bevy's standard mesh pipeline.

use crate::bounding::BoundingSphere;
use crate::ellipsoid::Ellipsoid;
use crate::geometry::{GeometryData, PrimitiveType, VertexFormat};
use crate::math_utils::EPSILON10;
use crate::polyline_pipeline::{generate_arc, ArcOptions};
use glam::DVec3;

/// Options describing a polyline.
#[derive(Debug, Clone)]
pub struct PolylineOptions {
    /// The polyline positions (at least 2).
    pub positions: Vec<DVec3>,
    /// Width in meters.
    pub width: f64,
    /// Angular granularity in radians for arc subdivision.
    pub granularity: f64,
    /// The reference ellipsoid.
    pub ellipsoid: Ellipsoid,
}

impl Default for PolylineOptions {
    fn default() -> Self {
        Self {
            positions: Vec::new(),
            width: 1.0,
            granularity: std::f64::consts::PI / 180.0,
            ellipsoid: Ellipsoid::WGS84,
        }
    }
}

/// Generates a polyline geometry as a flat ribbon (triangle strip).
///
/// The ribbon lies on the ellipsoid surface, centered on the geodesic arc,
/// with the specified width. Normals point outward from the ellipsoid.
pub fn polyline_geometry(options: &PolylineOptions, vf: VertexFormat) -> GeometryData {
    let ellipsoid = &options.ellipsoid;
    let width = options.width;

    // Remove duplicates.
    let mut positions: Vec<DVec3> = options.positions.clone();
    positions.dedup_by(|a, b| {
        (a.x - b.x).abs() <= EPSILON10
            && (a.y - b.y).abs() <= EPSILON10
            && (a.z - b.z).abs() <= EPSILON10
    });

    if positions.len() < 2 || width <= 0.0 {
        return empty_geometry();
    }

    // Subdivide into a geodesic arc.
    let opts = ArcOptions {
        positions: &positions,
        heights: None,
        granularity: options.granularity,
        ellipsoid,
    };
    let arc = generate_arc(&opts);

    let n = arc.len();
    if n < 2 {
        return empty_geometry();
    }

    let half_width = width / 2.0;

    // For each arc point, compute the perpendicular (left) direction and
    // offset to get left/right edge vertices.
    let mut pos_out: Vec<[f64; 3]> = Vec::with_capacity(n * 2);
    let mut normals_out: Option<Vec<[f64; 3]>> = if vf.normal { Some(Vec::with_capacity(n * 2)) } else { None };
    let mut tangents_out: Option<Vec<[f64; 3]>> = if vf.tangent { Some(Vec::with_capacity(n * 2)) } else { None };
    let mut bitangents_out: Option<Vec<[f64; 3]>> = if vf.bitangent { Some(Vec::with_capacity(n * 2)) } else { None };
    let mut st_out: Option<Vec<[f64; 2]>> = if vf.st { Some(Vec::with_capacity(n * 2)) } else { None };

    let st_s = if n > 1 { 1.0 / (n - 1) as f64 } else { 1.0 };

    for i in 0..n {
        let p = arc[i];
        let normal = ellipsoid.geodetic_surface_normal(p).unwrap_or(DVec3::Z);

        // Tangent direction along the arc.
        let tangent = if i == 0 {
            (arc[1] - arc[0]).normalize_or(DVec3::X)
        } else if i == n - 1 {
            (arc[n - 1] - arc[n - 2]).normalize_or(DVec3::X)
        } else {
            (arc[i + 1] - arc[i - 1]).normalize_or(DVec3::X)
        };

        // Left direction: cross(normal, tangent) gives the perpendicular in the tangent plane.
        let left = normal.cross(tangent).normalize_or(DVec3::Y);

        let right_pt = p - left * half_width;
        let left_pt = p + left * half_width;

        // Push right vertex, then left vertex.
        pos_out.push([right_pt.x, right_pt.y, right_pt.z]);
        pos_out.push([left_pt.x, left_pt.y, left_pt.z]);

        if let Some(ref mut norms) = normals_out {
            norms.push([normal.x, normal.y, normal.z]);
            norms.push([normal.x, normal.y, normal.z]);
        }
        if let Some(ref mut tans) = tangents_out {
            tans.push([tangent.x, tangent.y, tangent.z]);
            tans.push([tangent.x, tangent.y, tangent.z]);
        }
        if let Some(ref mut bits) = bitangents_out {
            let bitangent = normal.cross(tangent).normalize_or(DVec3::Y);
            bits.push([bitangent.x, bitangent.y, bitangent.z]);
            bits.push([bitangent.x, bitangent.y, bitangent.z]);
        }
        if let Some(ref mut st) = st_out {
            let s = i as f64 * st_s;
            st.push([s, 0.0]); // right
            st.push([s, 1.0]); // left
        }
    }

    // Triangulate: each quad between consecutive pairs.
    let mut indices: Vec<u32> = Vec::with_capacity((n - 1) * 6);
    for i in 0..n - 1 {
        let r0 = (i * 2) as u32;
        let l0 = (i * 2 + 1) as u32;
        let r1 = ((i + 1) * 2) as u32;
        let l1 = ((i + 1) * 2 + 1) as u32;

        indices.extend_from_slice(&[l0, r0, l1]);
        indices.extend_from_slice(&[l1, r0, r1]);
    }

    let bounding_sphere = BoundingSphere::from_points(
        &pos_out.iter().map(|p| DVec3::new(p[0], p[1], p[2])).collect::<Vec<_>>(),
    );

    GeometryData {
        positions: pos_out,
        normals: normals_out,
        tex_coords: st_out,
        tangents: tangents_out,
        bitangents: bitangents_out,
        indices,
        bounding_sphere,
        primitive_type: PrimitiveType::Triangles,
    }
}

fn empty_geometry() -> GeometryData {
    GeometryData {
        positions: Vec::new(),
        normals: None,
        tex_coords: None,
        tangents: None,
        bitangents: None,
        indices: Vec::new(),
        bounding_sphere: BoundingSphere::default(),
        primitive_type: PrimitiveType::Triangles,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartographic::Cartographic;

    fn polyline_opts() -> PolylineOptions {
        let ell = Ellipsoid::WGS84;
        let positions = vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(5.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(5.0, 5.0, 0.0)),
        ];
        PolylineOptions {
            positions,
            width: 10_000.0,
            granularity: std::f64::consts::PI / 180.0,
            ellipsoid: ell,
        }
    }

    #[test]
    fn test_polyline_basic() {
        let geo = polyline_geometry(&polyline_opts(), VertexFormat::ALL);
        assert!(!geo.positions.is_empty());
        assert_eq!(geo.primitive_type, PrimitiveType::Triangles);
        assert_eq!(geo.indices.len() % 3, 0);
        // Each arc point generates 2 vertices.
        assert_eq!(geo.positions.len() % 2, 0);
        assert!(geo.normals.is_some());
        assert!(geo.tex_coords.is_some());
        assert!(geo.tangents.is_some());
        assert!(geo.bitangents.is_some());
    }

    #[test]
    fn test_polyline_vertex_count() {
        let geo = polyline_geometry(&polyline_opts(), VertexFormat::POSITION_ONLY);
        let n_verts = geo.positions.len();
        // n_verts = 2 * arc_points, indices = 6 * (arc_points - 1)
        let arc_points = n_verts / 2;
        assert_eq!(geo.indices.len(), (arc_points - 1) * 6);
    }

    #[test]
    fn test_polyline_too_few_positions() {
        let ell = Ellipsoid::WGS84;
        let opts = PolylineOptions {
            positions: vec![ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0))],
            width: 100.0,
            ..Default::default()
        };
        let geo = polyline_geometry(&opts, VertexFormat::POSITION_ONLY);
        assert!(geo.positions.is_empty());
    }

    #[test]
    fn test_polyline_zero_width() {
        let ell = Ellipsoid::WGS84;
        let positions = vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 0.0, 0.0)),
        ];
        let opts = PolylineOptions {
            positions,
            width: 0.0,
            ..Default::default()
        };
        let geo = polyline_geometry(&opts, VertexFormat::POSITION_ONLY);
        assert!(geo.positions.is_empty());
    }

    #[test]
    fn test_polyline_normals_outward() {
        let ell = Ellipsoid::WGS84;
        let geo = polyline_geometry(&polyline_opts(), VertexFormat::ALL);
        let normals = geo.normals.unwrap();
        // All normals should point roughly outward (positive dot with position).
        for (i, p) in geo.positions.iter().enumerate() {
            let pos = DVec3::new(p[0], p[1], p[2]);
            let n = DVec3::new(normals[i][0], normals[i][1], normals[i][2]);
            assert!(pos.dot(n) > 0.0, "normal not outward at {}", i);
        }
    }
}
