//! Ported from `packages/engine/Source/Core/IonGeocodeProviderType.js`.

/// Underlying geocoding services that can be used via Cesium ion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IonGeocodeProviderType {
    /// Google geocoder, for use with Google data.
    Google,
    /// Bing geocoder, for use with Bing data.
    Bing,
    /// Use the default geocoder as set on the server.
    Default,
}

impl IonGeocodeProviderType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Google => "GOOGLE",
            Self::Bing => "BING",
            Self::Default => "DEFAULT",
        }
    }
}
