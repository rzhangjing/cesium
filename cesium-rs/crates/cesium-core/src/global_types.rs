//! Ported from `packages/engine/Source/Core/globalTypes.js`.
//!
//! Type aliases corresponding to CesiumJS global type definitions.
//! This module provides type-only exports; no runtime values.

/// Union of all numeric typed array types (Rust equivalent: Vec<f64> etc.).
/// In Rust we use `Vec<f64>` as the default numeric array.
pub type TypedArray = Vec<f64>;

/// Trait for objects that can be destroyed / cleaned up.
pub trait Destroyable {
    /// Releases resources held by this object.
    fn destroy(&mut self);
}

/// A GeoJSON position expressed as `[longitude, latitude]` or
/// `[longitude, latitude, altitude]`.
pub type GeoJsonPosition = Vec<f64>;
