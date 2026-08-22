//! Ported from `packages/engine/Source/Core/CullingVolume.js`.

use crate::cartesian3::Cartesian3;
use crate::cartesian4::Cartesian4;
use crate::bounding_sphere::BoundingSphere;
use crate::intersect::Intersect;
use crate::plane::Plane;

/// A culling volume defined by planes.
///
/// Each plane is represented by a Cartesian4, where x, y, z define the unit
/// normal and w is the signed distance from the origin.
#[derive(Clone, Debug)]
pub struct CullingVolume {
    /// The clipping planes.
    pub planes: Vec<Cartesian4>,
}

impl Default for CullingVolume {
    fn default() -> Self {
        Self { planes: Vec::new() }
    }
}

/// For plane masks, represents entirely outside the culling volume.
pub const MASK_OUTSIDE: u32 = 0xFFFFFFFF;
/// For plane masks, represents entirely inside the culling volume.
pub const MASK_INSIDE: u32 = 0x00000000;
/// For plane masks, represents possibly intersecting all planes.
pub const MASK_INDETERMINATE: u32 = 0x7FFFFFFF;

const FACE_NORMALS: [Cartesian3; 3] = [
    Cartesian3 { x: 1.0, y: 0.0, z: 0.0 },
    Cartesian3 { x: 0.0, y: 1.0, z: 0.0 },
    Cartesian3 { x: 0.0, y: 0.0, z: 1.0 },
];

impl CullingVolume {
    /// Constructs a culling volume from a bounding sphere. Creates six planes
    /// that create a box containing the sphere, aligned to x/y/z axes.
    pub fn from_bounding_sphere(bounding_sphere: &BoundingSphere, result: Option<&mut Self>) -> Self {
        let mut r = result.cloned().unwrap_or_default();

        let center = bounding_sphere.center;
        let radius = bounding_sphere.radius;

        r.planes.resize(6, Cartesian4::ZERO);

        let mut plane_index = 0;
        for face_normal in &FACE_NORMALS {
            // Near plane: center - radius * normal
            let mut scratch_center = Cartesian3::ZERO;
            Cartesian3::multiply_by_scalar(face_normal, -radius, &mut scratch_center);
            let mut plane_center = Cartesian3::ZERO;
            Cartesian3::add(&center, &scratch_center, &mut plane_center);

            r.planes[plane_index] = Cartesian4::new(
                face_normal.x,
                face_normal.y,
                face_normal.z,
                -Cartesian3::dot(face_normal, &plane_center),
            );

            // Far plane: center + radius * normal
            let mut scratch_center2 = Cartesian3::ZERO;
            Cartesian3::multiply_by_scalar(face_normal, radius, &mut scratch_center2);
            let mut plane_center2 = Cartesian3::ZERO;
            Cartesian3::add(&center, &scratch_center2, &mut plane_center2);

            let neg_normal = Cartesian3::new(-face_normal.x, -face_normal.y, -face_normal.z);
            r.planes[plane_index + 1] = Cartesian4::new(
                neg_normal.x,
                neg_normal.y,
                neg_normal.z,
                -Cartesian3::dot(&neg_normal, &plane_center2),
            );

            plane_index += 2;
        }

        r
    }

    /// Determines whether a bounding volume intersects the culling volume.
    pub fn compute_visibility(&self, bounding_volume: &BoundingSphere) -> Intersect {
        let mut intersecting = false;

        for plane4 in &self.planes {
            let plane = Plane::from_cartesian4_new(plane4);
            let result = BoundingSphere::intersect_plane(bounding_volume, &plane);
            if result == Intersect::Outside {
                return Intersect::Outside;
            } else if result == Intersect::Intersecting {
                intersecting = true;
            }
        }

        if intersecting {
            Intersect::Intersecting
        } else {
            Intersect::Inside
        }
    }

    /// Determines whether a bounding volume intersects the culling volume,
    /// using a plane mask to skip redundant checks.
    pub fn compute_visibility_with_plane_mask(
        &self,
        bounding_volume: &BoundingSphere,
        parent_plane_mask: u32,
    ) -> u32 {
        if parent_plane_mask == MASK_OUTSIDE || parent_plane_mask == MASK_INSIDE {
            return parent_plane_mask;
        }

        let mut mask = MASK_INSIDE;

        for (k, plane4) in self.planes.iter().enumerate() {
            let flag = if k < 31 { 1u32 << k } else { 0u32 };
            if k < 31 && (parent_plane_mask & flag) == 0 {
                continue;
            }

            let plane = Plane::from_cartesian4_new(plane4);
            let result = BoundingSphere::intersect_plane(bounding_volume, &plane);
            if result == Intersect::Outside {
                return MASK_OUTSIDE;
            } else if result == Intersect::Intersecting {
                mask |= flag;
            }
        }

        mask
    }
}
