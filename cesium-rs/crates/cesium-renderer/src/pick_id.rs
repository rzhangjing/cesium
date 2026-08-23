//! Ported from `packages/engine/Source/Renderer/PickId.js`.

use cesium_core::color::Color;

/// An identifier used for picking operations.
///
/// Each pick ID has a unique color that can be used to identify
/// the object during picking.
pub struct PickId {
    /// The unique object identifier.
    object_index: u32,
    /// The color used for picking.
    color: Color,
}

impl PickId {
    /// Creates a new pick ID.
    pub fn new(object_index: u32, color: Color) -> Self {
        Self { object_index, color }
    }

    /// Returns the object index.
    pub fn object_index(&self) -> u32 { self.object_index }

    /// Returns the pick color.
    pub fn color(&self) -> &Color { &self.color }
}
