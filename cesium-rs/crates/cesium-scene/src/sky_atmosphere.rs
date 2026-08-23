//! Ported from `packages/engine/Source/Scene/SkyAtmosphere.js`.
//!
//! Draws the sky atmosphere (blue sky, sunset colors) around the globe.

use cesium_core::cartesian3::Cartesian3;
use crate::frame_state::FrameState;

/// Draws the sky atmosphere (blue sky, sunset colors) around the globe.
///
/// Uses atmospheric scattering equations (Rayleigh + Mie) to render
/// a realistic sky dome around the ellipsoid.
pub struct SkyAtmosphere {
    /// Whether the atmosphere is shown.
    pub show: bool,
    /// The hue shift applied to the atmosphere.
    pub hue_shift: f64,
    /// The saturation shift applied to the atmosphere.
    pub saturation_shift: f64,
    /// The brightness shift applied to the atmosphere.
    pub brightness_shift: f64,
    /// Whether this has been destroyed.
    is_destroyed: bool,
}

impl SkyAtmosphere {
    /// Creates a new SkyAtmosphere.
    pub fn new() -> Self {
        Self {
            show: true,
            hue_shift: 0.0,
            saturation_shift: 0.0,
            brightness_shift: 0.0,
            is_destroyed: false,
        }
    }

    /// Updates the atmosphere for the current frame.
    pub fn update(&mut self, _frame_state: &FrameState) {
        if !self.show { return; }
        // In full port: compute scattering parameters, issue draw commands
    }

    /// Returns true if this object was destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }
    /// Destroys the WebGL resources held by this object.
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for SkyAtmosphere {
    fn default() -> Self { Self::new() }
}
