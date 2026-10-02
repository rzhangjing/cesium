//! 地形网格表示。
//! 存放地形数据处理后、可直接用于渲染的顶点/索引网格及其包围体。

use cesium_geospatial::bounding::BoundingSphere;
use serde::{Deserialize, Serialize};

/// 表示地形几何的网格。
///
/// 这是地形数据处理的输出 - 可直接用于渲染的实际 3D 位置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainMesh {
    /// ECEF 坐标中的顶点位置，每顶点 [x, y, z]
    ///
    /// 与 indices 一一对应，构成网格的几何骨架。
    pub positions: Vec<[f64; 3]>,

    /// 顶点法线（可选），每顶点 [x, y, z]
    ///
    /// 为 None 时可由 compute_normals 惰性生成。
    pub normals: Option<Vec<[f64; 3]>>,

    /// 纹理坐标，每顶点 [u, v]
    ///
    /// 缺省时由采样阶段按需补算。
    pub tex_coords: Option<Vec<[f64; 2]>>,

    /// 三角形索引，每三个一组构成一个面。
    ///
    /// 索引指向 [`positions`](Self::positions) 中的顶点。
    pub indices: Vec<u32>,

    /// 网格中的最小高度（大地水准面以上，单位：米）。
    pub minimum_height: f64,

    /// 网格中的最大高度（大地水准面以上，单位：米）。
    pub maximum_height: f64,

    /// 网格的包围球，用于可见性与细分判定。
    pub bounding_sphere: BoundingSphere,
}

impl TerrainMesh {
    /// 返回网格中的顶点数。
    pub fn vertex_count(&self) -> usize {
        // 顶点数即位置数组长度。
        self.positions.len()
    }

    /// 返回网格中的三角形数。
    pub fn triangle_count(&self) -> usize {
        // 每个三角形占用三个索引。
        self.indices.len() / 3
    }

    /// 若不存在，则从三角形面计算顶点法线。
    pub fn compute_normals(&mut self) {
        // 已有法线则直接复用，避免重算。
        if self.normals.is_some() {
            return;
        }

        let vertex_count = self.positions.len();
        let mut normals = vec![[0.0f64; 3]; vertex_count];

        // 累加面法线
        for tri in self.indices.chunks(3) {
            if tri.len() < 3 {
                continue;
            }

            let i0 = tri[0] as usize;
            let i1 = tri[1] as usize;
            let i2 = tri[2] as usize;

            if i0 >= vertex_count || i1 >= vertex_count || i2 >= vertex_count {
                continue;
            }

            let p0 = self.positions[i0];
            let p1 = self.positions[i1];
            let p2 = self.positions[i2];

            // 面法线由两条边向量的叉积得到。
            let e1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
            let e2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];

            let normal = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];

            // 将面法线累加到共享该面的各顶点。
            for &idx in tri {
                let idx = idx as usize;
                normals[idx][0] += normal[0];
                normals[idx][1] += normal[1];
                normals[idx][2] += normal[2];
            }
        }

        // 归一化：面法线累加后需除以其长度得到单位法线。
        for normal in normals.iter_mut() {
            let len = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
            if len > 0.0 {
                normal[0] /= len;
                normal[1] /= len;
                normal[2] /= len;
            }
        }

        self.normals = Some(normals);
    }
}

impl Default for TerrainMesh {
    /// 返回一个空网格：无顶点/索引，包围球退化为原点零半径。
    fn default() -> Self {
        // 高度区间与包围球均置零，供后续细分时重新计算。
        Self {
            positions: Vec::new(),
            normals: None,
            tex_coords: None,
            indices: Vec::new(),
            minimum_height: 0.0,
            maximum_height: 0.0,
            bounding_sphere: BoundingSphere::new(glam::DVec3::ZERO, 0.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;

    #[test]
    fn test_vertex_count() {
        let mesh = TerrainMesh {
            positions: vec![[0.0; 3]; 4],
            ..Default::default()
        };
        assert_eq!(mesh.vertex_count(), 4);
    }

    #[test]
    fn test_triangle_count() {
        let mesh = TerrainMesh {
            positions: vec![[0.0; 3]; 4],
            indices: vec![0, 1, 2, 0, 2, 3],
            ..Default::default()
        };
        assert_eq!(mesh.triangle_count(), 2);
    }

    #[test]
    fn test_compute_normals() {
        // XY 平面中的简单三角形
        let mut mesh = TerrainMesh {
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            indices: vec![0, 1, 2],
            bounding_sphere: BoundingSphere::new(DVec3::ZERO, 1.0),
            ..Default::default()
        };

        mesh.compute_normals();

        assert!(mesh.normals.is_some());
        let normals = mesh.normals.unwrap();
        // 法线应指向 +Z 方向
        assert!((normals[0][2] - 1.0).abs() < 0.01);
    }
}
