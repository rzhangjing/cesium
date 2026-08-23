//! Ported from `packages/engine/Source/Scene/Atmosphere.js`.

/// Atmospheric scattering parameters for the sky and horizon.
pub struct Atmosphere {
    /// Whether atmospheric effects are shown.
    pub show: bool,
    /// The intensity of the atmospheric scattering.
    pub intensity: f64,
}

impl Atmosphere {
    /// Creates a new atmosphere.
    pub fn new() -> Self {
        Self { show: true, intensity: 1.0 }
    }
}

impl Default for Atmosphere {
    fn default() -> Self { Self::new() }
}
