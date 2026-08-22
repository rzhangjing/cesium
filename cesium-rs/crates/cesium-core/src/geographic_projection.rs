//! Ported from packages/engine/Source/Core/GeographicProjection.js
//!
//! A simple map projection where longitude and latitude are linearly mapped
//! to X and Y by multiplying them by the ellipsoid's maximum radius.

use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;

/// A simple map projection where longitude and latitude are linearly mapped
/// to X and Y by multiplying them by the {@link Ellipsoid::maximum_radius}.
/// This projection is commonly known as geographic, equirectangular,
/// equidistant cylindrical, or plate carrée. When using the WGS84 ellipsoid,
/// it is also known as EPSG:4326.
#[derive(Debug, Clone)]
pub struct GeographicProjection {
    ellipsoid: Ellipsoid,
    semimajor_axis: f64,
    one_over_semimajor_axis: f64,
}

impl GeographicProjection {
    /// Creates a new `GeographicProjection`.
    ///
    /// * `ellipsoid` — The ellipsoid (defaults to WGS84).
    pub fn new(ellipsoid: Option<Ellipsoid>) -> Self {
        let ellipsoid = ellipsoid.unwrap_or(Ellipsoid::WGS84);
        let semimajor_axis = ellipsoid.maximum_radius();
        let one_over_semimajor_axis = 1.0 / semimajor_axis;
        Self {
            ellipsoid,
            semimajor_axis,
            one_over_semimajor_axis,
        }
    }

    /// Returns the ellipsoid.
    pub fn ellipsoid(&self) -> &Ellipsoid {
        &self.ellipsoid
    }

    /// Projects a set of `Cartographic` coordinates, in radians, to map
    /// coordinates, in meters. X and Y are the longitude and latitude,
    /// respectively, multiplied by the maximum radius of the ellipsoid.
    /// Z is the unmodified height.
    pub fn project(&self, cartographic: &Cartographic) -> Cartesian3 {
        let semimajor_axis = self.semimajor_axis;
        Cartesian3::new(
            cartographic.longitude * semimajor_axis,
            cartographic.latitude * semimajor_axis,
            cartographic.height,
        )
    }

    /// Projects into an existing `Cartesian3`.
    pub fn project_into(
        &self,
        cartographic: &Cartographic,
        result: &mut Cartesian3,
    ) {
        let semimajor_axis = self.semimajor_axis;
        result.x = cartographic.longitude * semimajor_axis;
        result.y = cartographic.latitude * semimajor_axis;
        result.z = cartographic.height;
    }

    /// Unprojects a set of projected `Cartesian3` coordinates, in meters, to
    /// `Cartographic` coordinates, in radians.
    pub fn unproject(&self, cartesian: &Cartesian3) -> Cartographic {
        let one_over = self.one_over_semimajor_axis;
        Cartographic::new(
            cartesian.x * one_over,
            cartesian.y * one_over,
            cartesian.z,
        )
    }

    /// Unprojects into an existing `Cartographic`.
    pub fn unproject_into(
        &self,
        cartesian: &Cartesian3,
        result: &mut Cartographic,
    ) {
        let one_over = self.one_over_semimajor_axis;
        result.longitude = cartesian.x * one_over;
        result.latitude = cartesian.y * one_over;
        result.height = cartesian.z;
    }
}
