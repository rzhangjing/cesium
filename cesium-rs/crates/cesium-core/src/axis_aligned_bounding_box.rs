//! Ported from `packages/engine/Source/Core/AxisAlignedBoundingBox.js`.
//!
//! | CesiumJS | Port |
//! | --- | --- |
//! | `new AxisAlignedBoundingBox(minimum, maximum, center)` | [`AxisAlignedBoundingBox::new`] / `Default` |
//! | `AxisAlignedBoundingBox.fromCorners` | [`AxisAlignedBoundingBox::from_corners`] |
//! | `AxisAlignedBoundingBox.fromPoints` | [`AxisAlignedBoundingBox::from_points`] |
//! | `AxisAlignedBoundingBox.clone` | [`AxisAlignedBoundingBox::clone_box`] |
//! | `AxisAlignedBoundingBox.equals` | [`AxisAlignedBoundingBox::equals`] |
//! | `AxisAlignedBoundingBox.intersectPlane` | [`AxisAlignedBoundingBox::intersect_plane`] |
//! | `AxisAlignedBoundingBox.intersectAxisAlignedBoundingBox` | [`AxisAlignedBoundingBox::intersect_axis_aligned_bounding_box`] |
//! | `prototype.clone` | derived [`Clone`] |
//! | `prototype.intersectPlane` | [`AxisAlignedBoundingBox::intersect_plane_instance`] |
//! | `prototype.intersectAxisAlignedBoundingBox` | [`AxisAlignedBoundingBox::intersect_axis_aligned_bounding_box_instance`] |
//! | `prototype.equals` | [`AxisAlignedBoundingBox::equals_instance`] |
//!
//! # Removed fabricated APIs
//!
//! An earlier revision of this file exposed `half_diagonal()` and
//! `intersect_sphere()`. Neither exists in CesiumJS — searching
//! `packages/engine/Source` for `halfDiagonal` or `intersectSphere` returns
//! nothing — and `half_diagonal` was also *wrong*: it returned
//! `maximum - center`, whereas the JS `intersectPlane` derives the positive
//! half diagonal as `(maximum - minimum) * 0.5` and never reads `center` for
//! it, so the two disagree for any box built with an explicit off-midpoint
//! `center` (the `AxisAlignedBoundingBoxSpec` "offset center" case). Both are
//! gone; the half diagonal is now a private helper that matches the JS
//! expression exactly. Tracked in `docs/deviations.md`.

use crate::cartesian3::Cartesian3;
use crate::intersect::Intersect;
use crate::math::{js_max, js_min};
use crate::plane::Plane;

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

    /// Port of `AxisAlignedBoundingBox.fromPoints`.
    ///
    /// Computes a bounding box enclosing all provided positions.
    ///
    /// The accumulator is seeded from `positions[0]` and the loop starts at
    /// index 1, exactly as the JS does.
    ///
    /// The six folds go through [`js_min`] / [`js_max`] rather than
    /// `f64::min` / `f64::max`, because the JS uses `Math.min` / `Math.max`
    /// and the two disagree on `NaN`: `Math.min(x, NaN)` is `NaN`, so one NaN
    /// position poisons the whole box, whereas `f64::min` returns the *other*
    /// operand and swallows it. Seeding from `positions[0]` alone does not fix
    /// that — only the fold operator does.
    pub fn from_points(positions: Option<&[Cartesian3]>) -> Self {
        let result = Self::default();
        let positions = match positions {
            // JS: `!defined(positions) || positions.length === 0` → the result's
            // minimum/maximum/center are reset to `Cartesian3.ZERO` clones,
            // which is what `default()` already holds.
            None => return result,
            Some(positions) if positions.is_empty() => return result,
            Some(positions) => positions,
        };

        let mut minimum_x = positions[0].x;
        let mut minimum_y = positions[0].y;
        let mut minimum_z = positions[0].z;

        let mut maximum_x = positions[0].x;
        let mut maximum_y = positions[0].y;
        let mut maximum_z = positions[0].z;

        for p in positions.iter().skip(1) {
            minimum_x = js_min(minimum_x, p.x);
            maximum_x = js_max(maximum_x, p.x);
            minimum_y = js_min(minimum_y, p.y);
            maximum_y = js_max(maximum_y, p.y);
            minimum_z = js_min(minimum_z, p.z);
            maximum_z = js_max(maximum_z, p.z);
        }

        let minimum = Cartesian3::new(minimum_x, minimum_y, minimum_z);
        let maximum = Cartesian3::new(maximum_x, maximum_y, maximum_z);
        let mut center = Cartesian3::ZERO;
        Cartesian3::midpoint(&minimum, &maximum, &mut center);
        Self {
            minimum,
            maximum,
            center,
        }
    }

    /// Port of `AxisAlignedBoundingBox.fromCorners`.
    ///
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

    /// Port of `AxisAlignedBoundingBox.clone`.
    ///
    /// `None` for `box` mirrors the JS returning `undefined` (the
    /// `clone returns undefined with no parameter` spec case). When `result` is
    /// supplied it is written into *and* returned, matching the JS
    /// `expect(result).toBe(returnedResult)` identity contract.
    pub fn clone_box(box_: Option<&Self>, result: Option<&mut Self>) -> Option<Self> {
        let box_ = box_?;
        match result {
            Some(r) => {
                r.minimum = box_.minimum;
                r.maximum = box_.maximum;
                r.center = box_.center;
                Some(r.clone())
            }
            None => Some(box_.clone()),
        }
    }

    /// Port of `AxisAlignedBoundingBox.prototype.clone`.
    pub fn clone_instance(&self) -> Self {
        Self::clone_box(Some(self), None).expect("box is defined")
    }

    /// The positive half diagonal, `h`, of [`Self::intersect_plane`].
    ///
    /// Private because CesiumJS has no such public API — it computes `h`
    /// inline. Note it is `(maximum - minimum) * 0.5`, *not*
    /// `maximum - center`: the JS never consults `center` here.
    fn positive_half_diagonal(&self) -> Cartesian3 {
        let mut diagonal = Cartesian3::ZERO;
        Cartesian3::subtract(&self.maximum, &self.minimum, &mut diagonal);
        let mut h = Cartesian3::ZERO;
        Cartesian3::multiply_by_scalar(&diagonal, 0.5, &mut h);
        h
    }

    /// Port of `AxisAlignedBoundingBox.intersectPlane`.
    ///
    /// Determines which side of a plane a box is located. Returns
    /// [`Intersect::Inside`] if the entire box is on the side of the plane the
    /// normal is pointing, [`Intersect::Outside`] if the entire box is on the
    /// opposite side, and [`Intersect::Intersecting`] if the box intersects
    /// the plane.
    ///
    /// This is the method `CullingVolume.computeVisibility` duck-types on, so
    /// it is what lets a culling volume contain an axis aligned bounding box
    /// and not just a bounding sphere.
    pub fn intersect_plane(box_: &Self, plane: &Plane) -> Intersect {
        let h = box_.positive_half_diagonal();
        let normal = &plane.normal;
        let e = h.x * normal.x.abs() + h.y * normal.y.abs() + h.z * normal.z.abs();
        // Signed distance from the center.
        let s = Cartesian3::dot(&box_.center, normal) + plane.distance;

        if s - e > 0.0 {
            return Intersect::Inside;
        }

        if s + e < 0.0 {
            // Not in front because normals point inward.
            return Intersect::Outside;
        }

        Intersect::Intersecting
    }

    /// Port of `AxisAlignedBoundingBox.prototype.intersectPlane`.
    pub fn intersect_plane_instance(&self, plane: &Plane) -> Intersect {
        Self::intersect_plane(self, plane)
    }

    /// Port of `AxisAlignedBoundingBox.intersectAxisAlignedBoundingBox`.
    ///
    /// Determines whether two axis aligned bounding boxes intersect. The six
    /// comparisons short-circuit in favour of boxes that do *not* intersect,
    /// in the same order as the JS.
    pub fn intersect_axis_aligned_bounding_box(box_: &Self, other: &Self) -> bool {
        box_.minimum.x <= other.maximum.x
            && box_.maximum.x >= other.minimum.x
            && box_.minimum.y <= other.maximum.y
            && box_.maximum.y >= other.minimum.y
            && box_.minimum.z <= other.maximum.z
            && box_.maximum.z >= other.minimum.z
    }

    /// Port of `AxisAlignedBoundingBox.prototype.intersectAxisAlignedBoundingBox`.
    pub fn intersect_axis_aligned_bounding_box_instance(&self, other: &Self) -> bool {
        Self::intersect_axis_aligned_bounding_box(self, other)
    }

    /// Port of `AxisAlignedBoundingBox.equals`.
    ///
    /// Compares componentwise. `None` mirrors JS `undefined`; the JS
    /// `left === right` reference-identity short circuit is reproduced by
    /// `(None, None) => true` (two `undefined`s are `===`), and for two `Some`
    /// boxes Rust's structural comparison already agrees with the JS field
    /// checks. The comparison order — center, then minimum, then maximum — is
    /// preserved. Matches [`Cartesian3::equals`], which handles the same case
    /// the same way.
    pub fn equals(left: Option<&Self>, right: Option<&Self>) -> bool {
        match (left, right) {
            (Some(left), Some(right)) => {
                Cartesian3::equals(Some(&left.center), Some(&right.center))
                    && Cartesian3::equals(Some(&left.minimum), Some(&right.minimum))
                    && Cartesian3::equals(Some(&left.maximum), Some(&right.maximum))
            }
            // JS: `left === right` — true when both are `undefined`.
            (None, None) => true,
            _ => false,
        }
    }

    /// Port of `AxisAlignedBoundingBox.prototype.equals`.
    pub fn equals_instance(&self, right: Option<&Self>) -> bool {
        Self::equals(Some(self), right)
    }
}
