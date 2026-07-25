//! Ground polyline geometry - a polyline clamped to the ellipsoid surface.
//!
//! Simplified adaptation of CesiumJS `GroundPolylineGeometry.js`. The full
//! CesiumJS version intersects with terrain/3D Tiles; here we clamp to the
//! ellipsoid surface and generate a renderable ribbon.

use crate::bounding::BoundingSphere;
use crate::ellipsoid::Ellipsoid;
use crate::geometry::{GeometryData, PrimitiveType, VertexFormat};
use crate::math_utils::EPSILON10;
use crate::polyline_pipeline::{generate_arc, ArcOptions};
use glam::DVec3;

/// Options describing a ground polyline.
#[derive(Debug, Clone)]
pub struct GroundPolylineOptions {
    /// The polyline positions (at least 2). Heights are ignored.
    pub positions: Vec<DVec3>,
    /// Width in meters.
    pub width: f64,
    /// Angular granularity in radians for arc subdivision.
    pub granularity: f64,
    /// Whether to close the loop (connect last to first).
    pub closed: bool,
    /// The reference ellipsoid.
    pub ellipsoid: Ellipsoid,
}

impl Default for GroundPolylineOptions {
    fn default() -> Self {
        Self {
            positions: Vec::new(),
            width: 1.0,
            granularity: std::f64::consts::PI / 180.0,
            closed: false,
            ellipsoid: Ellipsoid::WGS84,
        }
    }
}

/// Generates a ground polyline geometry clamped to the ellipsoid surface.
///
/// Maps to CesiumJS `GroundPolylineGeometry.createGeometry` (simplified).
pub fn ground_polyline_geometry(options: &GroundPolylineOptions, vf: VertexFormat) -> GeometryData {
    let ellipsoid = &options.ellipsoid;
    let width = options.width;

    // Scale positions to surface (ignore heights).
    let mut positions: Vec<DVec3> = options
        .positions
        .iter()
        .map(|&p| ellipsoid.scale_to_geodetic_surface(p).unwrap_or(p))
        .collect();

    // Remove duplicates.
    positions.dedup_by(|a, b| {
        (a.x - b.x).abs() <= EPSILON10
            && (a.y - b.y).abs() <= EPSILON10
            && (a.z - b.z).abs() <= EPSILON10
    });

    if positions.len() < 2 || width <= 0.0 {
        return empty_geometry();
    }

    // Close the loop if requested.
    if options.closed && positions.len() > 2 {
        positions.push(positions[0]);
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

    // Generate ribbon vertices.
    let mut pos_out: Vec<[f64; 3]> = Vec::with_capacity(n * 2);
    let mut normals_out: Option<Vec<[f64; 3]>> = if vf.normal { Some(Vec::with_capacity(n * 2)) } else { None };
    let mut tangents_out: Option<Vec<[f64; 3]>> = if vf.tangent { Some(Vec::with_capacity(n * 2)) } else { None };
    let mut bitangents_out: Option<Vec<[f64; 3]>> = if vf.bitangent { Some(Vec::with_capacity(n * 2)) } else { None };
    let mut st_out: Option<Vec<[f64; 2]>> = if vf.st { Some(Vec::with_capacity(n * 2)) } else { None };

    let st_s = if n > 1 { 1.0 / (n - 1) as f64 } else { 1.0 };

    for i in 0..n {
        let p = arc[i];
        let normal = ellipsoid.geodetic_surface_normal(p).unwrap_or(DVec3::Z);

        let tangent = if i == 0 {
            (arc[1] - arc[0]).normalize_or(DVec3::X)
        } else if i == n - 1 {
            (arc[n - 1] - arc[n - 2]).normalize_or(DVec3::X)
        } else {
            (arc[i + 1] - arc[i - 1]).normalize_or(DVec3::X)
        };

        let left = normal.cross(tangent).normalize_or(DVec3::Y);

        let right_pt = p - left * half_width;
        let left_pt = p + left * half_width;

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
            st.push([s, 0.0]);
            st.push([s, 1.0]);
        }
    }

    // Triangulate.
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

    fn ground_opts() -> GroundPolylineOptions {
        let ell = Ellipsoid::WGS84;
        let positions = vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(-112.0, 36.0, 1000.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(-111.0, 36.5, 2000.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(-110.0, 36.0, 500.0)),
        ];
        GroundPolylineOptions {
            positions,
            width: 5000.0,
            granularity: std::f64::consts::PI / 180.0,
            closed: false,
            ellipsoid: ell,
        }
    }

    #[test]
    fn test_ground_polyline_basic() {
        let geo = ground_polyline_geometry(&ground_opts(), VertexFormat::ALL);
        assert!(!geo.positions.is_empty());
        assert_eq!(geo.primitive_type, PrimitiveType::Triangles);
        assert_eq!(geo.indices.len() % 3, 0);
        assert_eq!(geo.positions.len() % 2, 0);
        assert!(geo.normals.is_some());
    }

    #[test]
    fn test_ground_polyline_on_surface() {
        let ell = Ellipsoid::WGS84;
        let geo = ground_polyline_geometry(&ground_opts(), VertexFormat::POSITION_ONLY);
        // All positions should be on the ellipsoid surface (within tolerance).
        for p in &geo.positions {
            let pos = DVec3::new(p[0], p[1], p[2]);
            let surface = ell.scale_to_geodetic_surface(pos).unwrap_or(pos);
            let dist = (pos - surface).length();
            // Allow small offset due to width expansion.
            assert!(dist < 5000.0, "position too far from surface: {}", dist);
        }
    }

    #[test]
    fn test_ground_polyline_loop() {
        let ell = Ellipsoid::WGS84;
        let positions = vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.5, 1.0, 0.0)),
        ];
        let opts = GroundPolylineOptions {
            positions,
            width: 1000.0,
            closed: true,
            ..Default::default()
        };
        let geo = ground_polyline_geometry(&opts, VertexFormat::POSITION_ONLY);
        assert!(!geo.positions.is_empty());
        // Loop should have more vertices than non-loop.
        let opts_no_loop = GroundPolylineOptions {
            closed: false,
            ..opts.clone()
        };
        let geo_no_loop = ground_polyline_geometry(&opts_no_loop, VertexFormat::POSITION_ONLY);
        assert!(geo.positions.len() > geo_no_loop.positions.len());
    }

    #[test]
    fn test_ground_polyline_too_few_positions() {
        let ell = Ellipsoid::WGS84;
        let opts = GroundPolylineOptions {
            positions: vec![ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0))],
            width: 100.0,
            ..Default::default()
        };
        let geo = ground_polyline_geometry(&opts, VertexFormat::POSITION_ONLY);
        assert!(geo.positions.is_empty());
    }

    #[test]
    fn test_ground_polyline_zero_width() {
        let ell = Ellipsoid::WGS84;
        let positions = vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 0.0, 0.0)),
        ];
        let opts = GroundPolylineOptions {
            positions,
            width: 0.0,
            ..Default::default()
        };
        let geo = ground_polyline_geometry(&opts, VertexFormat::POSITION_ONLY);
        assert!(geo.positions.is_empty());
    }
}
