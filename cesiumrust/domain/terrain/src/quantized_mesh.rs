//! 量化网格地形数据。
//! 以量化顶点（u/v/height 各限 [0,32767]）紧凑表示单个地形瓦片。

use cesium_geospatial::bounding::BoundingSphere;
use cesium_geospatial::cartographic::Cartographic;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::math_utils;
use cesium_geospatial::rectangle::Rectangle;
use glam::DVec3;
use serde::{Deserialize, Serialize};

use crate::terrain_mesh::TerrainMesh;
use crate::MAX_SHORT;

/// 单个图块的地形数据，其中地形以量化网格表示。
///
/// 量化网格由三个顶点属性组成：经度 (u)、纬度 (v) 和高度。
/// 所有属性均以 16 位值表示，范围为 0 到 32767。
///
/// - u：西边缘为 0，东边缘为 32767
/// - v：南边缘为 0，北边缘为 32767
/// - height：最小高度为 0，最大高度为 32767
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizedMeshTerrainData {
    /// 量化顶点数据：[u0, u1, ..., v0, v1, ..., h0, h1, ...]
    /// 每个分量是范围 [0, 32767] 内的 u16
    pub quantized_vertices: Vec<u16>,

    /// 三角形索引（根据顶点数可为 u16 或 u32）
    pub indices: Vec<u32>,

    /// 椭球上方以米计的最小地形高度
    pub minimum_height: f64,

    /// 椭球上方以米计的最大地形高度
    pub maximum_height: f64,

    /// 图块的包围球
    pub bounding_sphere: BoundingSphere,

    /// 椭球缩放坐标中的地平线遮挡点
    pub horizon_occlusion_point: DVec3,

    /// 西边缘顶点的索引
    pub west_indices: Vec<u32>,

    /// 南边缘顶点的索引
    pub south_indices: Vec<u32>,

    /// 东边缘顶点的索引
    pub east_indices: Vec<u32>,

    /// 北边缘顶点的索引
    pub north_indices: Vec<u32>,

    /// 西边缘的裙边高度
    pub west_skirt_height: f64,

    /// 南边缘的裙边高度
    pub south_skirt_height: f64,

    /// 东边缘的裙边高度
    pub east_skirt_height: f64,

    /// 北边缘的裙边高度
    pub north_skirt_height: f64,

    /// 指示哪些子块存在的位掩码（bit 0=SW, 1=SE, 2=NW, 3=NE）
    #[serde(default = "default_child_tile_mask")]
    pub child_tile_mask: u8,

    /// 是否由上采样创建
    #[serde(default)]
    pub created_by_upsampling: bool,

    /// Oct 编码的法线（可选）
    #[serde(default)]
    pub encoded_normals: Option<Vec<u8>>,

    /// 水面遮罩（可选）
    #[serde(default)]
    pub water_mask: Option<Vec<u8>>,
}

/// 缺省子瓦片掩码：低 4 位置 1，表示四个子块默认可用。
fn default_child_tile_mask() -> u8 {
    15 // 默认所有子块都存在
}

impl QuantizedMeshTerrainData {
    /// 返回网格中的顶点数。
    pub fn vertex_count(&self) -> usize {
        self.quantized_vertices.len() / 3
    }

    /// 返回所有顶点的 u 值（经度量化）。
    pub fn u_values(&self) -> &[u16] {
        let count = self.vertex_count();
        &self.quantized_vertices[0..count]
    }

    /// 返回所有顶点的 v 值（纬度量化）。
    pub fn v_values(&self) -> &[u16] {
        let count = self.vertex_count();
        &self.quantized_vertices[count..2 * count]
    }

    /// 返回所有顶点的高度值。
    pub fn height_values(&self) -> &[u16] {
        let count = self.vertex_count();
        &self.quantized_vertices[2 * count..3 * count]
    }

    /// 检查特定子块是否存在。
    ///
    /// # 参数
    /// * `child` - 子索引（0=SW, 1=SE, 2=NW, 3=NE）
    pub fn is_child_available(&self, child: usize) -> bool {
        (self.child_tile_mask & (1 << child)) != 0
    }

    /// 使用图块坐标检查子块是否可用。
    ///
    /// 由父子坐标算出相对位置，映射到西北/东北/西南/东南对应的掩码位。
    ///
    /// # 参数
    /// * `this_x` - 父块 X
    /// * `this_y` - 父块 Y
    /// * `child_x` - 子块 X
    /// * `child_y` - 子块 Y
    pub fn is_child_available_coords(
        &self,
        this_x: u32,
        this_y: u32,
        child_x: u32,
        child_y: u32,
    ) -> bool {
        let relative_x = child_x - this_x * 2;
        let relative_y = child_y - this_y * 2;

        // 图块坐标：Y 向南递增
        // relative_y=0 → 北行，relative_y=1 → 南行
        if relative_y == 0 {
            if relative_x == 0 {
                // 西北子块（bit 2）
                (self.child_tile_mask & 4) != 0
            } else {
                // 东北子块（bit 3）
                (self.child_tile_mask & 8) != 0
            }
        } else if relative_x == 0 {
            // 西南子块（bit 0）
            (self.child_tile_mask & 1) != 0
        } else {
            // 东南子块（bit 1）
            (self.child_tile_mask & 2) != 0
        }
    }

    /// 在图块矩形内对给定经度/纬度处的高度进行插值。
    ///
    /// 使用重心坐标找到包含该点的三角形并进行插值。
    pub fn interpolate_height(&self, rectangle: &Rectangle, longitude: f64, latitude: f64) -> f64 {
        let vertex_count = self.vertex_count();
        let u_values = self.u_values();
        let v_values = self.v_values();
        let height_values = self.height_values();

        // 限制到矩形边界
        let lon = longitude.clamp(rectangle.west, rectangle.east);
        let lat = latitude.clamp(rectangle.south, rectangle.north);

        // 转换为图块内归一化的 u,v 坐标
        let width = rectangle.east - rectangle.west;
        let height_range = rectangle.north - rectangle.south;
        let u = if width > 0.0 { (lon - rectangle.west) / width } else { 0.0 };
        let v = if height_range > 0.0 { (lat - rectangle.south) / height_range } else { 0.0 };

        // 将 u,v 转换为量化坐标
        let target_u = u * MAX_SHORT as f64;
        let target_v = v * MAX_SHORT as f64;

        // 找到包含该点的三角形并插值
        let indices = &self.indices;
        for tri in indices.chunks(3) {
            if tri.len() < 3 {
                break;
            }
            let i0 = tri[0] as usize;
            let i1 = tri[1] as usize;
            let i2 = tri[2] as usize;

            if i0 >= vertex_count || i1 >= vertex_count || i2 >= vertex_count {
                continue;
            }

            let u0 = u_values[i0] as f64;
            let v0 = v_values[i0] as f64;
            let u1 = u_values[i1] as f64;
            let v1 = v_values[i1] as f64;
            let u2 = u_values[i2] as f64;
            let v2 = v_values[i2] as f64;

            // 计算重心坐标
            let denom = (v1 - v2) * (u0 - u2) + (u2 - u1) * (v0 - v2);
            if denom.abs() < 1e-30 {
                continue;
            }

            let a = ((v1 - v2) * (target_u - u2) + (u2 - u1) * (target_v - v2)) / denom;
            let b = ((v2 - v0) * (target_u - u2) + (u0 - u2) * (target_v - v2)) / denom;
            let c = 1.0 - a - b;

            // 检查点是否在三角形内（带小容差）
            if a >= -1e-10 && b >= -1e-10 && c >= -1e-10 {
                let h0 = math_utils::lerp(
                    self.minimum_height,
                    self.maximum_height,
                    height_values[i0] as f64 / MAX_SHORT as f64,
                );
                let h1 = math_utils::lerp(
                    self.minimum_height,
                    self.maximum_height,
                    height_values[i1] as f64 / MAX_SHORT as f64,
                );
                let h2 = math_utils::lerp(
                    self.minimum_height,
                    self.maximum_height,
                    height_values[i2] as f64 / MAX_SHORT as f64,
                );

                return a * h0 + b * h1 + c * h2;
            }
        }

        // 回退：返回平均高度
        (self.minimum_height + self.maximum_height) * 0.5
    }

    /// 从量化数据创建地形网格。
    ///
    /// 这是主方法，使用图块矩形和椭球体将量化网格数据
    /// 转换为实际的 3D 位置。
    ///
    /// # 参数
    /// * `rectangle` - 图块矩形（west、south、east、north，以弧度计）
    /// * `ellipsoid` - 用于坐标转换的椭球体
    /// * `exaggeration` - 垂直夸张因子（1.0 = 无夸张）
    ///
    /// # 返回
    /// 带有实际 3D 位置的 TerrainMesh
    pub fn create_mesh(
        &self,
        rectangle: &Rectangle,
        ellipsoid: &Ellipsoid,
        exaggeration: f64,
    ) -> TerrainMesh {
        let vertex_count = self.vertex_count();
        let u_values = self.u_values();
        let v_values = self.v_values();
        let height_values = self.height_values();

        let west = rectangle.west;
        let south = rectangle.south;
        let east = rectangle.east;
        let north = rectangle.north;

        let mut positions = Vec::with_capacity(vertex_count);
        let mut uvs = Vec::with_capacity(vertex_count);
        let mut heights = Vec::with_capacity(vertex_count);
        let mut normals = Vec::with_capacity(vertex_count);

        let has_exaggeration = (exaggeration - 1.0).abs() > f64::EPSILON;

        for i in 0..vertex_count {
            let u = u_values[i] as f64 / MAX_SHORT as f64;
            let v = v_values[i] as f64 / MAX_SHORT as f64;
            let height = math_utils::lerp(
                self.minimum_height,
                self.maximum_height,
                height_values[i] as f64 / MAX_SHORT as f64,
            );

            let longitude = math_utils::lerp(west, east, u);
            let latitude = math_utils::lerp(south, north, v);

            let carto = Cartographic::from_radians(longitude, latitude, height);
            let position = ellipsoid.cartographic_to_cartesian(&carto);

            positions.push([position.x, position.y, position.z]);
            uvs.push([u, v]);
            heights.push(height);

            // 若应用了夸张，则计算大地测量表面法线
            if has_exaggeration {
                let normal = ellipsoid
                    .geodetic_surface_normal(position)
                    .unwrap_or(DVec3::Z);
                normals.push([normal.x, normal.y, normal.z]);
            }
        }

        // 若可用则解码 oct 编码的法线
        if let Some(ref encoded) = self.encoded_normals {
            normals = decode_oct_normals(encoded, vertex_count);
        }

        TerrainMesh {
            positions,
            normals: if normals.is_empty() { None } else { Some(normals) },
            tex_coords: Some(uvs),
            indices: self.indices.clone(),
            minimum_height: self.minimum_height,
            maximum_height: self.maximum_height,
            bounding_sphere: self.bounding_sphere,
        }
    }

    /// 创建带裙边的地形网格，以实现无缝的图块边界。
    ///
    /// # 参数
    /// * `rectangle` - 图块矩形
    /// * `ellipsoid` - 椭球体
    /// * `exaggeration` - 垂直夸张因子
    pub fn create_mesh_with_skirts(
        &self,
        rectangle: &Rectangle,
        ellipsoid: &Ellipsoid,
        exaggeration: f64,
    ) -> TerrainMesh {
        let mut mesh = self.create_mesh(rectangle, ellipsoid, exaggeration);

        // 添加裙边顶点
        self.add_skirts(&mut mesh, rectangle, ellipsoid);

        mesh
    }

    /// 向网格添加裙边顶点，以实现无缝边界。
    fn add_skirts(&self, mesh: &mut TerrainMesh, _rectangle: &Rectangle, ellipsoid: &Ellipsoid) {
        let base_vertex_count = mesh.positions.len();

        // 辅助函数：为某条边添加裙边
        let mut add_edge_skirt = |edge_indices: &[u32], skirt_height: f64| {
            for &idx in edge_indices {
                let idx = idx as usize;
                if idx < base_vertex_count {
                    let pos = mesh.positions[idx];
                    let position = DVec3::new(pos[0], pos[1], pos[2]);

                    // 获取地图投影坐标
                    if let Some(carto) = ellipsoid.cartesian_to_cartographic(position) {
                        // 按裙边量降低高度
                        let skirt_carto = Cartographic::from_radians(
                            carto.longitude,
                            carto.latitude,
                            carto.height - skirt_height,
                        );
                        let skirt_pos = ellipsoid.cartographic_to_cartesian(&skirt_carto);
                        mesh.positions.push([skirt_pos.x, skirt_pos.y, skirt_pos.z]);

                        // 复制 UV 和法线
                        let uv_to_copy = mesh.tex_coords.as_ref().and_then(|uvs| uvs.get(idx).copied());
                        if let Some(uv) = uv_to_copy {
                            if let Some(ref mut new_uvs) = mesh.tex_coords {
                                new_uvs.push(uv);
                            }
                        }
                        let normal_to_copy = mesh.normals.as_ref().and_then(|normals| normals.get(idx).copied());
                        if let Some(normal) = normal_to_copy {
                            if let Some(ref mut new_normals) = mesh.normals {
                                new_normals.push(normal);
                            }
                        }
                    }
                }
            }
        };

        // 为每条边添加裙边
        add_edge_skirt(&self.west_indices, self.west_skirt_height);
        add_edge_skirt(&self.south_indices, self.south_skirt_height);
        add_edge_skirt(&self.east_indices, self.east_skirt_height);
        add_edge_skirt(&self.north_indices, self.north_skirt_height);

        // 添加裙边三角形
        let mut add_skirt_indices = |edge_indices: &[u32], offset: usize| {
            for i in 0..edge_indices.len().saturating_sub(1) {
                let v0 = edge_indices[i];
                let v1 = edge_indices[i + 1];
                let v2 = (offset + i) as u32;
                let v3 = (offset + i + 1) as u32;

                // 裙边四边形对应的两个三角形
                mesh.indices.push(v0);
                mesh.indices.push(v2);
                mesh.indices.push(v1);

                mesh.indices.push(v1);
                mesh.indices.push(v2);
                mesh.indices.push(v3);
            }
        };

        let mut offset = base_vertex_count;
        add_skirt_indices(&self.west_indices, offset);
        offset += self.west_indices.len();
        add_skirt_indices(&self.south_indices, offset);
        offset += self.south_indices.len();
        add_skirt_indices(&self.east_indices, offset);
        offset += self.east_indices.len();
        add_skirt_indices(&self.north_indices, offset);
    }
}

/// 解码 oct 编码的法线。
///
/// Oct 编码使用八面体投影将单位向量映射为两个字节。
fn decode_oct_normals(encoded: &[u8], vertex_count: usize) -> Vec<[f64; 3]> {
    let mut normals = Vec::with_capacity(vertex_count);

    for i in 0..vertex_count {
        let x = encoded.get(i * 2).copied().unwrap_or(128);
        let y = encoded.get(i * 2 + 1).copied().unwrap_or(128);

        // 从 [0, 255] 解码到 [-1, 1]
        let mut decoded_x = (x as f64 / 255.0) * 2.0 - 1.0;
        let mut decoded_y = (y as f64 / 255.0) * 2.0 - 1.0;

        // Oct 解码
        let z = 1.0 - decoded_x.abs() - decoded_y.abs();
        if z < 0.0 {
            let old_x = decoded_x;
            decoded_x = (1.0 - decoded_y.abs()) * old_x.signum();
            decoded_y = (1.0 - old_x.abs()) * decoded_y.signum();
        }

        // 归一化
        let len = (decoded_x * decoded_x + decoded_y * decoded_y + z * z).sqrt();
        if len > 0.0 {
            normals.push([decoded_x / len, decoded_y / len, z / len]);
        } else {
            normals.push([0.0, 0.0, 1.0]);
        }
    }

    normals
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_geospatial::bounding::BoundingSphere;

    fn create_test_data() -> QuantizedMeshTerrainData {
        // 简单的 4 顶点四边形（SW, NW, SE, NE）
        QuantizedMeshTerrainData {
            quantized_vertices: vec![
                // u 值
                0, 0, 32767, 32767,
                // v 值
                0, 32767, 0, 32767,
                // 高度值
                16384, 0, 32767, 16384,
            ],
            indices: vec![0, 3, 1, 0, 2, 3],
            minimum_height: -100.0,
            maximum_height: 2101.0,
            bounding_sphere: BoundingSphere::new(DVec3::new(1.0, 2.0, 3.0), 10000.0),
            horizon_occlusion_point: DVec3::new(3.0, 2.0, 1.0),
            west_indices: vec![0, 1],
            south_indices: vec![0, 2],
            east_indices: vec![2, 3],
            north_indices: vec![1, 3],
            west_skirt_height: 100.0,
            south_skirt_height: 100.0,
            east_skirt_height: 100.0,
            north_skirt_height: 100.0,
            child_tile_mask: 15,
            created_by_upsampling: false,
            encoded_normals: None,
            water_mask: None,
        }
    }

    #[test]
    fn test_vertex_count() {
        let data = create_test_data();
        assert_eq!(data.vertex_count(), 4);
    }

    #[test]
    fn test_u_values() {
        let data = create_test_data();
        assert_eq!(data.u_values(), &[0, 0, 32767, 32767]);
    }

    #[test]
    fn test_v_values() {
        let data = create_test_data();
        assert_eq!(data.v_values(), &[0, 32767, 0, 32767]);
    }

    #[test]
    fn test_height_values() {
        let data = create_test_data();
        assert_eq!(data.height_values(), &[16384, 0, 32767, 16384]);
    }

    #[test]
    fn test_child_availability() {
        let data = create_test_data();
        assert!(data.is_child_available(0)); // SW
        assert!(data.is_child_available(1)); // SE
        assert!(data.is_child_available(2)); // NW
        assert!(data.is_child_available(3)); // NE
    }

    #[test]
    fn test_create_mesh() {
        let data = create_test_data();
        let rectangle = Rectangle::from_degrees(-10.0, -10.0, 10.0, 10.0);
        let ellipsoid = Ellipsoid::WGS84;

        let mesh = data.create_mesh(&rectangle, &ellipsoid, 1.0);

        assert_eq!(mesh.positions.len(), 4);
        assert_eq!(mesh.indices.len(), 6);
        assert!(mesh.tex_coords.is_some());
    }

    #[test]
    fn test_create_mesh_with_skirts() {
        let data = create_test_data();
        let rectangle = Rectangle::from_degrees(-10.0, -10.0, 10.0, 10.0);
        let ellipsoid = Ellipsoid::WGS84;

        let mesh = data.create_mesh_with_skirts(&rectangle, &ellipsoid, 1.0);

        // 由于裙边应有更多顶点
        assert!(mesh.positions.len() > 4);
        // 由于裙边三角形应有更多索引
        assert!(mesh.indices.len() > 6);
    }

    #[test]
    fn test_decode_oct_normals() {
        // 测试指向朝上的编码法线（128, 128 = 中心）
        let encoded = vec![128, 128];
        let normals = decode_oct_normals(&encoded, 1);

        assert_eq!(normals.len(), 1);
        // 应约为 [0, 0, 1]
        assert!((normals[0][2] - 1.0).abs() < 0.1);
    }
}
