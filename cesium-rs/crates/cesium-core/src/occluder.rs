//! Ported from `packages/engine/Source/Core/Occluder.js`.
//!
//! Determines whether objects are visible or hidden behind a visible horizon.

use crate::bounding_sphere::BoundingSphere;
use crate::cartesian3::Cartesian3;
use crate::visibility::Visibility;

/// An occluder derived from an object's position and radius, plus the camera position.
pub struct Occluder {
    occluder_position: Cartesian3,
    occluder_radius: f64,
    horizon_distance: f64,
    horizon_plane_normal: Option<Cartesian3>,
    horizon_plane_position: Option<Cartesian3>,
    camera_position: Option<Cartesian3>,
}

impl Occluder {
    /// Creates a new Occluder from a bounding sphere and camera position.
    pub fn new(occluder_bounding_sphere: &BoundingSphere, camera_position: &Cartesian3) -> Self {
        let mut occluder = Self {
            occluder_position: occluder_bounding_sphere.center.clone(),
            occluder_radius: occluder_bounding_sphere.radius,
            horizon_distance: 0.0,
            horizon_plane_normal: None,
            horizon_plane_position: None,
            camera_position: None,
        };
        occluder.set_camera_position(camera_position);
        occluder
    }

    /// Gets the occluder position.
    pub fn position(&self) -> &Cartesian3 {
        &self.occluder_position
    }

    /// Gets the occluder radius.
    pub fn radius(&self) -> f64 {
        self.occluder_radius
    }

    /// Sets the camera position, recomputing horizon parameters.
    pub fn set_camera_position(&mut self, camera_position: &Cartesian3) {
        let camera_to_occluder =
            Cartesian3::subtract_new(&self.occluder_position, camera_position);
        let cam_to_occ_dist_sq = Cartesian3::magnitude_squared(&camera_to_occluder);
        let occluder_radius_sq = self.occluder_radius * self.occluder_radius;

        if cam_to_occ_dist_sq > occluder_radius_sq {
            let horizon_distance = (cam_to_occ_dist_sq - occluder_radius_sq).sqrt();
            let inv_dist = 1.0 / cam_to_occ_dist_sq.sqrt();
            let horizon_plane_normal =
                Cartesian3::multiply_by_scalar_new(&camera_to_occluder, inv_dist);
            let near_plane_distance =
                horizon_distance * horizon_distance * inv_dist;
            let horizon_plane_position = Cartesian3::add_new(
                camera_position,
                &Cartesian3::multiply_by_scalar_new(&horizon_plane_normal, near_plane_distance),
            );

            self.horizon_distance = horizon_distance;
            self.horizon_plane_normal = Some(horizon_plane_normal);
            self.horizon_plane_position = Some(horizon_plane_position);
        } else {
            self.horizon_distance = f64::MAX;
            self.horizon_plane_normal = None;
            self.horizon_plane_position = None;
        }
        self.camera_position = Some(camera_position.clone());
    }

    /// Determines whether a point is visible from the camera.
    pub fn is_point_visible(&self, occludee: &Cartesian3) -> bool {
        if self.horizon_distance == f64::MAX {
            return false;
        }
        let temp_vec = Cartesian3::subtract_new(occludee, &self.occluder_position);
        let temp = Cartesian3::magnitude_squared(&temp_vec)
            - self.occluder_radius * self.occluder_radius;
        if temp > 0.0 {
            let temp = temp.sqrt() + self.horizon_distance;
            let temp_vec = Cartesian3::subtract_new(
                occludee,
                self.camera_position.as_ref().unwrap(),
            );
            return temp * temp > Cartesian3::magnitude_squared(&temp_vec);
        }
        false
    }

    /// Determines whether a bounding sphere is visible from the camera.
    pub fn is_bounding_sphere_visible(&self, occludee: &BoundingSphere) -> bool {
        let occludee_position = &occludee.center;
        let occludee_radius = occludee.radius;

        if self.horizon_distance == f64::MAX {
            return false;
        }

        let temp_vec =
            Cartesian3::subtract_new(occludee_position, &self.occluder_position);
        let temp = self.occluder_radius - occludee_radius;
        let temp = Cartesian3::magnitude_squared(&temp_vec) - temp * temp;

        if occludee_radius < self.occluder_radius {
            if temp > 0.0 {
                let temp = temp.sqrt() + self.horizon_distance;
                let temp_vec = Cartesian3::subtract_new(
                    occludee_position,
                    self.camera_position.as_ref().unwrap(),
                );
                return temp * temp + occludee_radius * occludee_radius
                    > Cartesian3::magnitude_squared(&temp_vec);
            }
            return false;
        }

        if temp > 0.0 {
            let temp_vec = Cartesian3::subtract_new(
                occludee_position,
                self.camera_position.as_ref().unwrap(),
            );
            let mag_sq = Cartesian3::magnitude_squared(&temp_vec);
            let occ_rad_sq = self.occluder_radius * self.occluder_radius;
            let ocl_rad_sq = occludee_radius * occludee_radius;
            if (self.horizon_distance * self.horizon_distance + occ_rad_sq) * ocl_rad_sq
                > mag_sq * occ_rad_sq
            {
                return true;
            }
            let temp = temp.sqrt() + self.horizon_distance;
            return temp * temp + ocl_rad_sq > mag_sq;
        }
        true
    }

    /// Computes the visibility of a bounding sphere.
    pub fn compute_visibility(&self, occludee_bs: &BoundingSphere) -> Visibility {
        let occludee_radius = occludee_bs.radius;
        if occludee_radius > self.occluder_radius {
            return Visibility::Full;
        }
        if self.horizon_distance == f64::MAX {
            return Visibility::None;
        }
        if self.is_bounding_sphere_visible(occludee_bs) {
            Visibility::Full
        } else {
            Visibility::None
        }
    }
}
