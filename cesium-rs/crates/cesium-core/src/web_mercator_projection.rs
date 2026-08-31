//! Ported from packages/engine/Source/Core/WebMercatorProjection.js
//!
//! The map projection used by Google Maps, Bing Maps, and most of ArcGIS
//! Online, EPSG:3857.

use crate::cartesian3::Cartesian3;
use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;
use crate::map_projection::MapProjection;
use crate::math::CesiumMath;

/// The map projection used by Google Maps, Bing Maps, and most of ArcGIS
/// Online, EPSG:3857. This projection uses longitude and latitude expressed
/// with WGS84 and transforms them to Mercator using the spherical (rather
/// than ellipsoidal) equations.
#[derive(Debug, Clone)]
pub struct WebMercatorProjection {
    ellipsoid: Ellipsoid,
    semimajor_axis: f64,
    one_over_semimajor_axis: f64,
}

impl WebMercatorProjection {
    /// Creates a new `WebMercatorProjection`.
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

    /// The maximum latitude (both North and South) supported by a Web Mercator
    /// (EPSG:3857) projection.
    ///
    /// Computed as `mercator_angle_to_geodetic_latitude(PI)` ≈ 1.48442…
    /// (= 85.05112878…°).
    pub const MAXIMUM_LATITUDE: f64 = 1.4844222297453324;

    /// Converts a Mercator angle, in the range -PI to PI, to a geodetic
    /// latitude in the range -PI/2 to PI/2.
    pub fn mercator_angle_to_geodetic_latitude(mercator_angle: f64) -> f64 {
        CesiumMath::PI_OVER_TWO - 2.0 * (-mercator_angle).exp().atan()
    }

    /// Converts a geodetic latitude in radians, in the range -PI/2 to PI/2,
    /// to a Mercator angle in the range -PI to PI.
    pub fn geodetic_latitude_to_mercator_angle(latitude: f64) -> f64 {
        // Clamp the latitude coordinate to the valid Mercator bounds.
        let latitude = if latitude > Self::MAXIMUM_LATITUDE {
            Self::MAXIMUM_LATITUDE
        } else if latitude < -Self::MAXIMUM_LATITUDE {
            -Self::MAXIMUM_LATITUDE
        } else {
            latitude
        };
        let sin_latitude = latitude.sin();
        0.5 * ((1.0 + sin_latitude) / (1.0 - sin_latitude)).ln()
    }

    /// Converts geodetic ellipsoid coordinates, in radians, to the equivalent
    /// Web Mercator X, Y, Z coordinates expressed in meters.
    pub fn project(&self, cartographic: &Cartographic) -> Cartesian3 {
        let semimajor_axis = self.semimajor_axis;
        Cartesian3::new(
            cartographic.longitude * semimajor_axis,
            Self::geodetic_latitude_to_mercator_angle(cartographic.latitude) * semimajor_axis,
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
        result.y =
            Self::geodetic_latitude_to_mercator_angle(cartographic.latitude) * semimajor_axis;
        result.z = cartographic.height;
    }

    /// Converts Web Mercator X, Y coordinates, expressed in meters, to a
    /// `Cartographic` containing geodetic ellipsoid coordinates.
    pub fn unproject(&self, cartesian: &Cartesian3) -> Cartographic {
        let one_over = self.one_over_semimajor_axis;
        Cartographic::new(
            cartesian.x * one_over,
            Self::mercator_angle_to_geodetic_latitude(cartesian.y * one_over),
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
        result.latitude =
            Self::mercator_angle_to_geodetic_latitude(cartesian.y * one_over);
        result.height = cartesian.z;
    }
}

impl MapProjection for WebMercatorProjection {
    fn ellipsoid(&self) -> &Ellipsoid {
        WebMercatorProjection::ellipsoid(self)
    }

    fn project(&self, cartographic: &Cartographic) -> Cartesian3 {
        WebMercatorProjection::project(self, cartographic)
    }

    fn unproject(&self, cartesian: &Cartesian3) -> Cartographic {
        WebMercatorProjection::unproject(self, cartesian)
    }
}
