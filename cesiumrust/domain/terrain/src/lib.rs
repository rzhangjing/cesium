//! cesium-terrain: Terrain domain models
//!
//! Maps to CesiumJS:
//! - `Core/QuantizedMeshTerrainData.js`
//! - `Core/HeightmapTerrainData.js`
//! - `Core/TerrainMesh.js`
//! - `Workers/createVerticesFromQuantizedTerrainMesh.js`

pub mod quantized_mesh;
pub mod terrain_mesh;
pub mod heightmap;

pub use quantized_mesh::QuantizedMeshTerrainData;
pub use terrain_mesh::TerrainMesh;
pub use heightmap::HeightmapTerrainData;

/// The maximum value for quantized terrain coordinates (u16).
pub const MAX_SHORT: u16 = 32767;

/// Terrain quantization mode.
/// Maps to CesiumJS `TerrainQuantization`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerrainQuantization {
    /// No quantization - positions stored as full precision.
    #[default]
    None,
    /// Positions quantized to 12 bits.
    Bits12,
}
