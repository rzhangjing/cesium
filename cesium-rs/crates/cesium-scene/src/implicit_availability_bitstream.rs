//! Ported from `packages/engine/Source/Scene/ImplicitAvailabilityBitstream.js`.

/// Implicit availability bitstream.
///
/// Tracks tile availability in an implicit tileset.
pub struct ImplicitAvailabilityBitstream {
    /// The number of available tiles.
    pub available_count: u32,
    /// The total number of tiles.
    pub total_count: u32,
}

impl ImplicitAvailabilityBitstream {
    /// Creates a new ImplicitAvailabilityBitstream.
    pub fn new() -> Self { Self { available_count: 0, total_count: 0 } }
}

impl Default for ImplicitAvailabilityBitstream {
    fn default() -> Self { Self::new() }
}
