//! cesium-kml: KML (Keyhole Markup Language) parser.
//!
//! Domain layer - pure Rust, f64 precision.
//!
//! CesiumJS mapping: `DataSources/KmlDataSource.js`

pub mod parser;

pub use parser::{
    KmlCoordinate, KmlDocument, KmlGeometry, KmlIconStyle, KmlLabelStyle, KmlLineStyle,
    KmlPlacemark, KmlPolyStyle, KmlStyle, kml_to_datasource, parse_coordinates,
    parse_kml_color, parse_kml_simple,
};
