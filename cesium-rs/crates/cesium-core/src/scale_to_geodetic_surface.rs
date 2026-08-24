//! Ported from packages/engine/Source/Core/scaleToGeodeticSurface.js

use crate::cartesian3::Cartesian3;
use crate::math::CesiumMath;

/// Scales the provided Cartesian position along the geodetic surface
/// normal so that it is on the surface of this ellipsoid. If the
/// position is at the center of the ellipsoid, the JS function returns
/// `undefined`, mirrored here by `false` / `None`.
///
/// Port of `scaleToGeodeticSurface` (`@private` in JS). The JS
/// `defined`-checks on the four required parameters are statically
/// impossible in Rust.
pub fn scale_to_geodetic_surface(
    cartesian: &Cartesian3,
    one_over_radii: &Cartesian3,
    one_over_radii_squared: &Cartesian3,
    center_tolerance_squared: f64,
    result: &mut Cartesian3,
) -> bool {
    let position_x = cartesian.x;
    let position_y = cartesian.y;
    let position_z = cartesian.z;

    let one_over_radii_x = one_over_radii.x;
    let one_over_radii_y = one_over_radii.y;
    let one_over_radii_z = one_over_radii.z;

    let x2 = position_x * position_x * one_over_radii_x * one_over_radii_x;
    let y2 = position_y * position_y * one_over_radii_y * one_over_radii_y;
    let z2 = position_z * position_z * one_over_radii_z * one_over_radii_z;

    // Compute the squared ellipsoid norm.
    let squared_norm = x2 + y2 + z2;
    let ratio = (1.0 / squared_norm).sqrt();

    // As an initial approximation, assume that the radial intersection
    // is the projection point.
    let mut intersection = Cartesian3::default();
    Cartesian3::multiply_by_scalar(cartesian, ratio, &mut intersection);

    // If the position is near the center, the iteration will not
    // converge.
    if squared_norm < center_tolerance_squared {
        if !ratio.is_finite() {
            return false;
        }
        Cartesian3::clone_into(&intersection, result);
        return true;
    }

    let one_over_radii_squared_x = one_over_radii_squared.x;
    let one_over_radii_squared_y = one_over_radii_squared.y;
    let one_over_radii_squared_z = one_over_radii_squared.z;

    // Use the gradient at the intersection point in place of the true
    // unit normal. The difference in magnitude will be absorbed in the
    // multiplier.
    let gradient = Cartesian3::new(
        intersection.x * one_over_radii_squared_x * 2.0,
        intersection.y * one_over_radii_squared_y * 2.0,
        intersection.z * one_over_radii_squared_z * 2.0,
    );

    // Compute the initial guess at the normal vector multiplier, lambda.
    let mut lambda = ((1.0 - ratio) * Cartesian3::magnitude(cartesian))
        / (0.5 * Cartesian3::magnitude(&gradient));
    let mut correction = 0.0;

    let mut func;
    let mut x_multiplier;
    let mut y_multiplier;
    let mut z_multiplier;

    loop {
        lambda -= correction;

        x_multiplier = 1.0 / (1.0 + lambda * one_over_radii_squared_x);
        y_multiplier = 1.0 / (1.0 + lambda * one_over_radii_squared_y);
        z_multiplier = 1.0 / (1.0 + lambda * one_over_radii_squared_z);

        let x_multiplier2 = x_multiplier * x_multiplier;
        let y_multiplier2 = y_multiplier * y_multiplier;
        let z_multiplier2 = z_multiplier * z_multiplier;

        let x_multiplier3 = x_multiplier2 * x_multiplier;
        let y_multiplier3 = y_multiplier2 * y_multiplier;
        let z_multiplier3 = z_multiplier2 * z_multiplier;

        func = x2 * x_multiplier2 + y2 * y_multiplier2 + z2 * z_multiplier2 - 1.0;

        // "denominator" here refers to the use of this expression in the
        // velocity and acceleration computations in the sections to
        // follow.
        let denominator = x2 * x_multiplier3 * one_over_radii_squared_x
            + y2 * y_multiplier3 * one_over_radii_squared_y
            + z2 * z_multiplier3 * one_over_radii_squared_z;

        let derivative = -2.0 * denominator;

        correction = func / derivative;

        // Faithful mirror of the JS `do/while` condition
        // `Math.abs(func) > CesiumMath.EPSILON12`: for non-converging /
        // non-finite inputs `func` becomes `NaN`, and `NaN > EPSILON12`
        // is false, so CesiumJS exits the loop here and proceeds with the
        // (NaN) multipliers. The previous `<=` formulation never held for
        // NaN and looped forever (Phase 2 finding D1).
        if !(func.abs() > CesiumMath::EPSILON12) {
            break;
        }
    }

    result.x = position_x * x_multiplier;
    result.y = position_y * y_multiplier;
    result.z = position_z * z_multiplier;
    true
}

/// Allocating variant of [`scale_to_geodetic_surface`]; `None` mirrors
/// the JS `undefined` return (position at the ellipsoid center).
pub fn scale_to_geodetic_surface_new(
    cartesian: &Cartesian3,
    one_over_radii: &Cartesian3,
    one_over_radii_squared: &Cartesian3,
    center_tolerance_squared: f64,
) -> Option<Cartesian3> {
    let mut result = Cartesian3::default();
    if scale_to_geodetic_surface(
        cartesian,
        one_over_radii,
        one_over_radii_squared,
        center_tolerance_squared,
        &mut result,
    ) {
        Some(result)
    } else {
        None
    }
}
