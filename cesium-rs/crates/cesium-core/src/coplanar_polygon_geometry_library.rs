//! Ported from `packages/engine/Source/Core/CoplanarPolygonGeometryLibrary.js`.
//!
//! Library for computing a 2D projection plane of coplanar polygon positions
//! and projecting points/positions into that plane.

use crate::cartesian2::Cartesian2;
use crate::cartesian3::Cartesian3;
use crate::matrix3::Matrix3;
use crate::oriented_bounding_box::OrientedBoundingBox;

/// Private namespace `CoplanarPolygonGeometryLibrary`.
pub struct CoplanarPolygonGeometryLibrary;

impl CoplanarPolygonGeometryLibrary {
    /// Port of `CoplanarPolygonGeometryLibrary.validOutline`.
    ///
    /// Returns `false` if all the points are on a line (a polygon can't be
    /// drawn). The JS `positions is required` debug check is enforced by
    /// Rust's type system.
    pub fn valid_outline(positions: &[Cartesian3]) -> bool {
        let oriented_bounding_box = OrientedBoundingBox::from_points(Some(positions), None);
        let half_axes = &oriented_bounding_box.half_axes;

        let mut x_axis = Cartesian3::ZERO;
        let mut y_axis = Cartesian3::ZERO;
        let mut z_axis = Cartesian3::ZERO;
        Matrix3::get_column(half_axes, 0, &mut x_axis);
        Matrix3::get_column(half_axes, 1, &mut y_axis);
        Matrix3::get_column(half_axes, 2, &mut z_axis);

        let x_mag = Cartesian3::magnitude(&x_axis);
        let y_mag = Cartesian3::magnitude(&y_axis);
        let z_mag = Cartesian3::magnitude(&z_axis);

        // If all the points are on a line return undefined because we can't draw a polygon
        !(
            (x_mag == 0.0 && (y_mag == 0.0 || z_mag == 0.0)) ||
            (y_mag == 0.0 && z_mag == 0.0)
        )
    }

    /// Port of `CoplanarPolygonGeometryLibrary.computeProjectTo2DArguments`
    /// (call after `removeDuplicates`). Returns `false` if the positions are
    /// coplanar only along a line.
    pub fn compute_project_to_2d_arguments(
        positions: &[Cartesian3],
        center_result: &mut Cartesian3,
        plane_axis1_result: &mut Cartesian3,
        plane_axis2_result: &mut Cartesian3,
    ) -> bool {
        let oriented_bounding_box = OrientedBoundingBox::from_points(Some(positions), None);
        let half_axes = &oriented_bounding_box.half_axes;

        let mut x_axis = Cartesian3::ZERO;
        let mut y_axis = Cartesian3::ZERO;
        let mut z_axis = Cartesian3::ZERO;
        Matrix3::get_column(half_axes, 0, &mut x_axis);
        Matrix3::get_column(half_axes, 1, &mut y_axis);
        Matrix3::get_column(half_axes, 2, &mut z_axis);

        let x_mag = Cartesian3::magnitude(&x_axis);
        let y_mag = Cartesian3::magnitude(&y_axis);
        let z_mag = Cartesian3::magnitude(&z_axis);
        let min = x_mag.min(y_mag).min(z_mag);

        // If all the points are on a line return undefined because we can't draw a polygon
        if (x_mag == 0.0 && (y_mag == 0.0 || z_mag == 0.0)) || (y_mag == 0.0 && z_mag == 0.0) {
            return false;
        }

        let mut plane_axis1: Option<Cartesian3> = None;
        let mut plane_axis2: Option<Cartesian3> = None;

        if min == y_mag || min == z_mag {
            plane_axis1 = Some(x_axis);
        }
        if min == x_mag {
            plane_axis1 = Some(y_axis);
        } else if min == z_mag {
            plane_axis2 = Some(y_axis);
        }
        if min == x_mag || min == y_mag {
            plane_axis2 = Some(z_axis);
        }

        // The degenerate check above guarantees both axes were assigned.
        Cartesian3::normalize(&plane_axis1.unwrap(), plane_axis1_result);
        Cartesian3::normalize(&plane_axis2.unwrap(), plane_axis2_result);
        *center_result = oriented_bounding_box.center;
        true
    }

    /// Port of `CoplanarPolygonGeometryLibrary.createProjectPointsTo2DFunction`.
    ///
    /// DEVIATION: the JS returned closure allocates a fresh `Cartesian2` per
    /// point; this port captures the center/axes by value and returns a
    /// boxed closure with the same behavior.
    pub fn create_project_points_to_2d_function(
        center: &Cartesian3,
        axis1: &Cartesian3,
        axis2: &Cartesian3,
    ) -> Box<dyn Fn(&[Cartesian3]) -> Vec<Cartesian2>> {
        let center = *center;
        let axis1 = *axis1;
        let axis2 = *axis2;
        Box::new(move |positions: &[Cartesian3]| {
            let mut position_results = Vec::with_capacity(positions.len());
            for position in positions {
                position_results.push(Self::project_to_2d_new(position, &center, &axis1, &axis2));
            }
            position_results
        })
    }

    /// Port of `CoplanarPolygonGeometryLibrary.createProjectPointTo2DFunction`.
    pub fn create_project_point_to_2d_function(
        center: &Cartesian3,
        axis1: &Cartesian3,
        axis2: &Cartesian3,
    ) -> Box<dyn Fn(&Cartesian3, &mut Cartesian2)> {
        let center = *center;
        let axis1 = *axis1;
        let axis2 = *axis2;
        Box::new(move |position: &Cartesian3, result: &mut Cartesian2| {
            Self::project_to_2d(position, &center, &axis1, &axis2, result);
        })
    }

    /// `projectTo2D` (module-private in JS) with a result out-parameter.
    pub fn project_to_2d<'a>(
        position: &Cartesian3,
        center: &Cartesian3,
        axis1: &Cartesian3,
        axis2: &Cartesian3,
        result: &'a mut Cartesian2,
    ) -> &'a mut Cartesian2 {
        let mut v = Cartesian3::ZERO;
        Cartesian3::subtract(position, center, &mut v);
        let x = Cartesian3::dot(axis1, &v);
        let y = Cartesian3::dot(axis2, &v);

        Cartesian2::from_elements(x, y, result);
        result
    }

    /// Allocating variant of [`Self::project_to_2d`].
    pub fn project_to_2d_new(
        position: &Cartesian3,
        center: &Cartesian3,
        axis1: &Cartesian3,
        axis2: &Cartesian3,
    ) -> Cartesian2 {
        let mut result = Cartesian2::default();
        Self::project_to_2d(position, center, axis1, axis2, &mut result);
        result
    }
}
