//! cesium-terrain：地形领域模型
//!
//! 涵盖量化网格/高度图地形数据、地形网格与顶点编码，以及高度图细分。
//!
//! 子模块：
//! - [`quantized_mesh`]：量化网格地形数据
//! - [`heightmap`]：高度图地形数据
//! - [`terrain_mesh`]：地形网格与瓦片矩形
//! - [`heightmap_tessellator`]：高度图顶点细分
//! - [`terrain_encoding`]：地形顶点属性编码

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
/// 控制顶点坐标量化的位宽精度。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerrainQuantization {
    /// 无量化 - 位置以完整精度存储。
    #[default]
    None,
    /// 位置量化到 12 位。
    Bits12,
}
