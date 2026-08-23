//! Ported from `packages/engine/Source/Scene/CloudType.js`.

/// Type of cloud.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CloudType {
    /// Cumulus cloud.
    Cumulus = 0,
}
