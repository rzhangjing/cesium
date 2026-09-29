//! 高程图地形数据。
//! 映射到 CesiumJS `Core/HeightmapTerrainData.js`

// legacy CesiumJS-port style debt (deferred.md #18); revisit at M13 lint-cleanup 或本文件在其里程碑被重写时
#![allow(clippy::too_many_arguments, clippy::needless_range_loop)]
use cesium_geospatial::bounding::BoundingSphere;
use cesium_geospatial::cartographic::Cartographic;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::math_utils;
use cesium_geospatial::rectangle::Rectangle;
use glam::DVec3;
use serde::{Deserialize, Serialize};

use crate::terrain_mesh::TerrainMesh;

/// 描述原始缓冲区中高度数据的布局。
///
/// 映射到 CesiumJS `HeightmapTessellator.DEFAULT_STRUCTURE` 以及
/// `HeightmapTerrainData` 的 `structure` 选项。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HeightmapStructure {
    /// 从一个高度到下一个高度需要跳过的元素数。
    pub stride: usize,
    /// 构成单个高度值的元素数。
    pub elements_per_height: usize,
    /// 元素之间的乘数（默认 256）。
    pub element_multiplier: f64,
    /// 多元素高度是否为大端序。
    pub is_big_endian: bool,
    /// 解码后应用的缩放。
    pub height_scale: f64,
    /// 缩放后添加的偏移。
    pub height_offset: f64,
    /// 可选的最低钳制值（编码单位）。
    pub lowest_encoded_height: Option<f64>,
    /// 可选的最高钳制值（编码单位）。
    pub highest_encoded_height: Option<f64>,
}

impl Default for HeightmapStructure {
    fn default() -> Self {
        Self {
            stride: 1,
            elements_per_height: 1,
            element_multiplier: 256.0,
            is_big_endian: false,
            height_scale: 1.0,
            height_offset: 0.0,
            lowest_encoded_height: None,
            highest_encoded_height: None,
        }
    }
}

/// 从原始缓冲区中读取给定顶点索引处的高度值。
///
/// 映射到 CesiumJS HeightmapTerrainData.js 中的 `getHeight`。
pub fn get_height_from_buffer(
    buffer: &[u8],
    structure: &HeightmapStructure,
    index: usize,
) -> f64 {
    let offset = index * structure.stride;
    let mut height = 0.0f64;

    if structure.is_big_endian {
        for i in 0..structure.elements_per_height {
            height = height * structure.element_multiplier + buffer[offset + i] as f64;
        }
    } else {
        for i in (0..structure.elements_per_height).rev() {
            height = height * structure.element_multiplier + buffer[offset + i] as f64;
        }
    }

    height
}

/// 将高度值写入原始缓冲区中给定顶点索引处。
///
/// 映射到 CesiumJS HeightmapTerrainData.js 中的 `setHeight`。
pub fn set_height_in_buffer(
    buffer: &mut [u8],
    structure: &HeightmapStructure,
    index: usize,
    mut height: f64,
) {
    let offset = index * structure.stride;
    let divisor = structure
        .element_multiplier
        .powi(structure.elements_per_height as i32 - 1);
    let mut div = divisor;

    if structure.is_big_endian {
        for i in 0..structure.elements_per_height - 1 {
            let val = (height / div).floor() as u8;
            buffer[offset + i] = val;
            height -= val as f64 * div;
            div /= structure.element_multiplier;
        }
        // 最后一个元素取余数
        buffer[offset + structure.elements_per_height - 1] = height as u8;
    } else {
        for i in (1..structure.elements_per_height).rev() {
            let val = (height / div).floor() as u8;
            buffer[offset + i] = val;
            height -= val as f64 * div;
            div /= structure.element_multiplier;
        }
        // 第一个元素（索引 0）取余数
        buffer[offset] = height as u8;
    }
}

/// 使用 CesiumJS 三角形方法从网格插值高度。
///
/// 网格以行主序存储，行从北（NORTH）到南（SOUTH）排列
/// （第 0 行 = 北，第 height-1 行 = 南），与 CesiumJS 网格布局一致。
/// `u` 为西到东 [0,1]，`v` 为南到北 [0,1]。
///
/// 映射到 CesiumJS `interpolateHeight` / `interpolateMeshHeight` +
/// `triangleInterpolateHeight`。
fn interpolate_height_from_grid(
    heights: &[f64],
    width: usize,
    height: usize,
    u: f64,
    v: f64,
) -> f64 {
    // 将 u,v 转换为网格坐标（fromWest、fromSouth）
    let from_west = u * (width - 1) as f64;
    let from_south = v * (height - 1) as f64;

    let mut west_int = from_west as usize;
    let mut east_int = west_int + 1;
    if east_int >= width {
        east_int = width - 1;
        west_int = width - 2;
    }

    let mut south_int = from_south as usize;
    let mut north_int = south_int + 1;
    if north_int >= height {
        north_int = height - 1;
        south_int = height - 2;
    }

    let dx = from_west - west_int as f64;
    let dy = from_south - south_int as f64;

    // 翻转行索引：网格行从北到南排列，但 south_int/north_int
    // 处于南到北的空间中。
    let south_row = height - 1 - south_int;
    let north_row = height - 1 - north_int;

    let sw = heights[south_row * width + west_int];
    let se = heights[south_row * width + east_int];
    let nw = heights[north_row * width + west_int];
    let ne = heights[north_row * width + east_int];

    // 三角形插值（CesiumJS 沿 SW 到 NE 对四边形二分）
    if dy < dx {
        // 右下三角形
        sw + dx * (se - sw) + dy * (ne - se)
    } else {
        // 左上三角形
        sw + dx * (ne - nw) + dy * (nw - sw)
    }
}

/// 以高程图表示的地形数据。
///
/// 高程图是覆盖矩形区域的高度值规则网格。
///
/// 映射到 CesiumJS `HeightmapTerrainData`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeightmapTerrainData {
    /// 以行主序存储的高度值（南到北，西到东）
    pub heights: Vec<f64>,

    /// 行数（纬度采样）
    pub width: usize,

    /// 列数（经度采样）
    pub height: usize,

    /// 图块中的最小高度
    pub minimum_height: f64,

    /// 图块中的最大高度
    pub maximum_height: f64,

    /// 图块的包围球
    pub bounding_sphere: BoundingSphere,

    /// 指示哪些子块存在的位掩码
    #[serde(default = "default_child_mask")]
    pub child_tile_mask: u8,

    /// 是否由上采样创建
    #[serde(default)]
    pub created_by_upsampling: bool,
}

fn default_child_mask() -> u8 {
    15
}

impl HeightmapTerrainData {
    /// 创建新的高程图地形数据。
    pub fn new(
        heights: Vec<f64>,
        width: usize,
        height: usize,
        minimum_height: f64,
        maximum_height: f64,
    ) -> Self {
        let bounding_sphere = BoundingSphere::new(DVec3::ZERO, 0.0);
        Self {
            heights,
            width,
            height,
            minimum_height,
            maximum_height,
            bounding_sphere,
            child_tile_mask: 15,
            created_by_upsampling: false,
        }
    }

    /// 获取特定网格位置处的高度。
    pub fn get_height(&self, col: usize, row: usize) -> Option<f64> {
        if col < self.width && row < self.height {
            Some(self.heights[row * self.width + col])
        } else {
            None
        }
    }

    /// 在小数网格位置处插值高度。
    pub fn interpolate_height(&self, u: f64, v: f64) -> f64 {
        let col_f = u * (self.width - 1) as f64;
        let row_f = v * (self.height - 1) as f64;

        let col0 = col_f.floor() as usize;
        let row0 = row_f.floor() as usize;
        let col1 = (col0 + 1).min(self.width - 1);
        let row1 = (row0 + 1).min(self.height - 1);

        let du = col_f - col0 as f64;
        let dv = row_f - row0 as f64;

        let h00 = self.heights[row0 * self.width + col0];
        let h10 = self.heights[row0 * self.width + col1];
        let h01 = self.heights[row1 * self.width + col0];
        let h11 = self.heights[row1 * self.width + col1];

        // 双线性插值
        let h0 = math_utils::lerp(h00, h10, du);
        let h1 = math_utils::lerp(h01, h11, du);
        math_utils::lerp(h0, h1, dv)
    }

    /// 从高程图创建地形网格。
    ///
    /// # 参数
    /// * `rectangle` - 图块矩形
    /// * `ellipsoid` - 椭球体
    pub fn create_mesh(&self, rectangle: &Rectangle, ellipsoid: &Ellipsoid) -> TerrainMesh {
        let mut positions = Vec::with_capacity(self.width * self.height);
        let mut uvs = Vec::with_capacity(self.width * self.height);
        let mut indices = Vec::new();

        // 生成顶点
        for row in 0..self.height {
            let v = row as f64 / (self.height - 1) as f64;
            let lat = math_utils::lerp(rectangle.south, rectangle.north, v);

            for col in 0..self.width {
                let u = col as f64 / (self.width - 1) as f64;
                let lon = math_utils::lerp(rectangle.west, rectangle.east, u);
                let height = self.heights[row * self.width + col];

                let carto = Cartographic::from_radians(lon, lat, height);
                let pos = ellipsoid.cartographic_to_cartesian(&carto);

                positions.push([pos.x, pos.y, pos.z]);
                uvs.push([u, v]);
            }
        }

        // 生成索引
        for row in 0..self.height - 1 {
            for col in 0..self.width - 1 {
                let i0 = (row * self.width + col) as u32;
                let i1 = i0 + 1;
                let i2 = i0 + self.width as u32;
                let i3 = i2 + 1;

                // 每个四边形两个三角形
                indices.push(i0);
                indices.push(i2);
                indices.push(i1);

                indices.push(i1);
                indices.push(i2);
                indices.push(i3);
            }
        }

        let mut mesh = TerrainMesh {
            positions,
            normals: None,
            tex_coords: Some(uvs),
            indices,
            minimum_height: self.minimum_height,
            maximum_height: self.maximum_height,
            bounding_sphere: self.bounding_sphere,
        };

        mesh.compute_normals();
        mesh
    }

    /// 检查特定子块是否存在。
    pub fn is_child_available(&self, child: usize) -> bool {
        (self.child_tile_mask & (1 << child)) != 0
    }

    /// 使用带结构编码的原始字节缓冲区对高程图进行上采样。
    ///
    /// 这是 CesiumJS `HeightmapTerrainData.upsample` 针对
    /// 多元素/步长/大端缓冲区的忠实移植。它从原始
    /// 缓冲区解码高度、插值、钳制并重新编码。
    ///
    /// # 参数
    /// * `buffer` - 包含编码高度的原始字节缓冲区
    /// * `structure` - 高度数据布局描述
    /// * `this_x/this_y/this_level` - 当前图块坐标
    /// * `descendant_x/descendant_y/descendant_level` - 子图块坐标
    pub fn upsample_with_structure(
        &self,
        buffer: &[u8],
        structure: &HeightmapStructure,
        this_x: u32,
        this_y: u32,
        this_level: u32,
        descendant_x: u32,
        descendant_y: u32,
        descendant_level: u32,
    ) -> Vec<u8> {
        let level_difference = descendant_level - this_level;
        assert!(level_difference == 1, "upsample can only cross one level");

        let width = self.width;
        let height = self.height;

        // 计算子块在父块内的相对位置
        let relative_x = descendant_x - this_x * 2;
        let relative_y = descendant_y - this_y * 2;

        // 子块覆盖父块的 [relative/2, (relative+1)/2]
        let west_frac = relative_x as f64 / 2.0;
        let east_frac = (relative_x + 1) as f64 / 2.0;
        // CesiumJS 图块 Y 向南递增；子块第 0 行 = 北
        let north_frac = relative_y as f64 / 2.0;
        let south_frac = (relative_y + 1) as f64 / 2.0;

        // 从缓冲区解码所有源高度
        let mut source_heights = vec![0.0f64; width * height];
        for idx in 0..width * height {
            let h = get_height_from_buffer(buffer, structure, idx);
            source_heights[idx] = h * structure.height_scale + structure.height_offset;
        }

        // 输出缓冲区
        let mut out_buffer = vec![0u8; width * height * structure.stride];

        for j in 0..height {
            // CesiumJS 从北到南遍历各行
            let v = j as f64 / (height - 1) as f64;
            // j=0 → 目标北 → 父块 v = 1 - north_frac
            // j=height-1 → 目标南 → 父块 v = 1 - south_frac
            let parent_v = (1.0 - north_frac) + v * ((1.0 - south_frac) - (1.0 - north_frac));

            for i in 0..width {
                let u = i as f64 / (width - 1) as f64;
                let parent_u = west_frac + u * (east_frac - west_frac);

                // 使用三角形方法插值（忠实于 CesiumJS）
                let h = interpolate_height_from_grid(
                    &source_heights,
                    width,
                    height,
                    parent_u,
                    parent_v,
                );

                // 钳制
                let mut h_clamped = h;
                if let Some(low) = structure.lowest_encoded_height {
                    if h_clamped < low {
                        h_clamped = low;
                    }
                }
                if let Some(high) = structure.highest_encoded_height {
                    if h_clamped > high {
                        h_clamped = high;
                    }
                }

                set_height_in_buffer(
                    &mut out_buffer,
                    structure,
                    j * width + i,
                    h_clamped,
                );
            }
        }

        out_buffer
    }

    /// 对高程图进行上采样，在给定位置生成子图块。
    ///
    /// 映射到 CesiumJS `HeightmapTerrainData.upsample`。
    ///
    /// # 参数
    /// * `this_x` - 当前图块的 X 坐标
    /// * `this_y` - 当前图块的 Y 坐标
    /// * `this_level` - 当前图块的层级
    /// * `descendant_x` - 子图块的 X 坐标
    /// * `descendant_y` - 子图块的 Y 坐标
    /// * `descendant_level` - 子图块的层级（必须为 this_level + 1）
    pub fn upsample(
        &self,
        this_x: u32,
        this_y: u32,
        this_level: u32,
        descendant_x: u32,
        descendant_y: u32,
        descendant_level: u32,
    ) -> HeightmapTerrainData {
        let level_difference = descendant_level - this_level;
        assert!(
            level_difference == 1,
            "upsample can only cross one level"
        );

        // 计算子块在父块内的位置
        let tiles_at_this_level = 1u32 << level_difference;
        let relative_x = descendant_x - this_x * tiles_at_this_level;
        let relative_y = descendant_y - this_y * tiles_at_this_level;

        // 子块覆盖父块的 [relative_x/tiles, (relative_x+1)/tiles]
        let west_fraction = relative_x as f64 / tiles_at_this_level as f64;
        let east_fraction = (relative_x + 1) as f64 / tiles_at_this_level as f64;
        let south_fraction = relative_y as f64 / tiles_at_this_level as f64;
        let north_fraction = (relative_y + 1) as f64 / tiles_at_this_level as f64;

        // 子块与父块尺寸相同
        let child_width = self.width;
        let child_height = self.height;
        let mut child_heights = vec![0.0f64; child_width * child_height];

        let mut min_h = f64::MAX;
        let mut max_h = f64::MIN;

        for row in 0..child_height {
            let v = row as f64 / (child_height - 1) as f64;
            let parent_v = south_fraction + v * (north_fraction - south_fraction);

            for col in 0..child_width {
                let u = col as f64 / (child_width - 1) as f64;
                let parent_u = west_fraction + u * (east_fraction - west_fraction);

                let h = self.interpolate_height(parent_u, parent_v);
                child_heights[row * child_width + col] = h;
                min_h = min_h.min(h);
                max_h = max_h.max(h);
            }
        }

        let mut child = HeightmapTerrainData::new(
            child_heights,
            child_width,
            child_height,
            min_h,
            max_h,
        );
        child.created_by_upsampling = true;
        child
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_heightmap() -> HeightmapTerrainData {
        // 3x3 高程图
        let heights = vec![
            0.0, 100.0, 0.0,
            100.0, 200.0, 100.0,
            0.0, 100.0, 0.0,
        ];
        HeightmapTerrainData::new(heights, 3, 3, 0.0, 200.0)
    }

    #[test]
    fn test_get_height() {
        let data = create_test_heightmap();
        assert_eq!(data.get_height(0, 0), Some(0.0));
        assert_eq!(data.get_height(1, 1), Some(200.0));
        assert_eq!(data.get_height(2, 2), Some(0.0));
        assert_eq!(data.get_height(3, 0), None);
    }

    #[test]
    fn test_interpolate_height() {
        let data = create_test_heightmap();
        // 中心应为 200
        assert!((data.interpolate_height(0.5, 0.5) - 200.0).abs() < 0.01);
        // 角点应为 0
        assert!((data.interpolate_height(0.0, 0.0) - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_create_mesh() {
        let data = create_test_heightmap();
        let rectangle = Rectangle::from_degrees(-1.0, -1.0, 1.0, 1.0);
        let ellipsoid = Ellipsoid::WGS84;

        let mesh = data.create_mesh(&rectangle, &ellipsoid);

        assert_eq!(mesh.positions.len(), 9); // 3x3
        assert_eq!(mesh.indices.len(), 24); // 4 个四边形 * 2 个三角形 * 3 个索引
        assert!(mesh.normals.is_some());
    }

    #[test]
    fn test_child_availability() {
        let data = create_test_heightmap();
        assert!(data.is_child_available(0));
        assert!(data.is_child_available(3));
    }
}
