//! Ported from `packages/engine/Source/Core/sampleTerrainMostDetailed.js`.
//!
//! Samples terrain heights at the most detailed level available.

/// Samples terrain heights at the most detailed level available.
/// Skeleton: requires terrain provider and network I/O.
pub struct SampleTerrainMostDetailed;

impl SampleTerrainMostDetailed {
    /// Samples terrain at the most detailed level for given positions.
    pub fn sample(
        _terrain_provider: &str,
        _positions: &mut [crate::cartographic::Cartographic],
    ) -> Result<(), String> {
        // Skeleton: requires terrain provider
        Err("Not implemented".to_string())
    }
}
