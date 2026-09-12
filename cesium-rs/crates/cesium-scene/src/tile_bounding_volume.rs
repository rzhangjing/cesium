//! Ported from `packages/engine/Source/Scene/TileBoundingVolume.js`.
//!
//! A bounding volume for a tile. The CesiumJS runtime has three concrete
//! implementations (`TileOrientedBoundingBox`, `TileBoundingSphere`,
//! `TileBoundingRegion`) behind the `TileBoundingVolume` interface; the Rust
//! port unifies them in a single enum so tiles can own their bounding
//! volumes without dynamic dispatch.

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::cartesian3::Cartesian3;
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

    /// The underlying [`OrientedBoundingBox`], when the volume has one.
    ///
    /// Mirrors `TileBoundingVolume.boundingVolume` (getter):
    /// - box: the box itself
    /// - sphere: none (`TileBoundingSphere.boundingVolume` is `undefined`)
    /// - region: `TileBoundingRegion.boundingVolume`, i.e. the
    ///   `_orientedBoundingBox` that `computeBoundingVolumes` builds
    pub fn bounding_box(&self) -> Option<OrientedBoundingBox> {
        match self {
            Self::Box { center, half_axes } => {
                Some(OrientedBoundingBox::new(Some(center), Some(half_axes)))
            }
            Self::Region {
                rectangle,
                minimum_height,
                maximum_height,
            } => Some(self.region_oriented_bounding_box(rectangle, *minimum_height, *maximum_height)),
            Self::Sphere { .. } => None,
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
    /// The region arm reproduces `TileBoundingRegion.prototype.computeBoundingVolumes`.
    ///
    /// DEVIATION: `computeBoundingVolumes` takes an ellipsoid; this getter has
    /// no way to receive one, so `OrientedBoundingBox.from_rectangle` falls
    /// back to its own default (`Ellipsoid::WGS84`) — the same ellipsoid the
    /// previous approximating implementation hard-coded.
    pub fn bounding_sphere(&self) -> BoundingSphere {
        match self {
            Self::Box { center, half_axes } => {
                let obb = OrientedBoundingBox::new(Some(center), Some(half_axes));
                BoundingSphere::from_oriented_bounding_box(&obb, None)
            }
            Self::Sphere { center, radius } => BoundingSphere::new(*center, *radius),
            Self::Region {
                rectangle,
                minimum_height,
                maximum_height,
            } => {
                let obb =
                    self.region_oriented_bounding_box(rectangle, *minimum_height, *maximum_height);
                BoundingSphere::from_oriented_bounding_box(&obb, None)
            }
        }
    }

    /// `OrientedBoundingBox.fromRectangle(rectangle, minimumHeight,
    /// maximumHeight, ellipsoid)` — the first half of
    /// `TileBoundingRegion.prototype.computeBoundingVolumes`.
    fn region_oriented_bounding_box(
        &self,
        rectangle: &Rectangle,
        minimum_height: f64,
        maximum_height: f64,
    ) -> OrientedBoundingBox {
        OrientedBoundingBox::from_rectangle(
            Some(rectangle),
            Some(minimum_height),
            Some(maximum_height),
            None,
            None,
        )
    }

    /// Gets the distance from the given point to the closest point on this
    /// bounding volume.
    ///
    /// Mirrors `TileBoundingVolume.distanceToCamera(frameState)` with the
    /// camera position passed directly (3D mode only):
    /// - box: `OrientedBoundingBox.distanceSquaredTo`
    /// - sphere: `Cartesian3.distance(center, position)`
    ///   (mirrors `TileBoundingSphere.distanceToCamera`)
    /// - region: distance to the bounding sphere's centre. DEVIATION:
    ///   `TileBoundingRegion.prototype.computeDistanceToCamera` clips against
    ///   the region's four edge planes and the height range instead; that is
    ///   not ported.
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
