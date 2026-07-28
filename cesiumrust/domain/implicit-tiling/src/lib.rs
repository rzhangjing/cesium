//! cesium-implicit-tiling: 3D Tiles 1.1 implicit tiling.
//!
//! Domain layer - pure Rust, f64 precision.
//!
//! CesiumJS mapping:
//! - `Scene/Implicit3DTileContent.js` → implicit_tiling

pub mod implicit_tiling;

pub use implicit_tiling::{
    morton_2d, morton_3d, decode_morton_2d, decode_morton_3d,
    AvailabilityBitstream, ImplicitTileCoord, ImplicitTilingConfig,
    SubdivisionScheme, Subtree,
};
