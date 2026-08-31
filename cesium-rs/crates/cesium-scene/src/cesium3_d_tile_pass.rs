//! Ported from `packages/engine/Source/Scene/Cesium3DTilePass.js`.
//!
//! The pass in which a 3D Tileset is updated, together with the frozen
//! per-pass options table (JS `passOptions`).

/// The pass in which a 3D Tileset is updated (JS `Cesium3DTilePass`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Cesium3DTilePass {
    /// Render pass (JS `RENDER = 0`).
    Render = 0,
    /// Pick pass (JS `PICK = 1`).
    Pick = 1,
    /// Shadow pass (JS `SHADOW = 2`).
    Shadow = 2,
    /// Preload pass (JS `PRELOAD = 3`).
    Preload = 3,
    /// Preload flight pass (JS `PRELOAD_FLIGHT = 4`).
    PreloadFlight = 4,
    /// Request render mode defer check (JS `REQUEST_RENDER_MODE_DEFER_CHECK = 5`).
    RequestRenderModeDeferCheck = 5,
    /// Most detailed preload (JS `MOST_DETAILED_PRELOAD = 6`).
    MostDetailedPreload = 6,
    /// Most detailed pick (JS `MOST_DETAILED_PICK = 7`).
    MostDetailedPick = 7,
}

impl Cesium3DTilePass {
    /// The number of passes (JS `NUMBER_OF_PASSES = 8`).
    pub const NUMBER_OF_PASSES: usize = 8;

    /// Converts a raw pass index; returns `None` for out-of-range values.
    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Cesium3DTilePass::Render),
            1 => Some(Cesium3DTilePass::Pick),
            2 => Some(Cesium3DTilePass::Shadow),
            3 => Some(Cesium3DTilePass::Preload),
            4 => Some(Cesium3DTilePass::PreloadFlight),
            5 => Some(Cesium3DTilePass::RequestRenderModeDeferCheck),
            6 => Some(Cesium3DTilePass::MostDetailedPreload),
            7 => Some(Cesium3DTilePass::MostDetailedPick),
            _ => None,
        }
    }
}

/// The frozen options for a 3D Tiles update pass (JS `passOptions[pass]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassOptions {
    /// The pass these options belong to.
    pub pass: Cesium3DTilePass,
    /// Whether the pass renders (JS `isRender`).
    pub is_render: bool,
    /// Whether the pass requests tiles (JS `requestTiles`).
    pub request_tiles: bool,
    /// Whether the pass ignores commands (JS `ignoreCommands`).
    pub ignore_commands: bool,
}

const RENDER_OPTIONS: PassOptions = PassOptions {
    pass: Cesium3DTilePass::Render,
    is_render: true,
    request_tiles: true,
    ignore_commands: false,
};
const PICK_OPTIONS: PassOptions = PassOptions {
    pass: Cesium3DTilePass::Pick,
    is_render: false,
    request_tiles: false,
    ignore_commands: false,
};
const SHADOW_OPTIONS: PassOptions = PassOptions {
    pass: Cesium3DTilePass::Shadow,
    is_render: false,
    request_tiles: true,
    ignore_commands: false,
};
const PRELOAD_OPTIONS: PassOptions = PassOptions {
    pass: Cesium3DTilePass::Preload,
    is_render: false,
    request_tiles: true,
    ignore_commands: true,
};
const PRELOAD_FLIGHT_OPTIONS: PassOptions = PassOptions {
    pass: Cesium3DTilePass::PreloadFlight,
    is_render: false,
    request_tiles: true,
    ignore_commands: true,
};
const REQUEST_RENDER_MODE_DEFER_CHECK_OPTIONS: PassOptions = PassOptions {
    pass: Cesium3DTilePass::RequestRenderModeDeferCheck,
    is_render: false,
    request_tiles: true,
    ignore_commands: true,
};
const MOST_DETAILED_PRELOAD_OPTIONS: PassOptions = PassOptions {
    pass: Cesium3DTilePass::MostDetailedPreload,
    is_render: false,
    request_tiles: true,
    ignore_commands: true,
};
const MOST_DETAILED_PICK_OPTIONS: PassOptions = PassOptions {
    pass: Cesium3DTilePass::MostDetailedPick,
    is_render: false,
    request_tiles: false,
    ignore_commands: false,
};

/// Returns the frozen options for a pass.
///
/// JS `Cesium3DTilePass.getPassOptions`.
///
/// DEVIATION: the JS returns `undefined` for out-of-range indices (the
/// table has exactly `NUMBER_OF_PASSES` slots); the Rust API takes a
/// `Cesium3DTilePass` and therefore cannot be out of range.
pub fn get_pass_options(pass: Cesium3DTilePass) -> &'static PassOptions {
    match pass {
        Cesium3DTilePass::Render => &RENDER_OPTIONS,
        Cesium3DTilePass::Pick => &PICK_OPTIONS,
        Cesium3DTilePass::Shadow => &SHADOW_OPTIONS,
        Cesium3DTilePass::Preload => &PRELOAD_OPTIONS,
        Cesium3DTilePass::PreloadFlight => &PRELOAD_FLIGHT_OPTIONS,
        Cesium3DTilePass::RequestRenderModeDeferCheck => {
            &REQUEST_RENDER_MODE_DEFER_CHECK_OPTIONS
        }
        Cesium3DTilePass::MostDetailedPreload => &MOST_DETAILED_PRELOAD_OPTIONS,
        Cesium3DTilePass::MostDetailedPick => &MOST_DETAILED_PICK_OPTIONS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pass_options_table_matches_js() {
        let options = get_pass_options(Cesium3DTilePass::Render);
        assert!(options.is_render);
        assert!(options.request_tiles);
        assert!(!options.ignore_commands);

        let options = get_pass_options(Cesium3DTilePass::Pick);
        assert!(!options.is_render);
        assert!(!options.request_tiles);
        assert!(!options.ignore_commands);

        let options = get_pass_options(Cesium3DTilePass::Shadow);
        assert!(!options.is_render);
        assert!(options.request_tiles);
        assert!(!options.ignore_commands);

        // Preload-like passes all ignore commands and request tiles.
        for pass in [
            Cesium3DTilePass::Preload,
            Cesium3DTilePass::PreloadFlight,
            Cesium3DTilePass::RequestRenderModeDeferCheck,
            Cesium3DTilePass::MostDetailedPreload,
        ] {
            let options = get_pass_options(pass);
            assert_eq!(options.pass, pass);
            assert!(!options.is_render);
            assert!(options.request_tiles);
            assert!(options.ignore_commands);
        }

        let options = get_pass_options(Cesium3DTilePass::MostDetailedPick);
        assert!(!options.is_render);
        assert!(!options.request_tiles);
        assert!(!options.ignore_commands);
    }

    #[test]
    fn pass_indices_match_js_enum_values() {
        assert_eq!(Cesium3DTilePass::Render as usize, 0);
        assert_eq!(Cesium3DTilePass::Pick as usize, 1);
        assert_eq!(Cesium3DTilePass::Shadow as usize, 2);
        assert_eq!(Cesium3DTilePass::Preload as usize, 3);
        assert_eq!(Cesium3DTilePass::PreloadFlight as usize, 4);
        assert_eq!(Cesium3DTilePass::RequestRenderModeDeferCheck as usize, 5);
        assert_eq!(Cesium3DTilePass::MostDetailedPreload as usize, 6);
        assert_eq!(Cesium3DTilePass::MostDetailedPick as usize, 7);
        assert_eq!(Cesium3DTilePass::NUMBER_OF_PASSES, 8);
        assert_eq!(Cesium3DTilePass::from_index(8), None);
        assert_eq!(Cesium3DTilePass::from_index(3), Some(Cesium3DTilePass::Preload));
    }
}
