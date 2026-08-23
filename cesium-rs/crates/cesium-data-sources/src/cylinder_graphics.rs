//! Ported from `packages/engine/Source/DataSources/CylinderGraphics.js`.

/// Graphics properties for a cylinder or cone.
#[derive(Clone)]
pub struct CylinderGraphics {
    /// Whether this graphics is shown.
    pub show: bool,
}

impl CylinderGraphics {
    /// Creates a new Cylinder graphics.
    pub fn new() -> Self {
        Self { show: true }
    }
}

impl Default for CylinderGraphics {
    fn default() -> Self { Self::new() }
}
