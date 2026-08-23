//! Ported from `packages/engine/Source/Scene/ImplicitAvailabilityBitstream.js`.

/// Implicit availability bitstream.
pub struct ImplicitAvailabilityBitstream {
    _private: (),
}

impl ImplicitAvailabilityBitstream {
    /// Creates a new ImplicitAvailabilityBitstream.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ImplicitAvailabilityBitstream {
    fn default() -> Self { Self::new() }
}
