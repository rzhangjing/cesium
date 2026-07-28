//! EllipsoidalOccluder - horizon culling against an ellipsoid.
//! Maps to CesiumJS `Core/EllipsoidalOccluder.js`

use crate::bounding::BoundingSphere;
use crate::ellipsoid::{normalize_cartesian3, Ellipsoid};
use crate::rectangle::Rectangle;
use glam::DVec3;

/// Determines whether or not other objects are visible or hidden behind the
/// visible horizon defined by an ellipsoid and a camera position.
/// Uses the algorithm described in the Horizon Culling blog post.
///
/// Maps to CesiumJS `EllipsoidalOccluder`
#[derive(Debug, Clone)]
pub struct EllipsoidalOccluder {
    ellipsoid: Ellipsoid,
    camera_position: DVec3,
    camera_position_in_scaled_space: DVec3,
    distance_to_limb_in_scaled_space_squared: f64,
}

impl EllipsoidalOccluder {
    /// Creates a new ellipsoidal occluder.
    /// If `camera_position` is provided, internal scaled-space values are computed immediately.
    ///
    /// Maps to `new EllipsoidalOccluder(ellipsoid, cameraPosition)`
    pub fn new(ellipsoid: Ellipsoid, camera_position: Option<DVec3>) -> Self {
        let mut occluder = Self {
            ellipsoid,
            camera_position: DVec3::ZERO,
            camera_position_in_scaled_space: DVec3::ZERO,
            distance_to_limb_in_scaled_space_squared: 0.0,
        };
        if let Some(cp) = camera_position {
            occluder.set_camera_position(cp);
        }
        occluder
    }

    /// Gets the occluding ellipsoid.
    #[inline]
    pub fn ellipsoid(&self) -> &Ellipsoid {
        &self.ellipsoid
    }

    /// Gets the camera position.
    #[inline]
    pub fn camera_position(&self) -> DVec3 {
        self.camera_position
    }

    /// Sets the camera position and recomputes internal scaled-space values.
    ///
    /// Maps to `EllipsoidalOccluder.prototype.cameraPosition` setter
    pub fn set_camera_position(&mut self, camera_position: DVec3) {
        let cv = self.ellipsoid.transform_position_to_scaled_space(camera_position);
        let vh_magnitude_squared = cv.length_squared() - 1.0;

        self.camera_position = camera_position;
        self.camera_position_in_scaled_space = cv;
        self.distance_to_limb_in_scaled_space_squared = vh_magnitude_squared;
    }

    /// Determines whether or not a point (the occludee) is hidden from view by the occluder.
    ///
    /// Maps to `EllipsoidalOccluder.prototype.isPointVisible`
    pub fn is_point_visible(&self, occludee: DVec3) -> bool {
        let occludee_scaled_space_position =
            self.ellipsoid.transform_position_to_scaled_space(occludee);
        is_scaled_space_point_visible(
            occludee_scaled_space_position,
            self.camera_position_in_scaled_space,
            self.distance_to_limb_in_scaled_space_squared,
        )
    }

    /// Determines whether or not a point expressed in the ellipsoid scaled space
    /// is hidden from view by the occluder.
    ///
    /// Maps to `EllipsoidalOccluder.prototype.isScaledSpacePointVisible`
    pub fn is_scaled_space_point_visible(&self, occludee_scaled_space_position: DVec3) -> bool {
        is_scaled_space_point_visible(
            occludee_scaled_space_position,
            self.camera_position_in_scaled_space,
            self.distance_to_limb_in_scaled_space_squared,
        )
    }

    /// Similar to `is_scaled_space_point_visible` except tests against an
    /// ellipsoid that has been shrunk by the minimum height when the minimum
    /// height is below the ellipsoid.
    ///
    /// Maps to `EllipsoidalOccluder.prototype.isScaledSpacePointVisiblePossiblyUnderEllipsoid`
    pub fn is_scaled_space_point_visible_possibly_under_ellipsoid(
        &self,
        occludee_scaled_space_position: DVec3,
        minimum_height: Option<f64>,
    ) -> bool {
        let ellipsoid = &self.ellipsoid;
        let (cv, vh_magnitude_squared);

        if let Some(mh) = minimum_height {
            if mh < 0.0 && ellipsoid.minimum_radius() > -mh {
                let radii = ellipsoid.radii();
                let cp = self.camera_position;
                cv = DVec3::new(
                    cp.x / (radii.x + mh),
                    cp.y / (radii.y + mh),
                    cp.z / (radii.z + mh),
                );
                vh_magnitude_squared = cv.length_squared() - 1.0;
            } else {
                cv = self.camera_position_in_scaled_space;
                vh_magnitude_squared = self.distance_to_limb_in_scaled_space_squared;
            }
        } else {
            cv = self.camera_position_in_scaled_space;
            vh_magnitude_squared = self.distance_to_limb_in_scaled_space_squared;
        }

        is_scaled_space_point_visible(occludee_scaled_space_position, cv, vh_magnitude_squared)
    }

    /// Computes a point that can be used for horizon culling from a list of positions.
    /// Returns None if the point cannot be computed (e.g., positions face opposite direction).
    ///
    /// Maps to `EllipsoidalOccluder.prototype.computeHorizonCullingPoint`
    pub fn compute_horizon_culling_point(
        &self,
        direction_to_point: DVec3,
        positions: &[DVec3],
    ) -> Option<DVec3> {
        compute_horizon_culling_point_from_positions(
            &self.ellipsoid,
            direction_to_point,
            positions,
        )
    }

    /// Similar to `compute_horizon_culling_point` except computes relative to an
    /// ellipsoid shrunk by the minimum height when below the ellipsoid.
    ///
    /// Maps to `EllipsoidalOccluder.prototype.computeHorizonCullingPointPossiblyUnderEllipsoid`
    pub fn compute_horizon_culling_point_possibly_under_ellipsoid(
        &self,
        direction_to_point: DVec3,
        positions: &[DVec3],
        minimum_height: Option<f64>,
    ) -> Option<DVec3> {
        let possibly_shrunk = get_possibly_shrunk_ellipsoid(&self.ellipsoid, minimum_height);
        compute_horizon_culling_point_from_positions(
            &possibly_shrunk,
            direction_to_point,
            positions,
        )
    }

    /// Computes a horizon culling point from vertex data with a stride.
    ///
    /// Maps to `EllipsoidalOccluder.prototype.computeHorizonCullingPointFromVertices`
    pub fn compute_horizon_culling_point_from_vertices(
        &self,
        direction_to_point: DVec3,
        vertices: &[f64],
        stride: usize,
        center: DVec3,
    ) -> Option<DVec3> {
        compute_horizon_culling_point_from_vertices(
            &self.ellipsoid,
            direction_to_point,
            vertices,
            stride,
            center,
        )
    }

    /// Similar to `compute_horizon_culling_point_from_vertices` except computes
    /// relative to a possibly-shrunk ellipsoid.
    ///
    /// Maps to `EllipsoidalOccluder.prototype.computeHorizonCullingPointFromVerticesPossiblyUnderEllipsoid`
    pub fn compute_horizon_culling_point_from_vertices_possibly_under_ellipsoid(
        &self,
        direction_to_point: DVec3,
        vertices: &[f64],
        stride: usize,
        center: DVec3,
        minimum_height: Option<f64>,
    ) -> Option<DVec3> {
        let possibly_shrunk = get_possibly_shrunk_ellipsoid(&self.ellipsoid, minimum_height);
        compute_horizon_culling_point_from_vertices(
            &possibly_shrunk,
            direction_to_point,
            vertices,
            stride,
            center,
        )
    }

    /// Computes a horizon culling point from a rectangle.
    /// Returns None if the bounding sphere center is too close to the ellipsoid center.
    ///
    /// Maps to `EllipsoidalOccluder.prototype.computeHorizonCullingPointFromRectangle`
    pub fn compute_horizon_culling_point_from_rectangle(
        &self,
        rectangle: &Rectangle,
        ellipsoid: &Ellipsoid,
    ) -> Option<DVec3> {
        let positions = rectangle.subsample(ellipsoid, 0.0);
        let bs = BoundingSphere::from_points(&positions);

        // If the bounding sphere center is too close to the center of the occluder,
        // it doesn't make sense to try to horizon cull it.
        if bs.center.length() < 0.1 * ellipsoid.minimum_radius() {
            return None;
        }

        self.compute_horizon_culling_point(bs.center, &positions)
    }
}

// --- Private helper functions ---

/// Core visibility test in scaled space.
/// Maps to the module-level `isScaledSpacePointVisible` function.
fn is_scaled_space_point_visible(
    occludee_scaled_space_position: DVec3,
    camera_position_in_scaled_space: DVec3,
    distance_to_limb_in_scaled_space_squared: f64,
) -> bool {
    let cv = camera_position_in_scaled_space;
    let vh_magnitude_squared = distance_to_limb_in_scaled_space_squared;
    let vt = occludee_scaled_space_position - cv;
    let vt_dot_vc = -vt.dot(cv);

    // If vhMagnitudeSquared < 0 then we are below the surface of the ellipsoid and
    // in this case, set the culling plane to be on V.
    let is_occluded = if vh_magnitude_squared < 0.0 {
        vt_dot_vc > 0.0
    } else {
        vt_dot_vc > vh_magnitude_squared
            && (vt_dot_vc * vt_dot_vc) / vt.length_squared() > vh_magnitude_squared
    };
    !is_occluded
}

/// Computes the horizon culling point from an array of positions.
fn compute_horizon_culling_point_from_positions(
    ellipsoid: &Ellipsoid,
    direction_to_point: DVec3,
    positions: &[DVec3],
) -> Option<DVec3> {
    let scaled_space_direction_to_point =
        compute_scaled_space_direction_to_point(ellipsoid, direction_to_point)?;

    let mut result_magnitude = 0.0_f64;

    for &position in positions {
        let candidate_magnitude =
            compute_magnitude(ellipsoid, position, scaled_space_direction_to_point);
        if candidate_magnitude < 0.0 {
            return None;
        }
        result_magnitude = result_magnitude.max(candidate_magnitude);
    }

    magnitude_to_point(scaled_space_direction_to_point, result_magnitude)
}

/// Computes the horizon culling point from vertex data with stride.
fn compute_horizon_culling_point_from_vertices(
    ellipsoid: &Ellipsoid,
    direction_to_point: DVec3,
    vertices: &[f64],
    stride: usize,
    center: DVec3,
) -> Option<DVec3> {
    let scaled_space_direction_to_point =
        compute_scaled_space_direction_to_point(ellipsoid, direction_to_point)?;

    let mut result_magnitude = 0.0_f64;

    let mut i = 0;
    while i + 2 < vertices.len() {
        let position = DVec3::new(
            vertices[i] + center.x,
            vertices[i + 1] + center.y,
            vertices[i + 2] + center.z,
        );

        let candidate_magnitude =
            compute_magnitude(ellipsoid, position, scaled_space_direction_to_point);
        if candidate_magnitude < 0.0 {
            return None;
        }
        result_magnitude = result_magnitude.max(candidate_magnitude);

        i += stride;
    }

    magnitude_to_point(scaled_space_direction_to_point, result_magnitude)
}

/// Computes the magnitude for a position relative to the scaled-space direction.
fn compute_magnitude(
    ellipsoid: &Ellipsoid,
    position: DVec3,
    scaled_space_direction_to_point: DVec3,
) -> f64 {
    let scaled_space_position = ellipsoid.transform_position_to_scaled_space(position);
    let mut magnitude_squared = scaled_space_position.length_squared();
    let mut magnitude = magnitude_squared.sqrt();
    let direction = scaled_space_position / magnitude;

    // For the purpose of this computation, points below the ellipsoid are considered to be on it instead.
    magnitude_squared = magnitude_squared.max(1.0);
    magnitude = magnitude.max(1.0);

    let cos_alpha = direction.dot(scaled_space_direction_to_point);
    let sin_alpha = direction.cross(scaled_space_direction_to_point).length();
    let cos_beta = 1.0 / magnitude;
    let sin_beta = (magnitude_squared - 1.0).sqrt() * cos_beta;

    1.0 / (cos_alpha * cos_beta - sin_alpha * sin_beta)
}

/// Converts a magnitude along the scaled-space direction to a point.
/// Returns None if the magnitude is invalid.
fn magnitude_to_point(scaled_space_direction_to_point: DVec3, result_magnitude: f64) -> Option<DVec3> {
    // The horizon culling point is undefined if there were no positions from which to compute it,
    // the directionToPoint is pointing opposite all of the positions, or if we computed NaN or infinity.
    if result_magnitude <= 0.0 || result_magnitude == f64::INFINITY || result_magnitude.is_nan() {
        return None;
    }

    Some(scaled_space_direction_to_point * result_magnitude)
}

/// Transforms a direction to scaled space and normalizes it.
/// Returns None if the direction is zero.
fn compute_scaled_space_direction_to_point(
    ellipsoid: &Ellipsoid,
    direction_to_point: DVec3,
) -> Option<DVec3> {
    if direction_to_point == DVec3::ZERO {
        return None;
    }

    let scaled = ellipsoid.transform_position_to_scaled_space(direction_to_point);
    Some(normalize_cartesian3(scaled))
}

/// Returns a possibly-shrunk ellipsoid based on minimum height.
fn get_possibly_shrunk_ellipsoid(ellipsoid: &Ellipsoid, minimum_height: Option<f64>) -> Ellipsoid {
    if let Some(mh) = minimum_height {
        if mh < 0.0 && ellipsoid.minimum_radius() > -mh {
            let radii = ellipsoid.radii();
            return Ellipsoid::new(radii.x + mh, radii.y + mh, radii.z + mh);
        }
    }
    *ellipsoid
}
