//! Core geometry batch: one-to-one Rust mirrors of the CesiumJS Jasmine
//! specs for the Track A2 geometry pipeline / library modules.
//!
//! This is a *new* top-level test entry; the `tests/core.rs` aggregator is
//! intentionally left untouched.
//!
//! Mirrors:
//! - `packages/engine/Specs/Core/PolylinePipelineSpec.js`
//!     -> `core_geometry/polyline_pipeline_spec.rs`
//! - `packages/engine/Specs/Core/CorridorGeometryLibrarySpec.js`
//!     -> `core_geometry/corridor_geometry_library_spec.rs`
//! - `packages/engine/Specs/Core/PolylineVolumeGeometryLibrarySpec.js`
//!     -> `core_geometry/polyline_volume_geometry_library_spec.rs`
//! - `packages/engine/Specs/Core/PolygonPipelineSpec.js`
//!     -> `core_geometry/polygon_pipeline_spec.rs`
//! - `packages/engine/Specs/Core/PolygonOutlineGeometrySpec.js`
//!     -> `core_geometry/polygon_outline_geometry_spec.rs`
//! - `packages/engine/Specs/Core/PolygonGeometrySpec.js`
//!     -> `core_geometry/polygon_geometry_spec.rs`
//! - `packages/engine/Specs/Core/PolylineVolumeGeometrySpec.js`
//!     -> `core_geometry/polyline_volume_geometry_spec.rs`
//! - `packages/engine/Specs/Core/CorridorGeometrySpec.js`
//!     -> `core_geometry/corridor_geometry_spec.rs`
//! - `packages/engine/Specs/Core/EllipseGeometrySpec.js`
//!     -> `core_geometry/ellipse_geometry_spec.rs`
//! - `packages/engine/Specs/Core/CorridorOutlineGeometrySpec.js`
//!     -> `core_geometry/corridor_outline_geometry_spec.rs`
//! - `packages/engine/Specs/Core/PolylineVolumeOutlineGeometrySpec.js`
//!     -> `core_geometry/polyline_volume_outline_geometry_spec.rs`
//! - `packages/engine/Specs/Core/EllipseOutlineGeometrySpec.js`
//!     -> `core_geometry/ellipse_outline_geometry_spec.rs`
//! - `packages/engine/Specs/Core/RectangleGeometrySpec.js`
//!     -> `core_geometry/rectangle_geometry_spec.rs`
//! - `packages/engine/Specs/Core/GroundPolylineGeometrySpec.js`
//!     -> `core_geometry/ground_polyline_geometry_spec.rs`

#[path = "core_geometry/polyline_pipeline_spec.rs"]
mod polyline_pipeline_spec;
#[path = "core_geometry/corridor_geometry_library_spec.rs"]
mod corridor_geometry_library_spec;
#[path = "core_geometry/polyline_volume_geometry_library_spec.rs"]
mod polyline_volume_geometry_library_spec;
#[path = "core_geometry/polygon_pipeline_spec.rs"]
mod polygon_pipeline_spec;
#[path = "core_geometry/polygon_outline_geometry_spec.rs"]
mod polygon_outline_geometry_spec;
#[path = "core_geometry/polygon_geometry_spec.rs"]
mod polygon_geometry_spec;
#[path = "core_geometry/polyline_volume_geometry_spec.rs"]
mod polyline_volume_geometry_spec;
#[path = "core_geometry/corridor_geometry_spec.rs"]
mod corridor_geometry_spec;
#[path = "core_geometry/ellipse_geometry_spec.rs"]
mod ellipse_geometry_spec;
#[path = "core_geometry/corridor_outline_geometry_spec.rs"]
mod corridor_outline_geometry_spec;
#[path = "core_geometry/polyline_volume_outline_geometry_spec.rs"]
mod polyline_volume_outline_geometry_spec;
#[path = "core_geometry/ellipse_outline_geometry_spec.rs"]
mod ellipse_outline_geometry_spec;
#[path = "core_geometry/cz01_pack_unpack_spec.rs"]
mod cz01_pack_unpack_spec;
#[path = "core_geometry/rectangle_geometry_spec.rs"]
mod rectangle_geometry_spec;
#[path = "core_geometry/ground_polyline_geometry_spec.rs"]
mod ground_polyline_geometry_spec;
