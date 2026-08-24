//! Ported from `packages/engine/Source/DataSources/CallbackProperty.js`.

use cesium_core::event::Event;

use crate::property::{Property, PropertyResult};

/// A property whose value is computed by a callback function.
pub struct CallbackProperty {
    callback: Box<dyn Fn(f64) -> PropertyResult + Send + Sync>,
    is_constant: bool,
    is_destroyed: bool,
    definition_changed: Event<()>,
}

impl CallbackProperty {
    /// Creates a new callback property.
    pub fn new(callback: Box<dyn Fn(f64) -> PropertyResult + Send + Sync>, is_constant: bool) -> Self {
        Self { callback, is_constant, is_destroyed: false, definition_changed: Event::new() }
    }

    /// Sets the callback and whether or not the property is constant.
    ///
    /// Port of `CallbackProperty.prototype.setCallback`: raises
    /// `definitionChanged` whenever the callback is replaced.
    pub fn set_callback(
        &mut self,
        callback: Box<dyn Fn(f64) -> PropertyResult + Send + Sync>,
        is_constant: bool,
    ) {
        self.callback = callback;
        self.is_constant = is_constant;
        self.definition_changed.raise_event(&());
    }

    /// Gets the event that is raised whenever the definition of this
    /// property changes (port of the `definitionChanged` getter).
    pub fn definition_changed_event(&self) -> &Event<()> {
        &self.definition_changed
    }
}

impl Property for CallbackProperty {
    fn get_value(&self, time: f64) -> PropertyResult {
        (self.callback)(time)
    }

    fn is_constant(&self) -> bool { self.is_constant }
    fn is_destroyed(&self) -> bool { self.is_destroyed }

    fn definition_changed(&self) -> Option<&Event<()>> {
        Some(&self.definition_changed)
    }
}
