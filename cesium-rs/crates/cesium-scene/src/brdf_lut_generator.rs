//! Ported from `packages/engine/Source/Scene/BrdfLutGenerator.js`.

/// BRDF LUT generator.
///
/// Generates the BRDF lookup table for PBR image-based lighting.
pub struct BrdfLutGenerator {
    /// Whether generation is complete.
    pub complete: bool,
}

impl BrdfLutGenerator {
    /// Creates a new BrdfLutGenerator.
    pub fn new() -> Self { Self { complete: false } }
}

impl Default for BrdfLutGenerator {
    fn default() -> Self { Self::new() }
}
