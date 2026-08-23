//! Ported from `packages/engine/Source/Scene/SensorVolumePortionToDisplay.js`.

/// Which portion of a sensor volume to display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SensorVolumePortionToDisplay {
    /// Display the complete sensor volume.
    Complete = 0,
    /// Display only above the ellipsoid horizon.
    AboveEllipsoidHorizonOnly = 1,
    /// Display only below the ellipsoid horizon.
    BelowEllipsoidHorizonOnly = 2,
}
