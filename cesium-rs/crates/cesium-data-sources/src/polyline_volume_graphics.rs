//! Ported from `packages/engine/Source/DataSources/PolylineVolumeGraphics.js`.

/// Graphics properties for a polyline volume.
#[derive(Clone)]
pub struct PolylineVolumeGraphics {
    /// Whether this graphics is shown.
    pub show: bool,
}

impl PolylineVolumeGraphics {
    /// Creates a new PolylineVolume graphics.
    pub fn new() -> Self {
        Self { show: true }
    }
}

impl Default for PolylineVolumeGraphics {
    fn default() -> Self { Self::new() }
}
