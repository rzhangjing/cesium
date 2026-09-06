//! Ported from `packages/engine/Source/Scene/IonImageryProviderFactory.js`.

/// Ion imagery provider factory.
///
/// Creates imagery providers from Cesium Ion asset endpoints.
pub struct IonImageryProviderFactory {
    /// Supported provider type identifiers.
    pub supported_types: Vec<String>,
}

impl IonImageryProviderFactory {
    /// Creates a new IonImageryProviderFactory.
    pub fn new() -> Self { Self { supported_types: Vec::new() } }
}

impl Default for IonImageryProviderFactory {
    fn default() -> Self { Self::new() }
}
