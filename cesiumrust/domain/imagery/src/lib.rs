//! cesium-imagery: Imagery layer domain models
//!
//! Maps to CesiumJS:
//! - `Scene/ImageryLayer.js`
//! - `Scene/ImageryLayerCollection.js`
//! - `Scene/Imagery.js`
//! - `Scene/ImageryState.js`
//! - `Scene/TileImagery.js`

pub mod imagery_layer;
pub mod imagery_state;
pub mod tile_imagery;
pub mod layer_collection;
pub mod tile_request;
pub mod blending;

pub use imagery_layer::ImageryLayer;
pub use imagery_state::ImageryState;
pub use tile_imagery::TileImagery;
pub use layer_collection::ImageryLayerCollection;
pub use tile_request::{ImageryTileRequest, compute_tile_requests, compute_texture_mapping};
pub use blending::{PixelColor, blend_pixel, composite_layers, compute_effective_alpha, apply_color_adjustments};

use serde::{Deserialize, Serialize};

/// Imagery split direction for split-screen comparison.
/// Maps to CesiumJS `Scene/SplitDirection`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SplitDirection {
    /// Use the left side of the splitter.
    Left = -1,
    /// No split, use the full screen.
    #[default]
    None = 0,
    /// Use the right side of the splitter.
    Right = 1,
}

/// Alpha blending mode for imagery layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AlphaBlendingMode {
    /// Standard alpha blending (src * alpha + dst * (1 - alpha))
    #[default]
    Standard,
    /// Additive blending (src + dst)
    Additive,
    /// Multiplicative blending (src * dst)
    Multiplicative,
}
