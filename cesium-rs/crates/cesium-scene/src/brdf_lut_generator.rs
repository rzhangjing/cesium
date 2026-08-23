//! Ported from `packages/engine/Source/Scene/BRDFLutGenerator.js`.

/// Generates the BRDF lookup table for PBR.
pub struct BrdfLutGenerator {
    _private: (),
}

impl BrdfLutGenerator {
    /// Creates a new BrdfLutGenerator.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for BrdfLutGenerator {
    fn default() -> Self { Self::new() }
}
