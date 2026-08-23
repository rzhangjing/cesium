//! Ported from `packages/engine/Source/Scene/Picking.js`.
//!
//! Picking utilities for the scene.

use cesium_core::cartesian2::Cartesian2;

use crate::frame_state::FrameState;

/// Picking utilities for identifying objects in the scene.
///
/// Provides functions for picking primitives, features, and globe positions
/// from screen coordinates.
/// Mirrors CesiumJS `Picking` (630 lines).
pub struct Picking {
    _private: (),
}

/// The result of a pick operation.
pub struct PickedObject {
    /// The picked primitive or object.
    pub primitive: Option<String>,
    /// The picked feature index.
    pub feature_index: Option<i32>,
    /// The screen position of the pick.
    pub position: Cartesian2,
}

impl Picking {
    /// Creates a new Picking.
    pub fn new() -> Self { Self { _private: () } }

    /// Picks the topmost object at the given window position.
    pub fn pick(&self, _frame_state: &FrameState, _window_position: &Cartesian2) -> Option<PickedObject> {
        // DEVIATION: Requires readback from pick framebuffer
        None
    }

    /// Picks all objects at the given window position.
    pub fn pick_all(&self, _frame_state: &FrameState, _window_position: &Cartesian2) -> Vec<PickedObject> {
        // DEVIATION: Requires readback from pick framebuffer
        Vec::new()
    }

    /// Picks the globe position at the given window position.
    pub fn pick_globe(&self, _frame_state: &FrameState, _window_position: &Cartesian2) -> Option<cesium_core::cartesian3::Cartesian3> {
        // DEVIATION: Requires ray-ellipsoid intersection
        None
    }
}

impl Default for Picking {
    fn default() -> Self { Self::new() }
}
