//! Ported from `packages/engine/Source/Core/TrackingReferenceFrame.js`.

/// Constants for identifying well-known tracking reference frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TrackingReferenceFrame {
    /// Auto-detect algorithm.
    Autodetect = 0,
    /// The entity's local East-North-Up reference frame.
    Enu = 1,
    /// The entity's inertial reference frame.
    Inertial = 2,
    /// The entity's inertial reference frame with orientation fixed to its
    /// VelocityOrientationProperty.
    Velocity = 3,
}
