//! cesium-terrain：地形领域模型
//!
//! 映射到 CesiumJS：
//! - `Core/QuantizedMeshTerrainData.js`
//! - `Core/HeightmapTerrainData.js`
//! - `Core/TerrainMesh.js`
//! - `Workers/createVerticesFromQuantizedTerrainMesh.js`

pub mod quantized_mesh;
pub mod terrain_mesh;
pub mod heightmap;
pub mod heightmap_tessellator;
pub mod terrain_encoding;

pub use quantized_mesh::QuantizedMeshTerrainData;
pub use terrain_mesh::TerrainMesh;
pub use heightmap::HeightmapTerrainData;
pub use terrain_encoding::{TerrainAttribute, TerrainAttributeLocations, TerrainEncoding};

/// 量化地形坐标的最大值（u16）。
pub const MAX_SHORT: u16 = 32767;

/// 地形量化模式。
/// 映射到 CesiumJS `TerrainQuantization`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerrainQuantization {
    /// 无量化 - 位置以完整精度存储。
    #[default]
    None,
    /// 位置量化到 12 位。
    Bits12,
}
