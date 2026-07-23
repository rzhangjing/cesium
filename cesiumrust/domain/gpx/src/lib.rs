//! cesium-gpx: GPX (GPS Exchange Format) parser.
//!
//! Domain layer - pure Rust, f64 precision.
//!
//! CesiumJS mapping: `DataSources/GpxDataSource.js`

pub mod parser;

pub use parser::{
    GpxDocument, GpxMetadata, GpxRoute, GpxRoutePoint, GpxTrack, GpxTrackPoint,
    GpxTrackSegment, GpxWaypoint, gpx_to_datasource, parse_gpx_simple,
};
