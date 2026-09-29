//! 地图投影 —— 经纬度（等角圆柱）投影与 Web Mercator 投影。
//! 映射到 CesiumJS `Core/GeographicProjection.js`, `Core/WebMercatorProjection.js`

use crate::cartographic::Cartographic;
use crate::ellipsoid::Ellipsoid;
use crate::math_utils;
use glam::DVec3;
use serde::{Deserialize, Serialize};

/// 地图投影的 trait。
pub trait MapProjection: Send + Sync {
    /// 将测绘位置投影为投影坐标 (x, y, z=height)。
    fn project(&self, cartographic: &Cartographic) -> DVec3;
    /// 将投影坐标反投影回测绘坐标。
    fn unproject(&self, projected: DVec3) -> Cartographic;
    /// 本投影所使用的椭球。
    fn ellipsoid(&self) -> &Ellipsoid;
}

/// 经纬度（等角圆柱）投影。
/// 映射到 CesiumJS `GeographicProjection`
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GeographicProjection {
    ellipsoid: Ellipsoid,
    semimajor_axis: f64,
    one_over_semimajor_axis: f64,
}

impl GeographicProjection {
    pub fn new(ellipsoid: Ellipsoid) -> Self {
        let semimajor_axis = ellipsoid.maximum_radius();
        Self {
            ellipsoid,
            semimajor_axis,
            one_over_semimajor_axis: 1.0 / semimajor_axis,
        }
    }

    pub fn wgs84() -> Self {
        Self::new(Ellipsoid::WGS84)
    }

    #[inline]
    pub fn semimajor_axis(&self) -> f64 {
        self.semimajor_axis
    }
}

impl MapProjection for GeographicProjection {
    fn project(&self, cartographic: &Cartographic) -> DVec3 {
        DVec3::new(
            cartographic.longitude * self.semimajor_axis,
            cartographic.latitude * self.semimajor_axis,
            cartographic.height,
        )
    }

    fn unproject(&self, projected: DVec3) -> Cartographic {
        Cartographic {
            longitude: projected.x * self.one_over_semimajor_axis,
            latitude: projected.y * self.one_over_semimajor_axis,
            height: projected.z,
        }
    }

    fn ellipsoid(&self) -> &Ellipsoid {
        &self.ellipsoid
    }
}

/// Web Mercator 投影（EPSG:3857）。
/// 映射到 CesiumJS `WebMercatorProjection`
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WebMercatorProjection {
    ellipsoid: Ellipsoid,
    semimajor_axis: f64,
    one_over_semimajor_axis: f64,
    maximum_latitude: f64,
}

impl WebMercatorProjection {
    /// Web Mercator 的最大纬度（约 85.0511287798 度）。
    /// 计算方式为：PI/2 - 2*atan(exp(-PI))
    pub const MAXIMUM_LATITUDE: f64 = 1.4844222297453324;

    pub fn new(ellipsoid: Ellipsoid) -> Self {
        let semimajor_axis = ellipsoid.maximum_radius();
        Self {
            ellipsoid,
            semimajor_axis,
            one_over_semimajor_axis: 1.0 / semimajor_axis,
            maximum_latitude: Self::MAXIMUM_LATITUDE,
        }
    }

    pub fn wgs84() -> Self {
        Self::new(Ellipsoid::WGS84)
    }

    #[inline]
    pub fn semimajor_axis(&self) -> f64 {
        self.semimajor_axis
    }

    #[inline]
    pub fn maximum_latitude(&self) -> f64 {
        self.maximum_latitude
    }

    /// 由纬度计算 mercator 角。
    /// 映射到 `WebMercatorProjection.geodeticLatitudeToMercatorAngle`
    pub fn geodetic_latitude_to_mercator_angle(latitude: f64) -> f64 {
        let clamped = math_utils::clamp(latitude, -Self::MAXIMUM_LATITUDE, Self::MAXIMUM_LATITUDE);
        let sin_latitude = clamped.sin();
        0.5 * ((1.0 + sin_latitude) / (1.0 - sin_latitude)).ln()
    }

    /// 由 mercator 角计算纬度。
    /// 映射到 `WebMercatorProjection.mercatorAngleToGeodeticLatitude`
    pub fn mercator_angle_to_geodetic_latitude(mercator_angle: f64) -> f64 {
        math_utils::PI_OVER_TWO - 2.0 * (-mercator_angle).exp().atan()
    }
}

impl MapProjection for WebMercatorProjection {
    fn project(&self, cartographic: &Cartographic) -> DVec3 {
        let y = Self::geodetic_latitude_to_mercator_angle(cartographic.latitude)
            * self.semimajor_axis;
        DVec3::new(
            cartographic.longitude * self.semimajor_axis,
            y,
            cartographic.height,
        )
    }

    fn unproject(&self, projected: DVec3) -> Cartographic {
        let longitude = projected.x * self.one_over_semimajor_axis;
        let mercator_angle = projected.y * self.one_over_semimajor_axis;
        let latitude = Self::mercator_angle_to_geodetic_latitude(mercator_angle);
        Cartographic {
            longitude,
            latitude,
            height: projected.z,
        }
    }

    fn ellipsoid(&self) -> &Ellipsoid {
        &self.ellipsoid
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geographic_projection_roundtrip() {
        let proj = GeographicProjection::wgs84();
        let c = Cartographic::from_degrees(45.0, 30.0, 1000.0);
        let projected = proj.project(&c);
        let result = proj.unproject(projected);
        assert!((result.longitude - c.longitude).abs() < 1e-10);
        assert!((result.latitude - c.latitude).abs() < 1e-10);
        assert!((result.height - c.height).abs() < 1e-10);
    }

    #[test]
    fn test_web_mercator_projection_roundtrip() {
        let proj = WebMercatorProjection::wgs84();
        let c = Cartographic::from_degrees(45.0, 30.0, 500.0);
        let projected = proj.project(&c);
        let result = proj.unproject(projected);
        assert!((result.longitude - c.longitude).abs() < 1e-10);
        assert!((result.latitude - c.latitude).abs() < 1e-10);
        assert!((result.height - c.height).abs() < 1e-10);
    }

    #[test]
    fn test_web_mercator_equator() {
        let proj = WebMercatorProjection::wgs84();
        let c = Cartographic::from_radians(0.0, 0.0, 0.0);
        let projected = proj.project(&c);
        assert!(projected.x.abs() < 1e-10);
        assert!(projected.y.abs() < 1e-10);
    }

    #[test]
    fn test_mercator_angle_conversion() {
        let lat = math_utils::to_radians(45.0);
        let angle = WebMercatorProjection::geodetic_latitude_to_mercator_angle(lat);
        let back = WebMercatorProjection::mercator_angle_to_geodetic_latitude(angle);
        assert!((back - lat).abs() < 1e-10);
    }

    #[test]
    fn test_maximum_latitude() {
        let max_lat = WebMercatorProjection::MAXIMUM_LATITUDE;
        assert!((math_utils::to_degrees(max_lat) - 85.0511287798).abs() < 1e-6);
    }
}
