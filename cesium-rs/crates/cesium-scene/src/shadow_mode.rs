//! Ported from `packages/engine/Source/Scene/ShadowMode.js`.

/// Whether or not an object casts or receives shadows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ShadowMode {
    /// No shadows.
    Disabled = 0,
    /// Casts and receives shadows.
    Enabled = 1,
    /// Only casts shadows.
    CastOnly = 2,
    /// Only receives shadows.
    ReceiveOnly = 3,
}
