//! Ported from `packages/engine/Source/DataSources/CallbackProperty.js`.

use crate::property::{Property, PropertyResult};

/// A property whose value is computed by a callback function.
pub struct CallbackProperty {
    callback: Box<dyn Fn(f64) -> PropertyResult + Send + Sync>,
    is_constant: bool,
    is_destroyed: bool,
}

impl CallbackProperty {
    /// Creates a new callback property.
    pub fn new(callback: Box<dyn Fn(f64) -> PropertyResult + Send + Sync>, is_constant: bool) -> Self {
        Self { callback, is_constant, is_destroyed: false }
    }
}

impl Property for CallbackProperty {
    fn get_value(&self, time: f64) -> PropertyResult {
        (self.callback)(time)
    }

    fn is_constant(&self) -> bool { self.is_constant }
    fn is_destroyed(&self) -> bool { self.is_destroyed }
}
