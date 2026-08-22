//! Ported from `packages/engine/Source/Core/sampleTerrain.js`.
//!
//! Samples terrain heights for given positions.

/// Samples terrain heights for given cartographic positions.
/// Skeleton: requires terrain provider and network I/O.
pub struct SampleTerrain;

impl SampleTerrain {
    /// Samples terrain heights for the given positions.
    pub fn sample(
        _terrain_provider: &str,
        _level: i32,
        _positions: &mut [crate::cartographic::Cartographic],
    ) -> Result<(), String> {
        // Skeleton: requires terrain provider
        Err("Not implemented".to_string())
    }
}
