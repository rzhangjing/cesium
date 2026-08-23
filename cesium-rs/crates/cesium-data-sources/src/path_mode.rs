//! Ported from `packages/engine/Source/DataSources/PathMode.js`.

/// The mode for computing the path of an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PathMode {
    /// The path is computed in the fixed frame.
    Fixed = 0,
    /// The path is computed in the inertial frame.
    Inertial = 1,
    /// The path is computed in the velocity-oriented frame.
    VelocityOrientation = 2,
}
