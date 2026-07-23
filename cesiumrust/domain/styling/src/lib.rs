//! cesium-styling: 3D Tiles styling and classification.
//!
//! Domain layer - pure Rust, f64 precision.
//!
//! CesiumJS mapping:
//! - `Scene/Cesium3DTileStyle.js` → tile_style
//! - `Scene/Expression.js` → tile_style
//! - `Scene/ClassificationPrimitive.js` → classification
//! - `Scene/ClassificationType.js` → classification

pub mod classification;
pub mod tile_style;

pub use classification::{
    Classification, ClassificationCollection, ClassificationType, FeatureMetadata, MetadataValue,
};
pub use tile_style::{
    ArithmeticOp, CompareOp, PropertyValue, StyleExpression, TileStyle,
};
