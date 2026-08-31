//! Ported from `packages/engine/Source/DataSources/KmlCamera.js`.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::heading_pitch_roll::HeadingPitchRoll;

/// Representation of `<Camera>` from KML (mirror of `KmlCamera`).
#[derive(Clone, Debug)]
pub struct KmlCamera {
    /// The camera position.
    pub position: Cartesian3,
    /// The camera orientation.
    pub heading_pitch_roll: HeadingPitchRoll,
}

impl KmlCamera {
    /// Creates a new KML camera.
    pub fn new(position: Cartesian3, heading_pitch_roll: HeadingPitchRoll) -> Self {
        Self {
            position,
            heading_pitch_roll,
        }
    }
}
