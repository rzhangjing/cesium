//! Terrain mesh representation.
//! Maps to CesiumJS `Core/TerrainMesh.js`

use cesium_geospatial::bounding::BoundingSphere;
use serde::{Deserialize, Serialize};

/// A mesh representing terrain geometry.
///
/// This is the output of terrain data processing - actual 3D positions
/// ready for rendering.
///
/// Maps to CesiumJS `TerrainMesh`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainMesh {
    /// Vertex positions in ECEF coordinates [x, y, z] per vertex
    pub positions: Vec<[f64; 3]>,

    /// Vertex normals (optional) [x, y, z] per vertex
    pub normals: Option<Vec<[f64; 3]>>,

    /// Texture coordinates [u, v] per vertex
    pub tex_coords: Option<Vec<[f64; 2]>>,

    /// Triangle indices
    pub indices: Vec<u32>,

    /// Minimum height in the mesh
    pub minimum_height: f64,

    /// Maximum height in the mesh
    pub maximum_height: f64,

    /// Bounding sphere for the mesh
    pub bounding_sphere: BoundingSphere,
}

impl TerrainMesh {
    /// Returns the number of vertices in the mesh.
    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    /// Returns the number of triangles in the mesh.
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Computes vertex normals from triangle faces if not present.
    pub fn compute_normals(&mut self) {
        if self.normals.is_some() {
            return;
        }

        let vertex_count = self.positions.len();
        let mut normals = vec![[0.0f64; 3]; vertex_count];

        // Accumulate face normals
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

            // Compute face normal
            let e1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
            let e2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];

            let normal = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];

            // Accumulate
            for &idx in tri {
                let idx = idx as usize;
                normals[idx][0] += normal[0];
                normals[idx][1] += normal[1];
                normals[idx][2] += normal[2];
            }
        }

        // Normalize
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
    fn default() -> Self {
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
        // Simple triangle in XY plane
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
        // Normal should point in +Z direction
        assert!((normals[0][2] - 1.0).abs() < 0.01);
    }
}
