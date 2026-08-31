//! Ported from `packages/engine/Source/Scene/Cesium3DTilePassState.js`.
//!
//! The state for a 3D Tiles update pass.

use crate::cesium3_d_tile_pass::Cesium3DTilePass;

/// The state for a 3D Tiles update pass (JS `Cesium3DTilePassState`).
///
/// DEVIATION: the JS holds live references to a `DrawCommand[]` command
/// list, a `Camera`, and a `CullingVolume` to override the frame state for
/// the current pass. The Rust port records only whether each override is
/// present (the JS `defined(options.commandList)` checks); the referenced
/// render objects live in the owning traversal's frame state plumbing and
/// are not duplicated here.
#[derive(Debug, Clone)]
pub struct Cesium3DTilePassState {
    /// The pass (JS `options.pass`).
    pub pass: Cesium3DTilePass,
    /// Whether a command list override was supplied (JS `commandList`).
    pub has_command_list: bool,
    /// Whether a camera override was supplied (JS `camera`).
    pub has_camera: bool,
    /// Whether a culling volume override was supplied (JS `cullingVolume`).
    pub has_culling_volume: bool,
    /// Whether the pass is ready, i.e. all tiles needed by the pass are
    /// loaded (JS `ready`, default `false`).
    pub ready: bool,
}

/// Construction options (the JS `options` object).
#[derive(Debug, Clone)]
pub struct Cesium3DTilePassStateOptions {
    /// The pass (required in JS: `Check.typeOf.number("options.pass", ...)`).
    pub pass: Cesium3DTilePass,
    /// Whether a command list override is supplied.
    pub has_command_list: bool,
    /// Whether a camera override is supplied.
    pub has_camera: bool,
    /// Whether a culling volume override is supplied.
    pub has_culling_volume: bool,
}

impl Cesium3DTilePassState {
    /// Creates a new pass state from options.
    ///
    /// JS `Cesium3DTilePassState(options)`.
    pub fn new(options: &Cesium3DTilePassStateOptions) -> Self {
        Self {
            pass: options.pass,
            has_command_list: options.has_command_list,
            has_camera: options.has_camera,
            has_culling_volume: options.has_culling_volume,
            ready: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_copies_options_and_defaults_ready_false() {
        let state = Cesium3DTilePassState::new(&Cesium3DTilePassStateOptions {
            pass: Cesium3DTilePass::Pick,
            has_command_list: true,
            has_camera: false,
            has_culling_volume: true,
        });
        assert_eq!(state.pass, Cesium3DTilePass::Pick);
        assert!(state.has_command_list);
        assert!(!state.has_camera);
        assert!(state.has_culling_volume);
        assert!(!state.ready);
    }
}
