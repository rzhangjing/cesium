//! Ported from `packages/engine/Source/Core/EllipsoidalOccluder.js`.
//!
//! Determines visibility based on an ellipsoid and camera position.

use crate::bounding_sphere::BoundingSphere;
use crate::cartesian3::Cartesian3;
use crate::ellipsoid::Ellipsoid;

/// Determines whether objects are visible behind the horizon defined by an ellipsoid.
pub struct EllipsoidalOccluder {
    ellipsoid: Ellipsoid,
    camera_position: Cartesian3,
    camera_position_in_scaled_space: Cartesian3,
    distance_to_limb_in_scaled_space_squared: f64,
}

impl EllipsoidalOccluder {
    /// Creates a new EllipsoidalOccluder.
    pub fn new(ellipsoid: Ellipsoid, camera_position: Option<&Cartesian3>) -> Self {
        let mut occluder = Self {
            ellipsoid,
            camera_position: Cartesian3::default(),
            camera_position_in_scaled_space: Cartesian3::default(),
            distance_to_limb_in_scaled_space_squared: 0.0,
        };
        if let Some(cp) = camera_position {
            occluder.set_camera_position(cp);
        }
        occluder
    }

    /// Gets the ellipsoid.
    pub fn ellipsoid(&self) -> &Ellipsoid {
        &self.ellipsoid
    }

    /// Gets the camera position.
    pub fn camera_position(&self) -> &Cartesian3 {
        &self.camera_position
    }

    /// Sets the camera position, recomputing horizon parameters.
    pub fn set_camera_position(&mut self, camera_position: &Cartesian3) {
        self.camera_position = camera_position.clone();
        let radii = self.ellipsoid.radii();
        let one_over_radii = Cartesian3::new(
            1.0 / radii.x,
            1.0 / radii.y,
            1.0 / radii.z,
        );
        let scaled_pos =
            Cartesian3::multiply_components_new(camera_position, &one_over_radii);
        let mag_sq = Cartesian3::magnitude_squared(&scaled_pos);
        self.distance_to_limb_in_scaled_space_squared = mag_sq - 1.0;
        self.camera_position_in_scaled_space = scaled_pos;
    }

    /// Determines if a point is visible.
    pub fn is_point_visible(&self, point: &Cartesian3) -> bool {
        let radii = self.ellipsoid.radii();
        let one_over_radii = Cartesian3::new(
            1.0 / radii.x,
            1.0 / radii.y,
            1.0 / radii.z,
        );
        let scaled_point =
            Cartesian3::multiply_components_new(point, &one_over_radii);
        let scaled_mag = Cartesian3::magnitude(&scaled_point);
        let cam_mag = Cartesian3::magnitude(&self.camera_position_in_scaled_space);
        if scaled_mag < 1e-10 || cam_mag < 1e-10 {
            return false;
        }
        let cos_alpha = Cartesian3::dot(
            &scaled_point,
            &self.camera_position_in_scaled_space,
        ) / (scaled_mag * cam_mag);
        let limb_angle = self.distance_to_limb_in_scaled_space_squared.sqrt().atan();
        cos_alpha > limb_angle.cos()
    }

    /// Determines if a bounding sphere is visible.
    pub fn is_bounding_sphere_visible(&self, bs: &BoundingSphere) -> bool {
        self.is_point_visible(&bs.center)
    }
}
