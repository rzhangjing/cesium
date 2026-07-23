//! cesium-datasource: Entity and DataSource domain models
//!
//! Maps to CesiumJS:
//! - `DataSources/Entity.js`
//! - `DataSources/EntityCollection.js`
//! - `DataSources/GeoJsonDataSource.js`
//! - `DataSources/CzmlDataSource.js`
//! - `DataSources/Property.js`
//!
//! # Features
//! - Property system (constant, sampled, time-dynamic)
//! - Entity with graphics (point, polyline, polygon, billboard, label, model, ellipse)
//! - EntityCollection management
//! - GeoJSON parsing (RFC 7946)
//! - CZML parsing (basic)

pub mod property;
pub mod entity;
pub mod entity_collection;
pub mod geojson;
pub mod czml;

pub use property::{Color, Property, PositionProperty, ColorProperty, NumberProperty, BoolProperty, StringProperty};
pub use entity::{
    Entity, PointGraphics, PolylineGraphics, PolygonGraphics,
    BillboardGraphics, LabelGraphics, ModelGraphics, EllipseGraphics,
};
pub use entity_collection::{EntityCollection, DataSource};
pub use geojson::{parse_geojson, GeoJsonOptions, GeoJsonError};
pub use czml::{parse_czml, CzmlError};
