//! Ported from `packages/engine/Source/Core/EventHelper.js`.

/// A convenience object that simplifies the common pattern of attaching event
/// listeners to several events, then removing all those listeners at once.
///
/// DEVIATION: Unlike JS, listeners are tracked by index-based removal from
/// the underlying Event. This struct stores opaque removal tokens.
pub struct EventHelper {
    removal_functions: Vec<Box<dyn FnMut()>>,
}

impl EventHelper {
    pub fn new() -> Self {
        Self {
            removal_functions: Vec::new(),
        }
    }

    /// Registers a removal function to be called when `remove_all` is invoked.
    pub fn add_removal(&mut self, removal: Box<dyn FnMut()>) {
        self.removal_functions.push(removal);
    }

    /// Unregisters all previously added listeners.
    pub fn remove_all(&mut self) {
        for f in &mut self.removal_functions {
            f();
        }
        self.removal_functions.clear();
    }
}

impl Default for EventHelper {
    fn default() -> Self {
        Self::new()
    }
}
