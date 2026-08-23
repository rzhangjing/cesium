//! Ported from `packages/engine/Source/Scene/IonImageryProviderFactory.js`.

/// Ion imagery provider factory.
pub struct IonImageryProviderFactory {
    _private: (),
}

impl IonImageryProviderFactory {
    /// Creates a new IonImageryProviderFactory.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for IonImageryProviderFactory {
    fn default() -> Self { Self::new() }
}
