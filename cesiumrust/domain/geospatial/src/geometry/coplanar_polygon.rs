//! Coplanar polygon geometry - a polygon from arbitrary coplanar positions.
//!
//! Faithful port of CesiumJS `CoplanarPolygonGeometry.js` and
//! `CoplanarPolygonGeometryLibrary.js`. Projects coplanar 3D positions onto
//! their best-fit plane, triangulates in 2D, and generates the mesh.

use crate::bounding::BoundingSphere;
use crate::ellipsoid::Ellipsoid;
use crate::geometry::{triangulate_polygon, GeometryData, PrimitiveType, VertexFormat};
use crate::math_utils::EPSILON10;
use glam::DVec3;

/// Options describing a coplanar polygon.
#[derive(Debug, Clone)]
pub struct CoplanarPolygonOptions {
    /// The polygon positions (at least 3, must be coplanar).
    pub positions: Vec<DVec3>,
    /// Texture coordinate rotation in radians.
    pub st_rotation: f64,
    /// The reference ellipsoid.
    pub ellipsoid: Ellipsoid,
}

impl Default for CoplanarPolygonOptions {
    fn default() -> Self {
        Self {
            positions: Vec::new(),
            st_rotation: 0.0,
            ellipsoid: Ellipsoid::WGS84,
        }
    }
}

/// Computes the plane normal using Newell's method (robust for arbitrary polygons).
fn compute_normal(positions: &[DVec3]) -> DVec3 {
    let n = positions.len();
    let mut normal = DVec3::ZERO;
    for i in 0..n {
        let current = positions[i];
        let next = positions[(i + 1) % n];
        normal.x += (current.y - next.y) * (current.z + next.z);
        normal.y += (current.z - next.z) * (current.x + next.x);
        normal.z += (current.x - next.x) * (current.y + next.y);
    }
    normal.normalize_or(DVec3::Z)
}

/// Computes the centroid of positions.
fn compute_center(positions: &[DVec3]) -> DVec3 {
    let sum: DVec3 = positions.iter().sum();
    sum / positions.len() as f64
}

/// Generates a coplanar polygon geometry.
///
/// Maps to CesiumJS `CoplanarPolygonGeometry.createGeometry`.
pub fn coplanar_polygon_geometry(options: &CoplanarPolygonOptions, vf: VertexFormat) -> GeometryData {
    let ellipsoid = &options.ellipsoid;

    // Remove duplicates.
    let mut positions: Vec<DVec3> = options.positions.clone();
    positions.dedup_by(|a, b| {
        (a.x - b.x).abs() <= EPSILON10
            && (a.y - b.y).abs() <= EPSILON10
            && (a.z - b.z).abs() <= EPSILON10
    });

    if positions.len() < 3 {
        return empty_geometry();
    }

    // Compute the plane normal and axes.
    let normal = compute_normal(&positions);

    // Ensure normal points outward (away from ellipsoid center).
    let center = compute_center(&positions);
    if center.length_squared() > 1e-12 {
        let surface_normal = ellipsoid.geodetic_surface_normal(center).unwrap_or(DVec3::Z);
        if normal.dot(surface_normal) < 0.0 {
            // Flip normal and axis1 to maintain consistent winding.
            let normal = -normal;
            return build_geometry(&positions, normal, options.st_rotation, &vf);
        }
    }

    build_geometry(&positions, normal, options.st_rotation, &vf)
}

fn build_geometry(
    positions: &[DVec3],
    normal: DVec3,
    st_rotation: f64,
    vf: &VertexFormat,
) -> GeometryData {
    let n = positions.len();

    // Compute plane axes.
    let axis1 = compute_axis1(normal);
    let axis2 = normal.cross(axis1).normalize_or(DVec3::Y);

    // Project positions to 2D.
    let center = compute_center(positions);
    let positions_2d: Vec<glam::DVec2> = positions
        .iter()
        .map(|&p| {
            let v = p - center;
            glam::DVec2::new(v.dot(axis1), v.dot(axis2))
        })
        .collect();

    // Triangulate.
    let indices = triangulate_polygon(&positions_2d, &[]);
    if indices.is_empty() {
        return empty_geometry();
    }

    // Compute bounding rectangle for ST.
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    for p in &positions_2d {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }
    let width = (max_x - min_x).max(1e-10);
    let height = (max_y - min_y).max(1e-10);

    // Apply ST rotation if needed.
    let (cos_r, sin_r) = if st_rotation.abs() > 1e-15 {
        (st_rotation.cos(), st_rotation.sin())
    } else {
        (1.0, 0.0)
    };

    // Generate vertex attributes.
    let mut pos_out: Vec<[f64; 3]> = Vec::with_capacity(n);
    let mut normals_out: Option<Vec<[f64; 3]>> = if vf.normal { Some(Vec::with_capacity(n)) } else { None };
    let mut tangents_out: Option<Vec<[f64; 3]>> = if vf.tangent { Some(Vec::with_capacity(n)) } else { None };
    let mut bitangents_out: Option<Vec<[f64; 3]>> = if vf.bitangent { Some(Vec::with_capacity(n)) } else { None };
    let mut st_out: Option<Vec<[f64; 2]>> = if vf.st { Some(Vec::with_capacity(n)) } else { None };

    for (i, &p) in positions.iter().enumerate() {
        pos_out.push([p.x, p.y, p.z]);

        if let Some(ref mut norms) = normals_out {
            norms.push([normal.x, normal.y, normal.z]);
        }
        if let Some(ref mut tans) = tangents_out {
            tans.push([axis1.x, axis1.y, axis1.z]);
        }
        if let Some(ref mut bits) = bitangents_out {
            bits.push([axis2.x, axis2.y, axis2.z]);
        }
        if let Some(ref mut st) = st_out {
            let p2d = positions_2d[i];
            // Apply rotation around center.
            let rx = p2d.x * cos_r - p2d.y * sin_r;
            let ry = p2d.x * sin_r + p2d.y * cos_r;
            let stx = ((rx - min_x) / width).clamp(0.0, 1.0);
            let sty = ((ry - min_y) / height).clamp(0.0, 1.0);
            st.push([stx, sty]);
        }
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

/// Computes a vector perpendicular to the normal (axis1 of the plane).
fn compute_axis1(normal: DVec3) -> DVec3 {
    // Choose the least-aligned world axis to cross with.
    let candidate = if normal.x.abs() <= normal.y.abs() && normal.x.abs() <= normal.z.abs() {
        DVec3::X
    } else if normal.y.abs() <= normal.z.abs() {
        DVec3::Y
    } else {
        DVec3::Z
    };
    normal.cross(candidate).normalize_or(DVec3::X)
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

    fn coplanar_opts() -> CoplanarPolygonOptions {
        let ell = Ellipsoid::WGS84;
        // A quad on the surface (roughly coplanar for small areas).
        let positions = vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(-72.0, 40.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(-70.0, 40.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(-70.0, 38.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(-72.0, 38.0, 0.0)),
        ];
        CoplanarPolygonOptions {
            positions,
            st_rotation: 0.0,
            ellipsoid: ell,
        }
    }

    #[test]
    fn test_coplanar_basic() {
        let geo = coplanar_polygon_geometry(&coplanar_opts(), VertexFormat::ALL);
        assert!(!geo.positions.is_empty());
        assert_eq!(geo.primitive_type, PrimitiveType::Triangles);
        assert_eq!(geo.indices.len() % 3, 0);
        // 4 vertices, fan triangulation = 2 triangles = 6 indices.
        assert_eq!(geo.positions.len(), 4);
        assert_eq!(geo.indices.len(), 6);
        assert!(geo.normals.is_some());
        assert!(geo.tex_coords.is_some());
    }

    #[test]
    fn test_coplanar_triangle() {
        let ell = Ellipsoid::WGS84;
        let positions = vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.5, 1.0, 0.0)),
        ];
        let opts = CoplanarPolygonOptions {
            positions,
            ..Default::default()
        };
        let geo = coplanar_polygon_geometry(&opts, VertexFormat::POSITION_ONLY);
        assert_eq!(geo.positions.len(), 3);
        assert_eq!(geo.indices.len(), 3);
    }

    #[test]
    fn test_coplanar_too_few_positions() {
        let ell = Ellipsoid::WGS84;
        let positions = vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 0.0, 0.0)),
        ];
        let opts = CoplanarPolygonOptions {
            positions,
            ..Default::default()
        };
        let geo = coplanar_polygon_geometry(&opts, VertexFormat::POSITION_ONLY);
        assert!(geo.positions.is_empty());
    }

    #[test]
    fn test_coplanar_normals_consistent() {
        let geo = coplanar_polygon_geometry(&coplanar_opts(), VertexFormat::ALL);
        let normals = geo.normals.unwrap();
        // All normals should be the same (coplanar polygon).
        let n0 = DVec3::new(normals[0][0], normals[0][1], normals[0][2]);
        for n in &normals {
            let ni = DVec3::new(n[0], n[1], n[2]);
            assert!((n0 - ni).length() < 1e-10);
        }
        // Normal should point outward.
        let center = compute_center(
            &geo.positions.iter().map(|p| DVec3::new(p[0], p[1], p[2])).collect::<Vec<_>>(),
        );
        assert!(center.dot(n0) > 0.0);
    }

    #[test]
    fn test_coplanar_pentagon() {
        let ell = Ellipsoid::WGS84;
        let positions = vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(1.5, 0.5, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.5, 1.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(-0.5, 0.5, 0.0)),
        ];
        let opts = CoplanarPolygonOptions {
            positions,
            ..Default::default()
        };
        let geo = coplanar_polygon_geometry(&opts, VertexFormat::POSITION_ONLY);
        assert_eq!(geo.positions.len(), 5);
        // Fan triangulation: 5-2 = 3 triangles = 9 indices.
        assert_eq!(geo.indices.len(), 9);
    }
}
