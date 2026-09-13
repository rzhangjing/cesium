//! cesium-kml: KML (Keyhole Markup Language) parser and exporter.
//!
//! Domain layer - pure Rust, f64 precision.
//!
//! CesiumJS mapping:
//! - `DataSources/KmlDataSource.js` → parser
//! - `DataSources/KmlTour.js` → tour
//! - `DataSources/exportKml.js` → export

pub mod export;
pub mod parser;
pub mod tour;

pub use export::{
    rgba_to_kml_color, KmlExportGeometry, KmlExportIconStyle, KmlExportLabelStyle,
    KmlExportLineStyle, KmlExportOptions, KmlExportPlacemark, KmlExportPolyStyle,
    KmlExportResult, KmlExportStyle, KmlExporter,
};
pub use parser::{
    KmlCoordinate, KmlDocument, KmlGeometry, KmlIconStyle, KmlLabelStyle, KmlLineStyle,
    KmlPlacemark, KmlPolyStyle, KmlStyle, kml_to_datasource, parse_coordinates,
    parse_kml_color, parse_kml_simple,
};
pub use tour::{FlyToMode, KmlTour, KmlTourEntry, KmlTourFlyTo, KmlTourWait};
