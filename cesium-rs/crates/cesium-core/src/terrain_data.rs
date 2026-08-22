//! Ported from `packages/engine/Source/Core/TerrainData.js`.

use crate::rectangle::Rectangle;

/// Terrain data for a single tile. This trait describes the interface
/// and is not intended to be instantiated directly.
pub trait TerrainData {
    /// Computes the terrain height at a specified longitude and latitude.
    fn interpolate_height(&self, rectangle: &Rectangle, longitude: f64, latitude: f64) -> f64;

    /// Determines if a given child tile is available.
    fn is_child_available(&self, this_x: i32, this_y: i32, child_x: i32, child_y: i32) -> bool;

    /// Gets a value indicating whether this terrain data was created by upsampling.
    fn was_created_by_upsampling(&self) -> bool;

    /// The maximum number of asynchronous tasks used for terrain processing.
    const MAXIMUM_ASYNCHRONOUS_TASKS: usize = 5;
}
