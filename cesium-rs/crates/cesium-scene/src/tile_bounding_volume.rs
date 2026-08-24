//! Ported from `packages/engine/Source/Scene/TileBoundingVolume.js`.
//!
//! A bounding volume for a tile. The CesiumJS runtime has three concrete
//! implementations (`TileOrientedBoundingBox`, `TileBoundingSphere`,
//! `TileBoundingRegion`) behind the `TileBoundingVolume` interface; the Rust
//! port unifies them in a single enum so tiles can own their bounding
//! volumes without dynamic dispatch.

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartographic::Cartographic;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::matrix3::Matrix3;
use cesium_core::oriented_bounding_box::OrientedBoundingBox;
use cesium_core::rectangle::Rectangle;

/// A bounding volume for a tile.
///
/// Mirrors the CesiumJS `TileBoundingVolume` interface implemented by
/// `TileOrientedBoundingBox` (box), `TileBoundingSphere` (sphere) and
/// `TileBoundingRegion` (region).
#[derive(Debug, Clone, PartialEq)]
pub enum TileBoundingVolume {
    /// An oriented bounding box (`TileOrientedBoundingBox`).
    Box {
        /// The center of the box.
        center: Cartesian3,
        /// The three half-axes of the box.
        half_axes: Matrix3,
    },
    /// A bounding sphere (`TileBoundingSphere`).
    Sphere {
        /// The center of the sphere.
        center: Cartesian3,
        /// The radius of the sphere.
        radius: f64,
    },
    /// A longitude/latitude/height region (`TileBoundingRegion`).
    Region {
        /// The longitude/latitude range of the region.
        rectangle: Rectangle,
        /// The minimum height of the region.
        minimum_height: f64,
        /// The maximum height of the region.
        maximum_height: f64,
    },
}

impl TileBoundingVolume {
    /// Creates an oriented bounding box volume.
    #[must_use]
    pub fn new_box(center: Cartesian3, half_axes: Matrix3) -> Self {
        Self::Box { center, half_axes }
    }

    /// Creates a bounding sphere volume.
    #[must_use]
    pub fn new_sphere(center: Cartesian3, radius: f64) -> Self {
        Self::Sphere { center, radius }
    }

    /// Creates a region volume.
    #[must_use]
    pub fn new_region(rectangle: Rectangle, minimum_height: f64, maximum_height: f64) -> Self {
        Self::Region {
            rectangle,
            minimum_height,
            maximum_height,
        }
    }

    /// The underlying [`OrientedBoundingBox`], when the volume is a box.
    ///
    /// Mirrors `TileBoundingVolume.boundingVolume` (getter).
    #[must_use]
    pub fn bounding_box(&self) -> Option<OrientedBoundingBox> {
        match self {
            Self::Box { center, half_axes } => {
                Some(OrientedBoundingBox::new(Some(center), Some(half_axes)))
            }
            _ => None,
        }
    }

    /// The bounding sphere enclosing this volume.
    ///
    /// Mirrors `TileBoundingVolume.boundingSphere` (getter):
    /// - box: `BoundingSphere.fromOrientedBoundingBox`
    /// - sphere: the volume itself
    /// - region: `BoundingSphere.fromOrientedBoundingBox` of
    ///   `OrientedBoundingBox.fromRectangle`
    ///
    /// DEVIATION: the Core API `OrientedBoundingBox.fromRectangle` is not
    /// yet ported, so region volumes approximate the bounding sphere from
    /// the rectangle corners and center sampled at the maximum height.
    #[must_use]
    pub fn bounding_sphere(&self) -> BoundingSphere {
        match self {
            Self::Box { center, half_axes } => {
                // BoundingSphere.fromOrientedBoundingBox: the radius is the
                // distance from the center to a corner, i.e. the magnitude
                // of the sum of the three half-axis column vectors.
                let u = Matrix3::get_column_new(half_axes, 0);
                let v = Matrix3::get_column_new(half_axes, 1);
                let w = Matrix3::get_column_new(half_axes, 2);
                let corner_offset = Cartesian3::new(
                    u.x + v.x + w.x,
                    u.y + v.y + w.y,
                    u.z + v.z + w.z,
                );
                BoundingSphere::new(
                    *center,
                    Cartesian3::magnitude(&corner_offset),
                )
            }
            Self::Sphere { center, radius } => BoundingSphere::new(*center, *radius),
            Self::Region {
                rectangle,
                maximum_height,
                ..
            } => {
                let ellipsoid = &Ellipsoid::WGS84;
                let samples = [
                    Cartographic::new(rectangle.west, rectangle.south, *maximum_height),
                    Cartographic::new(rectangle.east, rectangle.south, *maximum_height),
                    Cartographic::new(rectangle.west, rectangle.north, *maximum_height),
                    Cartographic::new(rectangle.east, rectangle.north, *maximum_height),
                    Cartographic::new(
                        (rectangle.west + rectangle.east) * 0.5,
                        (rectangle.south + rectangle.north) * 0.5,
                        *maximum_height,
                    ),
                ];
                let mut center = Cartesian3::ZERO;
                let mut points = [Cartesian3::ZERO; 5];
                for (point, cartographic) in points.iter_mut().zip(samples.iter()) {
                    ellipsoid.cartographic_to_cartesian(cartographic, point);
                    center.x += point.x;
                    center.y += point.y;
                    center.z += point.z;
                }
                center.x /= points.len() as f64;
                center.y /= points.len() as f64;
                center.z /= points.len() as f64;

                let mut radius = 0.0_f64;
                for point in &points {
                    radius = radius.max(Cartesian3::distance(&center, point));
                }
                BoundingSphere::new(center, radius)
            }
        }
    }

    /// Gets the distance from the given point to the closest point on this
    /// bounding volume.
    ///
    /// Mirrors `TileBoundingVolume.distanceToCamera(frameState)` with the
    /// camera position passed directly (3D mode only):
    /// - box: `OrientedBoundingBox.distanceSquaredTo`
    /// - sphere: `Cartesian3.distance(center, position)`
    ///   (mirrors `TileBoundingSphere.distanceToCamera`)
    /// - region: distance to the approximated bounding sphere center
    ///   (see the DEVIATION note on [`Self::bounding_sphere`]).
    #[must_use]
    pub fn distance_to_point(&self, point: &Cartesian3) -> f64 {
        match self {
            Self::Box { center, half_axes } => {
                let obb = OrientedBoundingBox::new(Some(center), Some(half_axes));
                OrientedBoundingBox::distance_squared_to(&obb, point).sqrt()
            }
            Self::Sphere { center, .. } => Cartesian3::distance(center, point),
            Self::Region { .. } => {
                let sphere = self.bounding_sphere();
                Cartesian3::distance(&sphere.center, point)
            }
        }
    }
}
