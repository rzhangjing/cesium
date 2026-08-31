//! Ported from `packages/engine/Source/Core/EllipsoidalOccluder.js`.
//!
//! Determine whether or not other objects are visible or hidden behind the
//! visible horizon defined by an [`Ellipsoid`] and a camera position. The
//! ellipsoid is assumed to be located at the origin of the coordinate system.
//! This uses the algorithm described in the Horizon Culling blog post:
//! <https://cesium.com/blog/2013/04/25/Horizon-culling/>.

use crate::bounding_sphere::BoundingSphere;
use crate::cartesian3::Cartesian3;
use crate::ellipsoid::Ellipsoid;
use crate::rectangle::Rectangle;

/// Determines whether objects are visible behind the horizon defined by an
/// ellipsoid and a camera position.
pub struct EllipsoidalOccluder {
    ellipsoid: Ellipsoid,
    camera_position: Cartesian3,
    camera_position_in_scaled_space: Cartesian3,
    distance_to_limb_in_scaled_space_squared: f64,
}

impl EllipsoidalOccluder {
    /// Creates a new `EllipsoidalOccluder`.
    ///
    /// `ellipsoid` mirrors the required JS `ellipsoid` parameter (throws a
    /// `DeveloperError` in debug builds when missing); `camera_position` is
    /// optional and may be set later via [`set_camera_position`](Self::set_camera_position).
    pub fn new(ellipsoid: Option<Ellipsoid>, camera_position: Option<&Cartesian3>) -> Self {
        // Check.typeOf.object("ellipsoid", ellipsoid)
        #[cfg(debug_assertions)]
        if ellipsoid.is_none() {
            crate::developer_error::throw_developer_error("ellipsoid is required.");
        }

        let mut occluder = Self {
            ellipsoid: ellipsoid.unwrap_or(Ellipsoid::UNIT_SPHERE),
            camera_position: Cartesian3::default(),
            camera_position_in_scaled_space: Cartesian3::default(),
            distance_to_limb_in_scaled_space_squared: 0.0,
        };

        // cameraPosition fills in the above values
        if let Some(cp) = camera_position {
            occluder.set_camera_position(cp);
        }
        occluder
    }

    /// Gets the occluding ellipsoid.
    pub fn ellipsoid(&self) -> &Ellipsoid {
        &self.ellipsoid
    }

    /// Gets the position of the camera.
    pub fn camera_position(&self) -> &Cartesian3 {
        &self.camera_position
    }

    /// Sets the position of the camera, recomputing the horizon parameters.
    pub fn set_camera_position(&mut self, camera_position: &Cartesian3) {
        // See https://cesium.com/blog/2013/04/25/Horizon-culling/
        let mut cv = Cartesian3::default();
        self.ellipsoid
            .transform_position_to_scaled_space(camera_position, &mut cv);
        let vh_magnitude_squared = Cartesian3::magnitude_squared(&cv) - 1.0;

        self.camera_position = camera_position.clone();
        self.camera_position_in_scaled_space = cv;
        self.distance_to_limb_in_scaled_space_squared = vh_magnitude_squared;
    }

    /// Determines whether or not a point, the `occludee`, is hidden from view
    /// by the occluder. Returns `true` if the occludee is visible.
    pub fn is_point_visible(&self, occludee: &Cartesian3) -> bool {
        let mut occludee_scaled_space_position = Cartesian3::default();
        self.ellipsoid
            .transform_position_to_scaled_space(occludee, &mut occludee_scaled_space_position);
        is_scaled_space_point_visible(
            &occludee_scaled_space_position,
            &self.camera_position_in_scaled_space,
            self.distance_to_limb_in_scaled_space_squared,
        )
    }

    /// Determines whether or not a point expressed in the ellipsoid scaled
    /// space is hidden from view by the occluder. Returns `true` if visible.
    pub fn is_scaled_space_point_visible(
        &self,
        occludee_scaled_space_position: &Cartesian3,
    ) -> bool {
        is_scaled_space_point_visible(
            occludee_scaled_space_position,
            &self.camera_position_in_scaled_space,
            self.distance_to_limb_in_scaled_space_squared,
        )
    }

    /// Similar to [`is_scaled_space_point_visible`](Self::is_scaled_space_point_visible)
    /// except tests against an ellipsoid that has been shrunk by the minimum
    /// height when the minimum height is below the ellipsoid.
    pub fn is_scaled_space_point_visible_possibly_under_ellipsoid(
        &self,
        occludee_scaled_space_position: &Cartesian3,
        minimum_height: Option<f64>,
    ) -> bool {
        let ellipsoid = &self.ellipsoid;
        let vh_magnitude_squared;
        let cv;
        let mut cv_shrunk = Cartesian3::default();

        if let Some(minimum_height) = minimum_height {
            if minimum_height < 0.0 && ellipsoid.minimum_radius() > -minimum_height {
                // This code is similar to the cameraPosition setter, but unrolled
                // for performance because it will be called a lot.
                let radii = ellipsoid.radii();
                cv_shrunk.x = self.camera_position.x / (radii.x + minimum_height);
                cv_shrunk.y = self.camera_position.y / (radii.y + minimum_height);
                cv_shrunk.z = self.camera_position.z / (radii.z + minimum_height);
                vh_magnitude_squared = cv_shrunk.x * cv_shrunk.x
                    + cv_shrunk.y * cv_shrunk.y
                    + cv_shrunk.z * cv_shrunk.z
                    - 1.0;
                cv = &cv_shrunk;
            } else {
                cv = &self.camera_position_in_scaled_space;
                vh_magnitude_squared = self.distance_to_limb_in_scaled_space_squared;
            }
        } else {
            cv = &self.camera_position_in_scaled_space;
            vh_magnitude_squared = self.distance_to_limb_in_scaled_space_squared;
        }

        is_scaled_space_point_visible(occludee_scaled_space_position, cv, vh_magnitude_squared)
    }

    /// Computes a point that can be used for horizon culling from a list of
    /// positions. Returns `None` (mirrors JS `undefined`) when the point is
    /// undefined, e.g. when positions face opposite the direction.
    pub fn compute_horizon_culling_point<'a>(
        &self,
        direction_to_point: &Cartesian3,
        positions: &[Cartesian3],
        result: &'a mut Cartesian3,
    ) -> Option<&'a mut Cartesian3> {
        compute_horizon_culling_point_from_positions(
            &self.ellipsoid,
            direction_to_point,
            positions,
            result,
        )
    }

    /// Allocating variant of [`compute_horizon_culling_point`](Self::compute_horizon_culling_point).
    pub fn compute_horizon_culling_point_new(
        &self,
        direction_to_point: &Cartesian3,
        positions: &[Cartesian3],
    ) -> Option<Cartesian3> {
        let mut result = Cartesian3::default();
        self.compute_horizon_culling_point(direction_to_point, positions, &mut result)?;
        Some(result)
    }

    /// Similar to [`compute_horizon_culling_point`](Self::compute_horizon_culling_point)
    /// except computes the culling point relative to an ellipsoid that has been
    /// shrunk by the minimum height when the minimum height is below the ellipsoid.
    pub fn compute_horizon_culling_point_possibly_under_ellipsoid<'a>(
        &self,
        direction_to_point: &Cartesian3,
        positions: &[Cartesian3],
        minimum_height: Option<f64>,
        result: &'a mut Cartesian3,
    ) -> Option<&'a mut Cartesian3> {
        let possibly_shrunk_ellipsoid =
            get_possibly_shrunk_ellipsoid(&self.ellipsoid, minimum_height);
        compute_horizon_culling_point_from_positions(
            &possibly_shrunk_ellipsoid,
            direction_to_point,
            positions,
            result,
        )
    }

    /// Allocating variant of
    /// [`compute_horizon_culling_point_possibly_under_ellipsoid`](Self::compute_horizon_culling_point_possibly_under_ellipsoid).
    pub fn compute_horizon_culling_point_possibly_under_ellipsoid_new(
        &self,
        direction_to_point: &Cartesian3,
        positions: &[Cartesian3],
        minimum_height: Option<f64>,
    ) -> Option<Cartesian3> {
        let mut result = Cartesian3::default();
        self.compute_horizon_culling_point_possibly_under_ellipsoid(
            direction_to_point,
            positions,
            minimum_height,
            &mut result,
        )?;
        Some(result)
    }

    /// Computes a point that can be used for horizon culling from a list of
    /// vertices. `stride` mirrors the optional JS `stride` parameter (defaults
    /// to 3; required in debug builds), and `center` defaults to
    /// [`Cartesian3::ZERO`].
    pub fn compute_horizon_culling_point_from_vertices<'a>(
        &self,
        direction_to_point: &Cartesian3,
        vertices: &[f64],
        stride: Option<usize>,
        center: Option<&Cartesian3>,
        result: &'a mut Cartesian3,
    ) -> Option<&'a mut Cartesian3> {
        compute_horizon_culling_point_from_vertices(
            &self.ellipsoid,
            direction_to_point,
            vertices,
            stride,
            center,
            result,
        )
    }

    /// Allocating variant of
    /// [`compute_horizon_culling_point_from_vertices`](Self::compute_horizon_culling_point_from_vertices).
    pub fn compute_horizon_culling_point_from_vertices_new(
        &self,
        direction_to_point: &Cartesian3,
        vertices: &[f64],
        stride: Option<usize>,
        center: Option<&Cartesian3>,
    ) -> Option<Cartesian3> {
        let mut result = Cartesian3::default();
        self.compute_horizon_culling_point_from_vertices(
            direction_to_point,
            vertices,
            stride,
            center,
            &mut result,
        )?;
        Some(result)
    }

    /// Similar to
    /// [`compute_horizon_culling_point_from_vertices`](Self::compute_horizon_culling_point_from_vertices)
    /// except computes the culling point relative to a possibly-shrunk ellipsoid.
    pub fn compute_horizon_culling_point_from_vertices_possibly_under_ellipsoid<'a>(
        &self,
        direction_to_point: &Cartesian3,
        vertices: &[f64],
        stride: Option<usize>,
        center: Option<&Cartesian3>,
        minimum_height: Option<f64>,
        result: &'a mut Cartesian3,
    ) -> Option<&'a mut Cartesian3> {
        let possibly_shrunk_ellipsoid =
            get_possibly_shrunk_ellipsoid(&self.ellipsoid, minimum_height);
        compute_horizon_culling_point_from_vertices(
            &possibly_shrunk_ellipsoid,
            direction_to_point,
            vertices,
            stride,
            center,
            result,
        )
    }

    /// Allocating variant of
    /// [`compute_horizon_culling_point_from_vertices_possibly_under_ellipsoid`](Self::compute_horizon_culling_point_from_vertices_possibly_under_ellipsoid).
    pub fn compute_horizon_culling_point_from_vertices_possibly_under_ellipsoid_new(
        &self,
        direction_to_point: &Cartesian3,
        vertices: &[f64],
        stride: Option<usize>,
        center: Option<&Cartesian3>,
        minimum_height: Option<f64>,
    ) -> Option<Cartesian3> {
        let mut result = Cartesian3::default();
        self.compute_horizon_culling_point_from_vertices_possibly_under_ellipsoid(
            direction_to_point,
            vertices,
            stride,
            center,
            minimum_height,
            &mut result,
        )?;
        Some(result)
    }

    /// Computes a point that can be used for horizon culling of a rectangle.
    /// Returns `None` (mirrors JS `undefined`) when the bounding sphere center
    /// is too close to the center of the occluder.
    pub fn compute_horizon_culling_point_from_rectangle<'a>(
        &self,
        rectangle: &Rectangle,
        ellipsoid: &Ellipsoid,
        result: &'a mut Cartesian3,
    ) -> Option<&'a mut Cartesian3> {
        // Check.typeOf.object("rectangle", rectangle) — guaranteed by the type system.

        let positions = Rectangle::subsample(rectangle, Some(ellipsoid), Some(0.0));
        let bs = BoundingSphere::from_points(&positions, None);

        // If the bounding sphere center is too close to the center of the occluder,
        // it doesn't make sense to try to horizon cull it.
        if Cartesian3::magnitude(&bs.center) < 0.1 * ellipsoid.minimum_radius() {
            return None;
        }

        self.compute_horizon_culling_point(&bs.center, &positions, result)
    }

    /// Allocating variant of
    /// [`compute_horizon_culling_point_from_rectangle`](Self::compute_horizon_culling_point_from_rectangle).
    pub fn compute_horizon_culling_point_from_rectangle_new(
        &self,
        rectangle: &Rectangle,
        ellipsoid: &Ellipsoid,
    ) -> Option<Cartesian3> {
        let mut result = Cartesian3::default();
        self.compute_horizon_culling_point_from_rectangle(rectangle, ellipsoid, &mut result)?;
        Some(result)
    }
}

fn get_possibly_shrunk_ellipsoid(
    ellipsoid: &Ellipsoid,
    minimum_height: Option<f64>,
) -> Ellipsoid {
    if let Some(minimum_height) = minimum_height {
        if minimum_height < 0.0 && ellipsoid.minimum_radius() > -minimum_height {
            let radii = ellipsoid.radii();
            let ellipsoid_shrunk_radii = Cartesian3::new(
                radii.x + minimum_height,
                radii.y + minimum_height,
                radii.z + minimum_height,
            );
            return Ellipsoid::from_cartesian3(Some(&ellipsoid_shrunk_radii));
        }
    }
    ellipsoid.clone()
}

fn compute_horizon_culling_point_from_positions<'a>(
    ellipsoid: &Ellipsoid,
    direction_to_point: &Cartesian3,
    positions: &[Cartesian3],
    result: &'a mut Cartesian3,
) -> Option<&'a mut Cartesian3> {
    // Check.typeOf.object("directionToPoint", directionToPoint) and
    // Check.defined("positions", positions) — guaranteed by the type system.

    let scaled_space_direction_to_point =
        compute_scaled_space_direction_to_point(ellipsoid, direction_to_point);
    let mut result_magnitude: f64 = 0.0;

    for position in positions {
        let candidate_magnitude = compute_magnitude(
            ellipsoid,
            position,
            &scaled_space_direction_to_point,
        );
        if candidate_magnitude < 0.0 {
            // all points should face the same direction, but this one doesn't,
            // so return undefined
            return None;
        }
        result_magnitude = result_magnitude.max(candidate_magnitude);
    }

    magnitude_to_point(&scaled_space_direction_to_point, result_magnitude, result)
}

fn compute_horizon_culling_point_from_vertices<'a>(
    ellipsoid: &Ellipsoid,
    direction_to_point: &Cartesian3,
    vertices: &[f64],
    stride: Option<usize>,
    center: Option<&Cartesian3>,
    result: &'a mut Cartesian3,
) -> Option<&'a mut Cartesian3> {
    // Check.typeOf.object("directionToPoint", directionToPoint),
    // Check.defined("vertices", vertices) — guaranteed by the type system.
    // Check.typeOf.number("stride", stride)
    #[cfg(debug_assertions)]
    if stride.is_none() {
        crate::developer_error::throw_developer_error("stride is required.");
    }

    let stride = stride.unwrap_or(3);
    let center = center.unwrap_or(&Cartesian3::ZERO);
    let scaled_space_direction_to_point =
        compute_scaled_space_direction_to_point(ellipsoid, direction_to_point);
    let mut result_magnitude: f64 = 0.0;

    let mut i = 0;
    while i < vertices.len() {
        let mut position_scratch = Cartesian3::default();
        position_scratch.x = vertices[i] + center.x;
        position_scratch.y = vertices[i + 1] + center.y;
        position_scratch.z = vertices[i + 2] + center.z;

        let candidate_magnitude = compute_magnitude(
            ellipsoid,
            &position_scratch,
            &scaled_space_direction_to_point,
        );
        if candidate_magnitude < 0.0 {
            // all points should face the same direction, but this one doesn't,
            // so return undefined
            return None;
        }
        result_magnitude = result_magnitude.max(candidate_magnitude);
        i += stride;
    }

    magnitude_to_point(&scaled_space_direction_to_point, result_magnitude, result)
}

fn is_scaled_space_point_visible(
    occludee_scaled_space_position: &Cartesian3,
    camera_position_in_scaled_space: &Cartesian3,
    distance_to_limb_in_scaled_space_squared: f64,
) -> bool {
    // See https://cesium.com/blog/2013/04/25/Horizon-culling/
    let cv = camera_position_in_scaled_space;
    let vh_magnitude_squared = distance_to_limb_in_scaled_space_squared;
    let mut vt = Cartesian3::default();
    Cartesian3::subtract(occludee_scaled_space_position, cv, &mut vt);
    let vt_dot_vc = -Cartesian3::dot(&vt, cv);
    // If vhMagnitudeSquared < 0 then we are below the surface of the ellipsoid
    // and in this case, set the culling plane to be on V.
    let is_occluded = if vh_magnitude_squared < 0.0 {
        vt_dot_vc > 0.0
    } else {
        vt_dot_vc > vh_magnitude_squared
            && (vt_dot_vc * vt_dot_vc) / Cartesian3::magnitude_squared(&vt)
                > vh_magnitude_squared
    };
    !is_occluded
}

fn compute_magnitude(
    ellipsoid: &Ellipsoid,
    position: &Cartesian3,
    scaled_space_direction_to_point: &Cartesian3,
) -> f64 {
    let mut scaled_space_position = Cartesian3::default();
    ellipsoid.transform_position_to_scaled_space(position, &mut scaled_space_position);
    let mut magnitude_squared = Cartesian3::magnitude_squared(&scaled_space_position);
    let mut magnitude = magnitude_squared.sqrt();
    let mut direction = Cartesian3::default();
    Cartesian3::divide_by_scalar(&scaled_space_position, magnitude, &mut direction);

    // For the purpose of this computation, points below the ellipsoid are
    // considered to be on it instead.
    magnitude_squared = magnitude_squared.max(1.0);
    magnitude = magnitude.max(1.0);

    let cos_alpha = Cartesian3::dot(&direction, scaled_space_direction_to_point);
    let sin_alpha = Cartesian3::magnitude(&Cartesian3::cross_new(
        &direction,
        scaled_space_direction_to_point,
    ));
    let cos_beta = 1.0 / magnitude;
    let sin_beta = (magnitude_squared - 1.0).sqrt() * cos_beta;

    1.0 / (cos_alpha * cos_beta - sin_alpha * sin_beta)
}

fn magnitude_to_point<'a>(
    scaled_space_direction_to_point: &Cartesian3,
    result_magnitude: f64,
    result: &'a mut Cartesian3,
) -> Option<&'a mut Cartesian3> {
    // The horizon culling point is undefined if there were no positions from
    // which to compute it, the directionToPoint is pointing opposite all of the
    // positions, or if we computed NaN or infinity.
    if result_magnitude <= 0.0
        || result_magnitude == f64::INFINITY
        || result_magnitude.is_nan()
    {
        return None;
    }

    Cartesian3::multiply_by_scalar(
        scaled_space_direction_to_point,
        result_magnitude,
        result,
    );
    Some(result)
}

fn compute_scaled_space_direction_to_point(
    ellipsoid: &Ellipsoid,
    direction_to_point: &Cartesian3,
) -> Cartesian3 {
    if Cartesian3::equals(Some(direction_to_point), Some(&Cartesian3::ZERO)) {
        return direction_to_point.clone();
    }

    let mut direction_to_point_scratch = Cartesian3::default();
    ellipsoid.transform_position_to_scaled_space(
        direction_to_point,
        &mut direction_to_point_scratch,
    );
    Cartesian3::normalize_new(&direction_to_point_scratch)
}
