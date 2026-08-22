//! Ported from packages/engine/Source/Core/Plane.js
//!
//! A plane in Hessian Normal Form defined by
//! ```text
//! ax + by + cz + d = 0
//! ```
//! where `(a, b, c)` is the plane's `normal`, `d` is the signed
//! `distance` to the plane, and `(x, y, z)` is any point on
//! the plane.

use crate::cartesian3::Cartesian3;
use crate::cartesian4::Cartesian4;
use crate::developer_error::throw_developer_error;
use crate::math::CesiumMath;

/// A plane in Hessian Normal Form.
///
/// Port of `Plane`.
#[derive(Clone, Debug)]
pub struct Plane {
    /// The plane's normal (must be normalized).
    ///
    /// Port of `Plane#normal`.
    pub normal: Cartesian3,

    /// The shortest distance from the origin to the plane.
    ///
    /// Port of `Plane#distance`.
    pub distance: f64,
}

impl Plane {
    /// Creates a new `Plane`.
    ///
    /// Port of the `Plane(normal, distance)` constructor.
    ///
    /// # Panics
    /// Panics with `DeveloperError` if `normal` is not normalized or
    /// `distance` is not a finite number.
    pub fn new(normal: &Cartesian3, distance: f64) -> Self {
        //>>includeStart('debug', pragmas.debug);
        if !CesiumMath::equals_epsilon(
            Cartesian3::magnitude(normal),
            1.0,
            Some(CesiumMath::EPSILON6),
            None,
        ) {
            throw_developer_error("normal must be normalized.");
        }
        //>>includeEnd('debug');

        Self {
            normal: *normal,
            distance,
        }
    }

    /// A constant initialized to the XY plane passing through the origin,
    /// with normal in positive Z.
    ///
    /// Port of `Plane.ORIGIN_XY_PLANE`.
    pub const ORIGIN_XY_PLANE: Plane = Plane {
        normal: Cartesian3::UNIT_Z,
        distance: 0.0,
    };

    /// A constant initialized to the YZ plane passing through the origin,
    /// with normal in positive X.
    ///
    /// Port of `Plane.ORIGIN_YZ_PLANE`.
    pub const ORIGIN_YZ_PLANE: Plane = Plane {
        normal: Cartesian3::UNIT_X,
        distance: 0.0,
    };

    /// A constant initialized to the ZX plane passing through the origin,
    /// with normal in positive Y.
    ///
    /// Port of `Plane.ORIGIN_ZX_PLANE`.
    pub const ORIGIN_ZX_PLANE: Plane = Plane {
        normal: Cartesian3::UNIT_Y,
        distance: 0.0,
    };

    /// Creates a plane from a normal and a point on the plane.
    ///
    /// Port of `Plane.fromPointNormal`.
    ///
    /// # Panics
    /// Panics with `DeveloperError` if `normal` is not normalized.
    pub fn from_point_normal(point: &Cartesian3, normal: &Cartesian3, result: &mut Self) {
        //>>includeStart('debug', pragmas.debug);
        if !CesiumMath::equals_epsilon(
            Cartesian3::magnitude(normal),
            1.0,
            Some(CesiumMath::EPSILON6),
            None,
        ) {
            throw_developer_error("normal must be normalized.");
        }
        //>>includeEnd('debug');

        let distance = -Cartesian3::dot(normal, point);
        result.normal = *normal;
        result.distance = distance;
    }

    /// Allocating variant of [`Plane::from_point_normal`].
    pub fn from_point_normal_new(point: &Cartesian3, normal: &Cartesian3) -> Self {
        let mut result = Self {
            normal: Cartesian3::ZERO,
            distance: 0.0,
        };
        Self::from_point_normal(point, normal, &mut result);
        result
    }

    /// Creates a plane from the general equation given as a `Cartesian4`.
    ///
    /// Port of `Plane.fromCartesian4`.
    ///
    /// # Panics
    /// Panics with `DeveloperError` if the normal extracted from `coefficients`
    /// is not normalized.
    pub fn from_cartesian4(coefficients: &Cartesian4, result: &mut Self) {
        let normal = Cartesian3::new(coefficients.x, coefficients.y, coefficients.z);
        let distance = coefficients.w;

        //>>includeStart('debug', pragmas.debug);
        if !CesiumMath::equals_epsilon(
            Cartesian3::magnitude(&normal),
            1.0,
            Some(CesiumMath::EPSILON6),
            None,
        ) {
            throw_developer_error("normal must be normalized.");
        }
        //>>includeEnd('debug');

        result.normal = normal;
        result.distance = distance;
    }

    /// Allocating variant of [`Plane::from_cartesian4`].
    pub fn from_cartesian4_new(coefficients: &Cartesian4) -> Self {
        let mut result = Self {
            normal: Cartesian3::ZERO,
            distance: 0.0,
        };
        Self::from_cartesian4(coefficients, &mut result);
        result
    }

    /// Computes the signed shortest distance of a point to a plane.
    ///
    /// Port of `Plane.getPointDistance`.
    pub fn get_point_distance(plane: &Self, point: &Cartesian3) -> f64 {
        Cartesian3::dot(&plane.normal, point) + plane.distance
    }

    /// Projects a point onto the plane.
    ///
    /// Port of `Plane.projectPointOntoPlane`.
    pub fn project_point_onto_plane(
        plane: &Self,
        point: &Cartesian3,
        result: &mut Cartesian3,
    ) {
        // projectedPoint = point - (normal·point + distance) * normal
        let point_distance = Self::get_point_distance(plane, point);
        let mut scaled_normal = Cartesian3::default();
        Cartesian3::multiply_by_scalar(&plane.normal, point_distance, &mut scaled_normal);
        Cartesian3::subtract(point, &scaled_normal, result);
    }

    /// Allocating variant of [`Plane::project_point_onto_plane`].
    pub fn project_point_onto_plane_new(plane: &Self, point: &Cartesian3) -> Cartesian3 {
        let mut result = Cartesian3::default();
        Self::project_point_onto_plane(plane, point, &mut result);
        result
    }

    /// Transforms the plane by the given transformation matrix.
    ///
    /// Port of `Plane.transform`.
    ///
    /// DEVIATION (deferred): requires `Matrix4::inverse_transpose` and
    /// `Matrix4::multiply_by_vector`; will be enabled once `Matrix4` is
    /// ported. See `docs/deferred.md`.
    // pub fn transform(plane: &Self, transform: &Matrix4, result: &mut Self) { ... }

    /// Duplicates a `Plane` instance.
    ///
    /// Port of `Plane.clone`.
    pub fn clone_plane(plane: &Self, result: &mut Self) {
        result.normal = plane.normal;
        result.distance = plane.distance;
    }

    /// Allocating variant of [`Plane::clone_plane`].
    pub fn clone_new(plane: &Self) -> Self {
        Self {
            normal: plane.normal,
            distance: plane.distance,
        }
    }

    /// Compares two planes by normal and distance.
    ///
    /// Port of `Plane.equals`.
    pub fn equals(left: &Self, right: &Self) -> bool {
        left.distance == right.distance && left.normal == right.normal
    }
}

impl PartialEq for Plane {
    fn eq(&self, other: &Self) -> bool {
        Self::equals(self, other)
    }
}
