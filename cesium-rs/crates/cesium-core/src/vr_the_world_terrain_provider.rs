//! Ported from `packages/engine/Source/Core/VRTHEWorldTerrainProvider.js`.

/// A terrain provider using VR-TheWorld terrain.
pub struct VRTHEWorldTerrainProvider {
    _private: (),
}

impl VRTHEWorldTerrainProvider {
    /// Creates a new VRTHEWorldTerrainProvider.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for VRTHEWorldTerrainProvider {
    fn default() -> Self { Self::new() }
}
