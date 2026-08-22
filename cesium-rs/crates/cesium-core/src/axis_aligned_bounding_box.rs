//! Ported from `packages/engine/Source/Core/AxisAlignedBoundingBox.js`.

use crate::cartesian3::Cartesian3;
use crate::bounding_sphere::BoundingSphere;
use crate::intersect::Intersect;

/// An axis-aligned bounding box defined by minimum and maximum points.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisAlignedBoundingBox {
    /// The minimum point defining the bounding box.
    pub minimum: Cartesian3,
    /// The maximum point defining the bounding box.
    pub maximum: Cartesian3,
    /// The center point of the bounding box.
    pub center: Cartesian3,
}

impl Default for AxisAlignedBoundingBox {
    fn default() -> Self {
        Self {
            minimum: Cartesian3::ZERO,
            maximum: Cartesian3::ZERO,
            center: Cartesian3::ZERO,
        }
    }
}

impl AxisAlignedBoundingBox {
    pub fn new(minimum: Cartesian3, maximum: Cartesian3, center: Option<Cartesian3>) -> Self {
        let center = center.unwrap_or_else(|| {
            let mut c = Cartesian3::ZERO;
            Cartesian3::midpoint(&minimum, &maximum, &mut c);
            c
        });
        Self {
            minimum,
            maximum,
            center,
        }
    }

    /// Creates from minimum and maximum corners.
    pub fn from_corners(minimum: &Cartesian3, maximum: &Cartesian3) -> Self {
        let mut center = Cartesian3::ZERO;
        Cartesian3::midpoint(minimum, maximum, &mut center);
        Self {
            minimum: *minimum,
            maximum: *maximum,
            center,
        }
    }

    /// Computes the half-diagonal vector (half extents).
    pub fn half_diagonal(&self) -> Cartesian3 {
        let mut result = Cartesian3::ZERO;
        Cartesian3::subtract(&self.maximum, &self.center, &mut result);
        result
    }

    /// Determines if this box intersects another bounding sphere.
    pub fn intersect_sphere(&self, sphere: &BoundingSphere) -> Intersect {
        let center = &sphere.center;
        let radius = sphere.radius;

        let mut distance = 0.0_f64;
        for i in 0..3 {
            let c = match i {
                0 => center.x,
                1 => center.y,
                _ => center.z,
            };
            let (min_v, max_v) = if i == 0 {
                (self.minimum.x, self.maximum.x)
            } else if i == 1 {
                (self.minimum.y, self.maximum.y)
            } else {
                (self.minimum.z, self.maximum.z)
            };

            if c < min_v {
                distance += (c - min_v) * (c - min_v);
            } else if c > max_v {
                distance += (c - max_v) * (c - max_v);
            }
        }

        if distance > radius * radius {
            Intersect::Outside
        } else {
            // Simplified: check if fully inside
            let half = self.half_diagonal();
            let dx = (center.x - self.center.x).abs() + radius;
            let dy = (center.y - self.center.y).abs() + radius;
            let dz = (center.z - self.center.z).abs() + radius;
            if dx <= half.x && dy <= half.y && dz <= half.z {
                Intersect::Inside
            } else {
                Intersect::Intersecting
            }
        }
    }

    /// Determines if two boxes are equal.
    pub fn equals(left: &Self, right: &Self) -> bool {
        left == right
    }
}
