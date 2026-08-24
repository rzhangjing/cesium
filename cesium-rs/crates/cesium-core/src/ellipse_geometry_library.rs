//! Ported from `packages/engine/Source/Core/EllipseGeometryLibrary.js`.
//!
//! Library functions shared by `CircleGeometry`, `EllipseGeometry` and their
//! outline variants: raising positions to a height and computing the ellipse
//! boundary/fill positions on the ellipsoid.

use crate::cartesian3::Cartesian3;
use crate::ellipsoid::Ellipsoid;
use crate::math::CesiumMath;
use crate::matrix3::Matrix3;
use crate::quaternion::Quaternion;

/// Private namespace `EllipseGeometryLibrary`.
pub struct EllipseGeometryLibrary;

/// JS `pointOnEllipsoid` helper (module-private).
#[allow(clippy::too_many_arguments)]
fn point_on_ellipsoid<'a>(
    theta: f64,
    rotation: f64,
    north_vec: &Cartesian3,
    east_vec: &Cartesian3,
    a_sqr: f64,
    ab: f64,
    b_sqr: f64,
    mag: f64,
    unit_pos: &Cartesian3,
    result: &'a mut Cartesian3,
) -> &'a mut Cartesian3 {
    let azimuth = theta + rotation;

    let mut rot_axis = Cartesian3::ZERO;
    Cartesian3::multiply_by_scalar(east_vec, azimuth.cos(), &mut rot_axis);
    let mut temp_vec = Cartesian3::ZERO;
    Cartesian3::multiply_by_scalar(north_vec, azimuth.sin(), &mut temp_vec);
    // DEVIATION: JS adds into `rotAxis` in place; Rust needs a temporary.
    let mut rot_axis_sum = Cartesian3::ZERO;
    Cartesian3::add(&rot_axis, &temp_vec, &mut rot_axis_sum);
    rot_axis = rot_axis_sum;

    let mut cos_theta_squared = theta.cos();
    cos_theta_squared *= cos_theta_squared;

    let mut sin_theta_squared = theta.sin();
    sin_theta_squared *= sin_theta_squared;

    let radius = ab / (b_sqr * cos_theta_squared + a_sqr * sin_theta_squared).sqrt();
    let angle = radius / mag;

    // Create the quaternion to rotate the position vector to the boundary of the ellipse.
    let mut unit_quat = Quaternion::default();
    Quaternion::from_axis_angle(&rot_axis, angle, &mut unit_quat);
    let mut rot_mtx = Matrix3::default();
    Matrix3::from_quaternion(&unit_quat, &mut rot_mtx);

    Matrix3::multiply_by_vector(&rot_mtx, unit_pos, result);
    // DEVIATION: JS normalizes/scales in place; Rust needs temporaries.
    let mut normalized = Cartesian3::ZERO;
    Cartesian3::normalize(result, &mut normalized);
    Cartesian3::multiply_by_scalar(&normalized, mag, result);
    result
}

/// Port of `EllipseGeometryLibrary.raisePositionsToHeight` — returns the
/// positions raised to the given heights (doubled when `extrude`).
///
/// DEVIATION: JS takes the geometry `options` object; this port takes the
/// three fields it reads (`ellipsoid`, `height`, `extrudedHeight`) directly.
pub fn raise_positions_to_height(
    positions: &[f64],
    ellipsoid: &Ellipsoid,
    height: f64,
    extruded_height: f64,
    extrude: bool,
) -> Vec<f64> {
    let size = if extrude {
        (positions.len() / 3) * 2
    } else {
        positions.len() / 3
    };

    let mut final_positions = vec![0.0f64; size * 3];

    let length = positions.len();
    let bottom_offset = if extrude { length } else { 0 };
    let mut i = 0;
    while i < length {
        let i1 = i + 1;
        let i2 = i + 2;

        let mut position = Cartesian3::ZERO;
        Cartesian3::from_array(positions, Some(i), &mut position);
        let mut scaled = Cartesian3::ZERO;
        ellipsoid.scale_to_geodetic_surface(&position, &mut scaled);
        position = scaled;

        let extruded_position = position;
        let mut normal = Cartesian3::ZERO;
        ellipsoid.geodetic_surface_normal(&position, &mut normal);
        let mut scaled_normal = Cartesian3::ZERO;
        Cartesian3::multiply_by_scalar(&normal, height, &mut scaled_normal);
        // DEVIATION: JS adds into `position` in place.
        let mut raised = Cartesian3::ZERO;
        Cartesian3::add(&position, &scaled_normal, &mut raised);
        position = raised;

        if extrude {
            Cartesian3::multiply_by_scalar(&normal, extruded_height, &mut scaled_normal);
            let mut extruded = Cartesian3::ZERO;
            Cartesian3::add(&extruded_position, &scaled_normal, &mut extruded);

            final_positions[i + bottom_offset] = extruded.x;
            final_positions[i1 + bottom_offset] = extruded.y;
            final_positions[i2 + bottom_offset] = extruded.z;
        }

        final_positions[i] = position.x;
        final_positions[i1] = position.y;
        final_positions[i2] = position.z;

        i += 3;
    }

    final_positions
}

/// Options read by [`EllipseGeometryLibrary::compute_ellipse_positions`]
/// (JS anonymous `options` object).
pub struct EllipseGeometryOptions {
    pub semi_minor_axis: f64,
    pub semi_major_axis: f64,
    pub rotation: f64,
    pub center: Cartesian3,
    pub granularity: f64,
}

/// Result of [`EllipseGeometryLibrary::compute_ellipse_positions`].
pub struct ComputeEllipsePositionsResult {
    pub positions: Option<Vec<f64>>,
    pub num_pts: usize,
    pub outer_positions: Option<Vec<f64>>,
}

impl EllipseGeometryLibrary {
    /// Port of `EllipseGeometryLibrary.computeEllipsePositions` — returns the
    /// positions that make up the ellipse (fill and/or edge).
    pub fn compute_ellipse_positions(
        options: &EllipseGeometryOptions,
        add_fill_positions: bool,
        add_edge_positions: bool,
    ) -> ComputeEllipsePositionsResult {
        let semi_minor_axis = options.semi_minor_axis;
        let semi_major_axis = options.semi_major_axis;
        let rotation = options.rotation;
        let center = &options.center;

        // Computing the arc-length of the ellipse is too expensive to be
        // practical. Estimating it using the arc length of the sphere is too
        // inaccurate and creates sharp edges when either the semi-major or
        // semi-minor axis is much bigger than the other. Instead, scale the
        // angle delta to make the distance along the ellipse boundary more
        // closely match the granularity.
        let granularity = options.granularity * 8.0;

        let a_sqr = semi_minor_axis * semi_minor_axis;
        let b_sqr = semi_major_axis * semi_major_axis;
        let ab = semi_major_axis * semi_minor_axis;

        let mag = Cartesian3::magnitude(center);

        let mut unit_pos = Cartesian3::ZERO;
        Cartesian3::normalize(center, &mut unit_pos);
        let mut east_vec = Cartesian3::ZERO;
        Cartesian3::cross(&Cartesian3::UNIT_Z, center, &mut east_vec);
        let mut east_normalized = Cartesian3::ZERO;
        Cartesian3::normalize(&east_vec, &mut east_normalized);
        east_vec = east_normalized;
        let mut north_vec = Cartesian3::ZERO;
        Cartesian3::cross(&unit_pos, &east_vec, &mut north_vec);

        // The number of points in the first quadrant
        let mut num_pts = 1 + (CesiumMath::PI_OVER_TWO / granularity).ceil() as usize;

        let delta_theta = CesiumMath::PI_OVER_TWO / (num_pts - 1) as f64;
        let theta = CesiumMath::PI_OVER_TWO - num_pts as f64 * delta_theta;
        if theta < 0.0 {
            num_pts -= (theta.abs() / delta_theta).ceil() as usize;
        }

        // If the number of points were three, the ellipse
        // would be tessellated like below:
        //
        //         *---*
        //       / | \ | \
        //     *---*---*---*
        //   / | \ | \ | \ | \
        //  / .*---*---*---*. \
        // * ` | \ | \ | \ | `*
        //  \`.*---*---*---*.`/
        //   \ | \ | \ | \ | /
        //     *---*---*---*
        //       \ | \ | /
        //         *---*
        // The first and last column have one position and fan to connect to the adjacent column.
        // Each other vertical column contains an even number of positions.
        let size = 2 * (num_pts * (num_pts + 2));
        let mut positions: Option<Vec<f64>> = if add_fill_positions {
            Some(Vec::with_capacity(size * 3))
        } else {
            None
        };

        let outer_positions_length = num_pts * 4 * 3;
        let mut outer_right_index = outer_positions_length;
        let mut outer_left_index = 0usize;
        let mut outer_positions: Option<Vec<f64>> = if add_edge_positions {
            Some(vec![0.0; outer_positions_length])
        } else {
            None
        };

        let mut position = Cartesian3::ZERO;
        let mut reflected_position = Cartesian3::ZERO;

        // Compute points in the 'eastern' half of the ellipse
        let mut theta = CesiumMath::PI_OVER_TWO;
        point_on_ellipsoid(
            theta,
            rotation,
            &north_vec,
            &east_vec,
            a_sqr,
            ab,
            b_sqr,
            mag,
            &unit_pos,
            &mut position,
        );
        if let Some(positions) = &mut positions {
            positions.push(position.x);
            positions.push(position.y);
            positions.push(position.z);
        }
        if let Some(outer_positions) = &mut outer_positions {
            outer_right_index -= 1;
            outer_positions[outer_right_index] = position.z;
            outer_right_index -= 1;
            outer_positions[outer_right_index] = position.y;
            outer_right_index -= 1;
            outer_positions[outer_right_index] = position.x;
        }
        theta = CesiumMath::PI_OVER_TWO - delta_theta;
        for i in 1..num_pts + 1 {
            point_on_ellipsoid(
                theta,
                rotation,
                &north_vec,
                &east_vec,
                a_sqr,
                ab,
                b_sqr,
                mag,
                &unit_pos,
                &mut position,
            );
            point_on_ellipsoid(
                std::f64::consts::PI - theta,
                rotation,
                &north_vec,
                &east_vec,
                a_sqr,
                ab,
                b_sqr,
                mag,
                &unit_pos,
                &mut reflected_position,
            );

            if let Some(positions) = &mut positions {
                positions.push(position.x);
                positions.push(position.y);
                positions.push(position.z);

                let num_interior = 2 * i + 2;
                for j in 1..num_interior - 1 {
                    let t = j as f64 / (num_interior - 1) as f64;
                    let mut interior_position = Cartesian3::ZERO;
                    Cartesian3::lerp(&position, &reflected_position, t, &mut interior_position);
                    positions.push(interior_position.x);
                    positions.push(interior_position.y);
                    positions.push(interior_position.z);
                }

                positions.push(reflected_position.x);
                positions.push(reflected_position.y);
                positions.push(reflected_position.z);
            }

            if let Some(outer_positions) = &mut outer_positions {
                outer_right_index -= 1;
                outer_positions[outer_right_index] = position.z;
                outer_right_index -= 1;
                outer_positions[outer_right_index] = position.y;
                outer_right_index -= 1;
                outer_positions[outer_right_index] = position.x;
                outer_positions[outer_left_index] = reflected_position.x;
                outer_left_index += 1;
                outer_positions[outer_left_index] = reflected_position.y;
                outer_left_index += 1;
                outer_positions[outer_left_index] = reflected_position.z;
                outer_left_index += 1;
            }

            theta = CesiumMath::PI_OVER_TWO - (i + 1) as f64 * delta_theta;
        }

        // Compute points in the 'western' half of the ellipse
        let mut i = num_pts;
        while i > 1 {
            theta = CesiumMath::PI_OVER_TWO - (i - 1) as f64 * delta_theta;

            point_on_ellipsoid(
                -theta,
                rotation,
                &north_vec,
                &east_vec,
                a_sqr,
                ab,
                b_sqr,
                mag,
                &unit_pos,
                &mut position,
            );
            point_on_ellipsoid(
                theta + std::f64::consts::PI,
                rotation,
                &north_vec,
                &east_vec,
                a_sqr,
                ab,
                b_sqr,
                mag,
                &unit_pos,
                &mut reflected_position,
            );

            if let Some(positions) = &mut positions {
                positions.push(position.x);
                positions.push(position.y);
                positions.push(position.z);

                let num_interior = 2 * (i - 1) + 2;
                for j in 1..num_interior - 1 {
                    let t = j as f64 / (num_interior - 1) as f64;
                    let mut interior_position = Cartesian3::ZERO;
                    Cartesian3::lerp(&position, &reflected_position, t, &mut interior_position);
                    positions.push(interior_position.x);
                    positions.push(interior_position.y);
                    positions.push(interior_position.z);
                }

                positions.push(reflected_position.x);
                positions.push(reflected_position.y);
                positions.push(reflected_position.z);
            }

            if let Some(outer_positions) = &mut outer_positions {
                outer_right_index -= 1;
                outer_positions[outer_right_index] = position.z;
                outer_right_index -= 1;
                outer_positions[outer_right_index] = position.y;
                outer_right_index -= 1;
                outer_positions[outer_right_index] = position.x;
                outer_positions[outer_left_index] = reflected_position.x;
                outer_left_index += 1;
                outer_positions[outer_left_index] = reflected_position.y;
                outer_left_index += 1;
                outer_positions[outer_left_index] = reflected_position.z;
                outer_left_index += 1;
            }

            i -= 1;
        }

        theta = CesiumMath::PI_OVER_TWO;
        point_on_ellipsoid(
            -theta,
            rotation,
            &north_vec,
            &east_vec,
            a_sqr,
            ab,
            b_sqr,
            mag,
            &unit_pos,
            &mut position,
        );

        let mut result = ComputeEllipsePositionsResult {
            positions: None,
            num_pts,
            outer_positions: None,
        };
        if let Some(positions) = &mut positions {
            positions.push(position.x);
            positions.push(position.y);
            positions.push(position.z);
            result.positions = Some(std::mem::take(positions));
        }
        if let Some(outer_positions) = &mut outer_positions {
            outer_right_index -= 1;
            outer_positions[outer_right_index] = position.z;
            outer_right_index -= 1;
            outer_positions[outer_right_index] = position.y;
            outer_right_index -= 1;
            outer_positions[outer_right_index] = position.x;
            result.outer_positions = Some(std::mem::take(outer_positions));
        }

        result
    }
}
