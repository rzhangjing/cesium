//! Ellipse / circle geometry on the ellipsoid surface.
//!
//! Faithful port of CesiumJS `EllipseGeometryLibrary.js`, `EllipseGeometry.js`
//! and `EllipseOutlineGeometry.js`. The ellipse is tessellated in "columns"
//! that run east→west: the first and last columns hold a single position (the
//! eastern-/western-most points) and every interior column holds an even number
//! of positions, producing the characteristic diamond-ish fan shown in the
//! CesiumJS source comments.

use crate::bounding::BoundingSphere;
use crate::ellipsoid::Ellipsoid;
use crate::geometry::{GeometryData, PrimitiveType, VertexFormat};
use crate::projection::{GeographicProjection, MapProjection};
use glam::{DMat3, DQuat, DVec3};

/// Computes a single point on the boundary of the ellipse.
///
/// Port of `pointOnEllipsoid` in `EllipseGeometryLibrary.js`. Given a parameter
/// angle `theta`, it finds the point on the ellipsoid surface that lies on the
/// ellipse boundary by rotating the centre's unit position vector about an axis
/// in the local east/north plane by the ellipse's angular radius at `theta`.
#[allow(clippy::too_many_arguments)]
fn point_on_ellipsoid(
    theta: f64,
    rotation: f64,
    north_vec: DVec3,
    east_vec: DVec3,
    a_sqr: f64,
    ab: f64,
    b_sqr: f64,
    mag: f64,
    unit_pos: DVec3,
) -> DVec3 {
    let azimuth = theta + rotation;

    let rot_axis = east_vec * azimuth.cos() + north_vec * azimuth.sin();

    let cos_theta_squared = theta.cos() * theta.cos();
    let sin_theta_squared = theta.sin() * theta.sin();

    let radius = ab / (b_sqr * cos_theta_squared + a_sqr * sin_theta_squared).sqrt();
    let angle = radius / mag;

    // Rotate the position vector to the boundary of the ellipse.
    let unit_quat = DQuat::from_axis_angle(rot_axis.normalize(), angle);
    let rot_mtx = DMat3::from_quat(unit_quat);

    let mut result = rot_mtx * unit_pos;
    result = result.normalize() * mag;
    result
}

/// Result of [`compute_ellipse_positions`].
pub struct EllipsePositions {
    /// Fill positions (column-major tessellation), if requested.
    pub positions: Vec<[f64; 3]>,
    /// Number of points in the first quadrant (drives the tessellation).
    pub num_pts: usize,
    /// Outer boundary positions (ordered ring), if requested.
    pub outer_positions: Vec<[f64; 3]>,
}

/// Computes the positions that make up the ellipse.
///
/// Port of `EllipseGeometryLibrary.computeEllipsePositions`.
///
/// * `semi_minor_axis` / `semi_major_axis` – ellipse radii in metres.
/// * `rotation` – rotation of the ellipse about its centre (radians).
/// * `center` – centre position (cartesian, on/near the ellipsoid).
/// * `granularity` – angular granularity (radians); scaled by 8 internally.
/// * `add_fill_positions` – produce the filled tessellation positions.
/// * `add_edge_positions` – produce the outer boundary ring positions.
pub fn compute_ellipse_positions(
    semi_minor_axis: f64,
    semi_major_axis: f64,
    rotation: f64,
    center: DVec3,
    granularity: f64,
    add_fill_positions: bool,
    add_edge_positions: bool,
) -> EllipsePositions {
    // Scaling the angle delta makes the distance along the ellipse boundary
    // more closely match the granularity (see CesiumJS comment).
    let granularity = granularity * 8.0;

    let a_sqr = semi_minor_axis * semi_minor_axis;
    let b_sqr = semi_major_axis * semi_major_axis;
    let ab = semi_major_axis * semi_minor_axis;

    let mag = center.length();

    let unit_pos = center.normalize();
    let east_vec = DVec3::Z.cross(center).normalize();
    let north_vec = unit_pos.cross(east_vec);

    // The number of points in the first quadrant.
    let mut num_pts = 1 + (std::f64::consts::FRAC_PI_2 / granularity).ceil() as usize;

    let delta_theta = std::f64::consts::FRAC_PI_2 / (num_pts - 1) as f64;
    let theta = std::f64::consts::FRAC_PI_2 - num_pts as f64 * delta_theta;
    if theta < 0.0 {
        num_pts -= (theta.abs() / delta_theta).ceil() as usize;
    }

    let size = 2 * (num_pts * (num_pts + 2));
    let mut positions: Vec<[f64; 3]> = if add_fill_positions {
        Vec::with_capacity(size)
    } else {
        Vec::new()
    };

    let outer_positions_length = num_pts * 4;
    // The outer ring is filled from both ends towards the middle.
    let mut outer_positions: Vec<[f64; 3]> = if add_edge_positions {
        vec![[0.0; 3]; outer_positions_length]
    } else {
        Vec::new()
    };
    let mut outer_right_index = outer_positions_length; // exclusive, decrements
    let mut outer_left_index = 0usize;

    // Compute points in the 'eastern' half of the ellipse.
    let mut theta = std::f64::consts::FRAC_PI_2;
    let position = point_on_ellipsoid(
        theta, rotation, north_vec, east_vec, a_sqr, ab, b_sqr, mag, unit_pos,
    );
    if add_fill_positions {
        positions.push([position.x, position.y, position.z]);
    }
    if add_edge_positions {
        outer_right_index -= 1;
        outer_positions[outer_right_index] = [position.x, position.y, position.z];
    }

    theta = std::f64::consts::FRAC_PI_2 - delta_theta;
    for i in 1..num_pts + 1 {
        let position = point_on_ellipsoid(
            theta, rotation, north_vec, east_vec, a_sqr, ab, b_sqr, mag, unit_pos,
        );
        let reflected_position = point_on_ellipsoid(
            std::f64::consts::PI - theta,
            rotation,
            north_vec,
            east_vec,
            a_sqr,
            ab,
            b_sqr,
            mag,
            unit_pos,
        );

        if add_fill_positions {
            positions.push([position.x, position.y, position.z]);

            let num_interior = 2 * i + 2;
            for j in 1..num_interior - 1 {
                let t = j as f64 / (num_interior - 1) as f64;
                let interior = position.lerp(reflected_position, t);
                positions.push([interior.x, interior.y, interior.z]);
            }

            positions.push([reflected_position.x, reflected_position.y, reflected_position.z]);
        }

        if add_edge_positions {
            outer_right_index -= 1;
            outer_positions[outer_right_index] = [position.x, position.y, position.z];
            outer_positions[outer_left_index] =
                [reflected_position.x, reflected_position.y, reflected_position.z];
            outer_left_index += 1;
        }

        theta = std::f64::consts::FRAC_PI_2 - (i + 1) as f64 * delta_theta;
    }

    // Compute points in the 'western' half of the ellipse.
    for i in (2..=num_pts).rev() {
        let theta = std::f64::consts::FRAC_PI_2 - (i - 1) as f64 * delta_theta;

        let position = point_on_ellipsoid(
            -theta, rotation, north_vec, east_vec, a_sqr, ab, b_sqr, mag, unit_pos,
        );
        let reflected_position = point_on_ellipsoid(
            theta + std::f64::consts::PI,
            rotation,
            north_vec,
            east_vec,
            a_sqr,
            ab,
            b_sqr,
            mag,
            unit_pos,
        );

        if add_fill_positions {
            positions.push([position.x, position.y, position.z]);

            let num_interior = 2 * (i - 1) + 2;
            for j in 1..num_interior - 1 {
                let t = j as f64 / (num_interior - 1) as f64;
                let interior = position.lerp(reflected_position, t);
                positions.push([interior.x, interior.y, interior.z]);
            }

            positions.push([reflected_position.x, reflected_position.y, reflected_position.z]);
        }

        if add_edge_positions {
            outer_right_index -= 1;
            outer_positions[outer_right_index] = [position.x, position.y, position.z];
            outer_positions[outer_left_index] =
                [reflected_position.x, reflected_position.y, reflected_position.z];
            outer_left_index += 1;
        }
    }

    let theta = -std::f64::consts::FRAC_PI_2;
    let position = point_on_ellipsoid(
        theta, rotation, north_vec, east_vec, a_sqr, ab, b_sqr, mag, unit_pos,
    );
    if add_fill_positions {
        positions.push([position.x, position.y, position.z]);
    }
    if add_edge_positions {
        outer_right_index -= 1;
        outer_positions[outer_right_index] = [position.x, position.y, position.z];
    }

    EllipsePositions {
        positions,
        num_pts,
        outer_positions,
    }
}

/// Generates the triangle indices for the filled ellipse tessellation.
///
/// Port of `topIndices` in `EllipseGeometry.js`. The index arithmetic mirrors
/// the column layout produced by [`compute_ellipse_positions`].
pub fn top_indices(num_pts: usize) -> Vec<u32> {
    // total triangles = 2 * (-1 + 4 * (n*(n+1)/2)); indices = triangles * 3.
    let total = 12 * (num_pts * (num_pts + 1)) - 6;
    let mut indices: Vec<u32> = Vec::with_capacity(total);

    let mut prev_index: u32 = 0;
    let mut position_index: u32 = 1;

    // Triangles to the 'right' of the north vector (first fan).
    for _ in 0..3 {
        indices.push(position_index);
        position_index += 1;
        indices.push(prev_index);
        indices.push(position_index);
    }

    for i in 2..num_pts + 1 {
        position_index = (i * (i + 1) - 1) as u32;
        prev_index = ((i - 1) * i - 1) as u32;

        indices.push(position_index);
        position_index += 1;
        indices.push(prev_index);
        indices.push(position_index);

        let num_interior = 2 * i;
        for _ in 0..num_interior - 1 {
            indices.push(position_index);
            indices.push(prev_index);
            prev_index += 1;
            indices.push(prev_index);

            indices.push(position_index);
            position_index += 1;
            indices.push(prev_index);
            indices.push(position_index);
        }

        indices.push(position_index);
        position_index += 1;
        indices.push(prev_index);
        indices.push(position_index);
    }

    // Indices for the centre column of triangles.
    let num_interior = num_pts * 2;
    position_index += 1;
    prev_index += 1;
    for _ in 0..num_interior - 1 {
        indices.push(position_index);
        indices.push(prev_index);
        prev_index += 1;
        indices.push(prev_index);

        indices.push(position_index);
        position_index += 1;
        indices.push(prev_index);
        indices.push(position_index);
    }

    indices.push(position_index);
    indices.push(prev_index);
    prev_index += 1;
    indices.push(prev_index);

    indices.push(position_index);
    position_index += 1;
    indices.push(prev_index);
    prev_index += 1;
    indices.push(prev_index);

    // Reverse the process creating indices to the 'left' of the north vector.
    prev_index += 1;
    for i in (2..=num_pts - 1).rev() {
        indices.push(prev_index);
        prev_index += 1;
        indices.push(prev_index);
        indices.push(position_index);

        let num_interior = 2 * i;
        for _ in 0..num_interior - 1 {
            indices.push(position_index);
            indices.push(prev_index);
            prev_index += 1;
            indices.push(prev_index);

            indices.push(position_index);
            position_index += 1;
            indices.push(prev_index);
            indices.push(position_index);
        }

        indices.push(prev_index);
        prev_index += 1;
        indices.push(prev_index);
        prev_index += 1;
        indices.push(position_index);
        position_index += 1;
    }

    for _ in 0..3 {
        indices.push(prev_index);
        prev_index += 1;
        indices.push(prev_index);
        indices.push(position_index);
    }

    indices
}

/// Options for ellipse geometry generation.
pub struct EllipseOptions {
    /// Centre position (cartesian).
    pub center: DVec3,
    /// Semi-major axis in metres.
    pub semi_major_axis: f64,
    /// Semi-minor axis in metres.
    pub semi_minor_axis: f64,
    /// Ellipsoid.
    pub ellipsoid: Ellipsoid,
    /// Angular granularity in radians.
    pub granularity: f64,
    /// Height above the ellipsoid in metres.
    pub height: f64,
    /// Rotation of the ellipse about its centre in radians.
    pub rotation: f64,
    /// Texture-coordinate rotation in radians.
    pub st_rotation: f64,
}

impl Default for EllipseOptions {
    fn default() -> Self {
        Self {
            center: DVec3::ZERO,
            semi_major_axis: 1.0,
            semi_minor_axis: 1.0,
            ellipsoid: Ellipsoid::WGS84,
            granularity: crate::math_utils::to_radians(1.0),
            height: 0.0,
            rotation: 0.0,
            st_rotation: 0.0,
        }
    }
}

/// Generates a filled ellipse geometry on the ellipsoid.
///
/// Maps to CesiumJS `EllipseGeometry`. `CircleGeometry` is the special case
/// where `semi_major_axis == semi_minor_axis`.
pub fn ellipse_geometry(options: &EllipseOptions, vf: VertexFormat) -> GeometryData {
    let ellipsoid = options.ellipsoid;

    let cep = compute_ellipse_positions(
        options.semi_minor_axis,
        options.semi_major_axis,
        options.rotation,
        options.center,
        options.granularity,
        true,
        false,
    );
    let num_pts = cep.num_pts;

    // Raise positions to height and compute attributes.
    let positions = raise_positions_to_height(&cep.positions, &ellipsoid, options.height);

    let indices = top_indices(num_pts);

    let projection = GeographicProjection::new(ellipsoid);
    let center_carto = ellipsoid
        .cartesian_to_cartographic(options.center)
        .unwrap_or_default();
    let projected_center = projection.project(&center_carto);

    let mut tex_coords: Option<Vec<[f64; 2]>> = if vf.st { Some(Vec::new()) } else { None };
    let mut normals: Option<Vec<[f64; 3]>> = if vf.normal { Some(Vec::new()) } else { None };

    for p in &positions {
        let pos = DVec3::new(p[0], p[1], p[2]);

        if let Some(ref mut st) = tex_coords {
            let carto = ellipsoid.cartesian_to_cartographic(pos).unwrap_or_default();
            let projected = projection.project(&carto);
            let rel = projected - projected_center;
            let u = (rel.x + options.semi_major_axis) / (2.0 * options.semi_major_axis);
            let v = (rel.y + options.semi_minor_axis) / (2.0 * options.semi_minor_axis);
            st.push([u, v]);
        }

        if let Some(ref mut n) = normals {
            let normal = ellipsoid.geodetic_surface_normal(pos).unwrap_or(DVec3::Z);
            n.push([normal.x, normal.y, normal.z]);
        }
    }

    // Bounding sphere: centre raised to height, radius = semi-major axis.
    let bs_center = options
        .center
        + ellipsoid
            .geodetic_surface_normal(options.center)
            .unwrap_or(DVec3::Z)
            * options.height;
    let bounding_sphere = BoundingSphere::new(bs_center, options.semi_major_axis);

    GeometryData {
        positions,
        normals,
        tex_coords,
        tangents: None,
        bitangents: None,
        indices,
        bounding_sphere,
        primitive_type: PrimitiveType::Triangles,
    }
}

/// Generates an ellipse outline geometry (line segments).
///
/// Maps to CesiumJS `EllipseOutlineGeometry`.
pub fn ellipse_outline_geometry(options: &EllipseOptions) -> GeometryData {
    let ellipsoid = options.ellipsoid;

    let cep = compute_ellipse_positions(
        options.semi_minor_axis,
        options.semi_major_axis,
        options.rotation,
        options.center,
        options.granularity,
        false,
        true,
    );

    let positions = raise_positions_to_height(&cep.outer_positions, &ellipsoid, options.height);

    // Line-loop around the outer ring.
    let n = positions.len();
    let mut indices: Vec<u32> = Vec::with_capacity(n * 2);
    for i in 0..n {
        indices.push(i as u32);
        indices.push(((i + 1) % n) as u32);
    }

    let bs_center = options
        .center
        + ellipsoid
            .geodetic_surface_normal(options.center)
            .unwrap_or(DVec3::Z)
            * options.height;
    let bounding_sphere = BoundingSphere::new(bs_center, options.semi_major_axis);

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

/// Raises positions to the given height above the ellipsoid surface.
///
/// Port of `EllipseGeometryLibrary.raisePositionsToHeight` (non-extruded case).
fn raise_positions_to_height(positions: &[[f64; 3]], ellipsoid: &Ellipsoid, height: f64) -> Vec<[f64; 3]> {
    positions
        .iter()
        .map(|p| {
            let pos = DVec3::new(p[0], p[1], p[2]);
            let surface = ellipsoid.scale_to_geodetic_surface(pos).unwrap_or(pos);
            let normal = ellipsoid.geodetic_surface_normal(surface).unwrap_or(DVec3::Z);
            let raised = surface + normal * height;
            [raised.x, raised.y, raised.z]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math_utils;

    fn equator_center() -> DVec3 {
        Ellipsoid::WGS84.cartographic_to_cartesian(&crate::cartographic::Cartographic::from_degrees(0.0, 0.0, 0.0))
    }

    #[test]
    fn test_ellipse_positions_count() {
        let opts = EllipseOptions {
            center: equator_center(),
            semi_major_axis: 500_000.0,
            semi_minor_axis: 300_000.0,
            granularity: math_utils::to_radians(8.0),
            ..Default::default()
        };
        let cep = compute_ellipse_positions(
            opts.semi_minor_axis,
            opts.semi_major_axis,
            opts.rotation,
            opts.center,
            opts.granularity,
            true,
            true,
        );
        // Fill count must match the column formula 2*n*(n+2).
        assert_eq!(cep.positions.len(), 2 * cep.num_pts * (cep.num_pts + 2));
        // Outer ring count must be 4*n.
        assert_eq!(cep.outer_positions.len(), 4 * cep.num_pts);
    }

    #[test]
    fn test_top_indices_count() {
        for num_pts in 2..8 {
            let indices = top_indices(num_pts);
            let expected = 12 * (num_pts * (num_pts + 1)) - 6;
            assert_eq!(indices.len(), expected, "num_pts={}", num_pts);
            // All indices must be within the fill-position range.
            let max_index = 2 * num_pts * (num_pts + 2);
            assert!(
                indices.iter().all(|&i| (i as usize) < max_index),
                "index out of range for num_pts={}",
                num_pts
            );
        }
    }

    #[test]
    fn test_ellipse_geometry() {
        let opts = EllipseOptions {
            center: equator_center(),
            semi_major_axis: 500_000.0,
            semi_minor_axis: 300_000.0,
            granularity: math_utils::to_radians(8.0),
            ..Default::default()
        };
        let geo = ellipse_geometry(&opts, VertexFormat::ALL);
        assert_eq!(geo.positions.len(), geo.normals.as_ref().unwrap().len());
        assert_eq!(geo.positions.len(), geo.tex_coords.as_ref().unwrap().len());
        assert_eq!(geo.indices.len() % 3, 0);
        assert_eq!(geo.primitive_type, PrimitiveType::Triangles);
        // Bounding sphere radius equals the semi-major axis.
        assert!((geo.bounding_sphere.radius - 500_000.0).abs() < 1e-6);
    }

    #[test]
    fn test_circle_is_ellipse_special_case() {
        let opts = EllipseOptions {
            center: equator_center(),
            semi_major_axis: 400_000.0,
            semi_minor_axis: 400_000.0,
            granularity: math_utils::to_radians(8.0),
            ..Default::default()
        };
        let geo = ellipse_geometry(&opts, VertexFormat::POSITION_ONLY);
        assert!(!geo.positions.is_empty());
        assert!(!geo.indices.is_empty());
    }

    #[test]
    fn test_ellipse_outline_geometry() {
        let opts = EllipseOptions {
            center: equator_center(),
            semi_major_axis: 500_000.0,
            semi_minor_axis: 300_000.0,
            granularity: math_utils::to_radians(8.0),
            ..Default::default()
        };
        let geo = ellipse_outline_geometry(&opts);
        assert_eq!(geo.primitive_type, PrimitiveType::Lines);
        // Line indices come in pairs and form a closed loop.
        assert_eq!(geo.indices.len(), geo.positions.len() * 2);
        assert_eq!(geo.indices.len() % 2, 0);
    }

    #[test]
    fn test_ellipse_positions_on_surface() {
        // Every generated position should be (approximately) on the ellipsoid
        // surface raised to the requested height.
        let opts = EllipseOptions {
            center: equator_center(),
            semi_major_axis: 500_000.0,
            semi_minor_axis: 300_000.0,
            granularity: math_utils::to_radians(8.0),
            height: 1000.0,
            ..Default::default()
        };
        let geo = ellipse_geometry(&opts, VertexFormat::POSITION_ONLY);
        for p in &geo.positions {
            let carto = Ellipsoid::WGS84
                .cartesian_to_cartographic(DVec3::new(p[0], p[1], p[2]))
                .unwrap();
            assert!((carto.height - 1000.0).abs() < 1.0, "height={}", carto.height);
        }
    }

    #[test]
    fn test_ellipse_triangles_non_degenerate() {
        // Every triangle must reference three pairwise-distinct vertices and
        // have non-zero area, confirming the column tessellation is valid.
        let opts = EllipseOptions {
            center: equator_center(),
            semi_major_axis: 500_000.0,
            semi_minor_axis: 300_000.0,
            granularity: math_utils::to_radians(8.0),
            ..Default::default()
        };
        let geo = ellipse_geometry(&opts, VertexFormat::POSITION_ONLY);
        assert!(!geo.indices.is_empty());
        for tri in geo.indices.chunks_exact(3) {
            let (a, b, c) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
            assert!(a != b && b != c && a != c, "degenerate triangle {:?}", tri);
            let pa = DVec3::from(geo.positions[a]);
            let pb = DVec3::from(geo.positions[b]);
            let pc = DVec3::from(geo.positions[c]);
            let area = (pb - pa).cross(pc - pa).length() * 0.5;
            assert!(area > 1e-6, "zero-area triangle {:?}", tri);
        }
    }
}
