//! cesium-geospatial: Ellipsoid, coordinates, projections, tiling, bounding volumes, geometry
//! Domain layer - pure Rust, f64 precision, no framework dependency.
//!
//! CesiumJS mapping: `packages/engine/Source/Core/` (~180 math/geometry files)

pub mod math_utils;
pub mod cartographic;
pub mod ellipsoid;
pub mod rectangle;
pub mod projection;
pub mod tiling_scheme;
pub mod bounding;
pub mod ray;
pub mod frustum;
pub mod transforms;
pub mod geometry;
pub mod geodesic;
pub mod polyline_pipeline;

pub use cartographic::Cartographic;
pub use ellipsoid::Ellipsoid;
pub use rectangle::Rectangle;
pub use projection::{GeographicProjection, MapProjection, WebMercatorProjection};
pub use tiling_scheme::TilingScheme;
pub use bounding::{AxisAlignedBoundingBox, BoundingSphere, OrientedBoundingBox};
pub use ray::{Intersect, Plane, Ray};
pub use frustum::{CullingVolume, OrthographicFrustum, PerspectiveFrustum};
pub use transforms::{HeadingPitchRoll, HeadingPitchRange, TranslationRotationScale};
pub use geometry::{GeometryData, VertexFormat};
pub use geodesic::EllipsoidGeodesic;
