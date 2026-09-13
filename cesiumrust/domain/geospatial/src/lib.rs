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
pub mod polygon_pipeline;
pub mod ellipsoid_rhumb_line;
pub mod ellipsoid_tangent_plane;
pub mod ellipsoidal_occluder;
pub mod attribute_compression;
pub mod encoded_cartesian3;
pub mod color;
pub mod tipsify;
pub mod array_utils;
pub mod polynomial;
pub mod morton_hilbert;
pub mod occluder;
pub mod s2cell;
pub mod utilities;
pub mod spherical;
pub mod stereographic;
pub mod heap;
pub mod managed_array;
pub mod vertical_exaggeration;
pub mod queue;
pub mod wireframe;
pub mod double_ended_priority_queue;
pub mod associative_array;
pub mod doubly_linked_list;
pub mod polygon_geometry_library;
pub mod geometry_instance_attribute;
pub mod cartesian3_ext;
pub mod matrix4_ext;
pub mod quaternion_ext;
pub mod cartesian2_ext;
pub mod cartesian4_ext;
pub mod matrix3_ext;
pub mod matrix2_ext;
pub mod simon1994_planetary_positions;
pub mod iau_orientation;
pub mod uri_utils;
pub mod simple_polyline_geometry;

pub use cartographic::Cartographic;
pub use ellipsoid::Ellipsoid;
pub use rectangle::Rectangle;
pub use projection::{GeographicProjection, MapProjection, WebMercatorProjection};
pub use tiling_scheme::TilingScheme;
pub use bounding::{AxisAlignedBoundingBox, BoundingRectangle, BoundingSphere, OrientedBoundingBox};
pub use ray::{ray_ellipsoid, Intersect, Plane, Ray};
pub use frustum::{Cullable, CullingVolume, OrthographicFrustum, PerspectiveFrustum};
pub use transforms::{HeadingPitchRoll, HeadingPitchRange, TranslationRotationScale};
pub use geometry::{GeometryData, VertexFormat};
pub use attribute_compression::{ComponentDatatype, IndexDatatype};
pub use geodesic::EllipsoidGeodesic;
