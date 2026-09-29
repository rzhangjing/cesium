//! cesium-kml：KML（Keyhole Markup Language，可标记语言）解析器与导出器。
//!
//! Domain 层 - 纯 Rust，f64 精度。
//!
//! CesiumJS 映射：
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
