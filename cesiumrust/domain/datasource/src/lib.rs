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
pub mod property_system;
pub mod entity;
pub mod entity_collection;
pub mod geojson;
pub mod czml;
pub mod geometry_updater;
pub mod visualizer;
pub mod primitives;
pub mod datasource_display;
pub mod cluster;
pub mod animation;
pub mod property_bag;
pub mod datasource_collection;
pub mod composite_entity_collection;
pub mod velocity_vector_property;
pub mod velocity_orientation_property;
pub mod node_transformation_property;
pub mod datasource_clock;
pub mod custom_data_source;
pub mod property_array;

pub use property::{Color, Property, PositionProperty, ColorProperty, NumberProperty, BoolProperty, StringProperty};
pub use entity::{
    Entity, PointGraphics, PolylineGraphics, PolygonGraphics,
    BillboardGraphics, LabelGraphics, ModelGraphics, EllipseGraphics,
    BoxGraphics, CylinderGraphics, CorridorGraphics, RectangleGraphics,
    WallGraphics, EllipsoidGraphics, PlaneGraphics, PathGraphics,
    PolylineVolumeGraphics, HeightReference, CornerType, ClassificationType,
    ShadowMode, PlaneDef,
};
pub use entity_collection::{EntityCollection, DataSource};
pub use geojson::{parse_geojson, GeoJsonOptions, GeoJsonError};
pub use czml::{parse_czml, CzmlError};
pub use property_bag::PropertyBag;
pub use datasource_collection::DataSourceCollection;
pub use composite_entity_collection::CompositeEntityCollection;
pub use velocity_vector_property::VelocityVectorProperty;
pub use velocity_orientation_property::VelocityOrientationProperty;
pub use node_transformation_property::{NodeTransformationProperty, NodeTransformationValue};
pub use datasource_clock::DataSourceClock;
pub use custom_data_source::CustomDataSource;
pub use property_array::{PropertyArray, PositionPropertyArray};
