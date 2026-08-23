//! Ported from `packages/engine/Source/Core/IonResource.js`.

/// A resource from Cesium ion.
pub struct IonResource {
    _private: (),
}

impl IonResource {
    /// Creates a new IonResource.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for IonResource {
    fn default() -> Self { Self::new() }
}
