//! 3D Tiles bounding volume definitions.
//!
//! Maps to CesiumJS `Scene/Cesium3DTileBoundingVolume.js`
//! Supports three types: Box (OBB), Region (geographic), and Sphere.

use cesium_geospatial::bounding::BoundingSphere;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::rectangle::Rectangle;
use glam::DVec3;
use serde::{Deserialize, Serialize};

/// A bounding volume for a 3D Tile.
///
/// Maps to the `boundingVolume` property in tileset.json.
/// Three types are supported per the 3D Tiles specification:
/// - `box`: An oriented bounding box (center + 3 half-axis vectors)
/// - `region`: A geographic region [west, south, east, north, minHeight, maxHeight]
/// - `sphere`: A bounding sphere [centerX, centerY, centerZ, radius]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BoundingVolume {
    /// Oriented bounding box: [cx, cy, cz, xDirX, xDirY, xDirZ, yDirX, yDirY, yDirZ, zDirX, zDirY, zDirZ]
    Box([f64; 12]),
    /// Geographic region: [west, south, east, north, minHeight, maxHeight] (radians + meters)
    Region([f64; 6]),
    /// Bounding sphere: [centerX, centerY, centerY, radius]
    Sphere([f64; 4]),
}

impl BoundingVolume {
    /// Creates a bounding box from center and half-axis vectors.
    pub fn from_box(center: DVec3, half_x: DVec3, half_y: DVec3, half_z: DVec3) -> Self {
        BoundingVolume::Box([
            center.x, center.y, center.z,
            half_x.x, half_x.y, half_x.z,
            half_y.x, half_y.y, half_y.z,
            half_z.x, half_z.y, half_z.z,
        ])
    }

    /// Creates a bounding sphere from center and radius.
    pub fn from_sphere(center: DVec3, radius: f64) -> Self {
        BoundingVolume::Sphere([center.x, center.y, center.z, radius])
    }

    /// Creates a geographic region bounding volume.
    pub fn from_region(west: f64, south: f64, east: f64, north: f64, min_height: f64, max_height: f64) -> Self {
        BoundingVolume::Region([west, south, east, north, min_height, max_height])
    }

    /// Gets the center of the bounding volume in ECEF coordinates.
    pub fn center(&self, ellipsoid: &Ellipsoid) -> DVec3 {
        match self {
            BoundingVolume::Box(data) => DVec3::new(data[0], data[1], data[2]),
            BoundingVolume::Sphere(data) => DVec3::new(data[0], data[1], data[2]),
            BoundingVolume::Region(data) => {
                let lon = (data[0] + data[2]) / 2.0;
                let lat = (data[1] + data[3]) / 2.0;
                let height = (data[4] + data[5]) / 2.0;
                ellipsoid.cartographic_to_cartesian(
                    &cesium_geospatial::cartographic::Cartographic::from_radians(lon, lat, height),
                )
            }
        }
    }

    /// Converts this bounding volume to a BoundingSphere for distance calculations.
    pub fn to_bounding_sphere(&self, ellipsoid: &Ellipsoid) -> BoundingSphere {
        match self {
            BoundingVolume::Sphere(data) => {
                BoundingSphere::new(DVec3::new(data[0], data[1], data[2]), data[3])
            }
            BoundingVolume::Box(data) => {
                let center = DVec3::new(data[0], data[1], data[2]);
                let half_x = DVec3::new(data[3], data[4], data[5]);
                let half_y = DVec3::new(data[6], data[7], data[8]);
                let half_z = DVec3::new(data[9], data[10], data[11]);
                // Radius is the length of the longest diagonal
                let radius = (half_x.length_squared()
                    + half_y.length_squared()
                    + half_z.length_squared())
                .sqrt();
                BoundingSphere::new(center, radius)
            }
            BoundingVolume::Region(data) => {
                let rect = Rectangle::new(data[0], data[1], data[2], data[3]);
                let min_h = data[4];
                let max_h = data[5];
                // Approximate with a sphere
                let center_carto = cesium_geospatial::cartographic::Cartographic::from_radians(
                    (rect.west + rect.east) / 2.0,
                    (rect.south + rect.north) / 2.0,
                    (min_h + max_h) / 2.0,
                );
                let center = ellipsoid.cartographic_to_cartesian(&center_carto);

                // Compute radius from corner distances
                let sw = ellipsoid.cartographic_to_cartesian(
                    &cesium_geospatial::cartographic::Cartographic::from_radians(
                        rect.west, rect.south, min_h,
                    ),
                );
                let ne = ellipsoid.cartographic_to_cartesian(
                    &cesium_geospatial::cartographic::Cartographic::from_radians(
                        rect.east, rect.north, max_h,
                    ),
                );
                let radius = center.distance(sw).max(center.distance(ne));
                BoundingSphere::new(center, radius)
            }
        }
    }

    /// Computes the distance from a point to the bounding volume.
    ///
    /// Returns 0 if the point is inside the volume.
    pub fn distance_to(&self, point: DVec3, ellipsoid: &Ellipsoid) -> f64 {
        match self {
            BoundingVolume::Sphere(data) => {
                let center = DVec3::new(data[0], data[1], data[2]);
                let radius = data[3];
                (point.distance(center) - radius).max(0.0)
            }
            BoundingVolume::Box(data) => {
                let center = DVec3::new(data[0], data[1], data[2]);
                let half_x = DVec3::new(data[3], data[4], data[5]);
                let half_y = DVec3::new(data[6], data[7], data[8]);
                let half_z = DVec3::new(data[9], data[10], data[11]);

                // Transform point to box-local coordinates
                let offset = point - center;
                let dx = offset.dot(half_x.normalize_or_zero());
                let dy = offset.dot(half_y.normalize_or_zero());
                let dz = offset.dot(half_z.normalize_or_zero());

                let ex = (dx.abs() - half_x.length()).max(0.0);
                let ey = (dy.abs() - half_y.length()).max(0.0);
                let ez = (dz.abs() - half_z.length()).max(0.0);

                (ex * ex + ey * ey + ez * ez).sqrt()
            }
            BoundingVolume::Region(_) => {
                // Use bounding sphere approximation for region
                let sphere = self.to_bounding_sphere(ellipsoid);
                (point.distance(sphere.center) - sphere.radius).max(0.0)
            }
        }
    }

    /// Gets the geographic rectangle if this is a region volume.
    pub fn as_region(&self) -> Option<Rectangle> {
        match self {
            BoundingVolume::Region(data) => {
                Some(Rectangle::new(data[0], data[1], data[2], data[3]))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounding_sphere_creation() {
        let bv = BoundingVolume::from_sphere(DVec3::new(1.0, 2.0, 3.0), 10.0);
        assert_eq!(bv, BoundingVolume::Sphere([1.0, 2.0, 3.0, 10.0]));
    }

    #[test]
    fn test_bounding_box_creation() {
        let bv = BoundingVolume::from_box(
            DVec3::ZERO,
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(0.0, 1.0, 0.0),
            DVec3::new(0.0, 0.0, 1.0),
        );
        if let BoundingVolume::Box(data) = bv {
            assert_eq!(data[0], 0.0); // center x
            assert_eq!(data[3], 1.0); // half_x x
        } else {
            panic!("Expected Box variant");
        }
    }

    #[test]
    fn test_sphere_center() {
        let bv = BoundingVolume::from_sphere(DVec3::new(100.0, 200.0, 300.0), 50.0);
        let center = bv.center(&Ellipsoid::WGS84);
        assert!((center.x - 100.0).abs() < 1e-10);
        assert!((center.y - 200.0).abs() < 1e-10);
        assert!((center.z - 300.0).abs() < 1e-10);
    }

    #[test]
    fn test_sphere_distance() {
        let bv = BoundingVolume::from_sphere(DVec3::ZERO, 10.0);
        let point = DVec3::new(20.0, 0.0, 0.0);
        let dist = bv.distance_to(point, &Ellipsoid::WGS84);
        assert!((dist - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_sphere_distance_inside() {
        let bv = BoundingVolume::from_sphere(DVec3::ZERO, 10.0);
        let point = DVec3::new(5.0, 0.0, 0.0);
        let dist = bv.distance_to(point, &Ellipsoid::WGS84);
        assert!((dist - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_box_distance() {
        let bv = BoundingVolume::from_box(
            DVec3::ZERO,
            DVec3::new(5.0, 0.0, 0.0),
            DVec3::new(0.0, 5.0, 0.0),
            DVec3::new(0.0, 0.0, 5.0),
        );
        // Point outside on X axis
        let point = DVec3::new(10.0, 0.0, 0.0);
        let dist = bv.distance_to(point, &Ellipsoid::WGS84);
        assert!((dist - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_to_bounding_sphere_from_box() {
        let bv = BoundingVolume::from_box(
            DVec3::ZERO,
            DVec3::new(3.0, 0.0, 0.0),
            DVec3::new(0.0, 4.0, 0.0),
            DVec3::new(0.0, 0.0, 0.0),
        );
        let sphere = bv.to_bounding_sphere(&Ellipsoid::WGS84);
        assert!((sphere.radius - 5.0).abs() < 1e-10); // sqrt(9 + 16) = 5
    }

    #[test]
    fn test_region_as_rectangle() {
        let bv = BoundingVolume::from_region(-1.0, -0.5, 1.0, 0.5, 0.0, 100.0);
        let rect = bv.as_region().unwrap();
        assert!((rect.west - (-1.0)).abs() < 1e-10);
        assert!((rect.east - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_serde_roundtrip() {
        let bv = BoundingVolume::from_sphere(DVec3::new(1.0, 2.0, 3.0), 10.0);
        let json = serde_json::to_string(&bv).unwrap();
        let parsed: BoundingVolume = serde_json::from_str(&json).unwrap();
        assert_eq!(bv, parsed);
    }
}
