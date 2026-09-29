//! cesium-gpx：GPX（GPS Exchange Format）解析器。
//!
//! 领域层 —— 纯 Rust，f64 精度。
//!
//! CesiumJS 映射：`DataSources/GpxDataSource.js`

pub mod parser;

pub use parser::{
    GpxDocument, GpxMetadata, GpxRoute, GpxRoutePoint, GpxTrack, GpxTrackPoint,
    GpxTrackSegment, GpxWaypoint, gpx_to_datasource, parse_gpx_simple,
};
