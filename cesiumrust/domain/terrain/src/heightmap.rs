//! Heightmap terrain data.
//! Maps to CesiumJS `Core/HeightmapTerrainData.js`

use cesium_geospatial::bounding::BoundingSphere;
use cesium_geospatial::cartographic::Cartographic;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::math_utils;
use cesium_geospatial::rectangle::Rectangle;
use glam::DVec3;
use serde::{Deserialize, Serialize};

use crate::terrain_mesh::TerrainMesh;

/// Terrain data represented as a heightmap.
///
/// A heightmap is a regular grid of height values covering a rectangular region.
///
/// Maps to CesiumJS `HeightmapTerrainData`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeightmapTerrainData {
    /// Height values in row-major order (south to north, west to east)
    pub heights: Vec<f64>,

    /// Number of rows (latitude samples)
    pub width: usize,

    /// Number of columns (longitude samples)
    pub height: usize,

    /// Minimum height in the tile
    pub minimum_height: f64,

    /// Maximum height in the tile
    pub maximum_height: f64,

    /// Bounding sphere for the tile
    pub bounding_sphere: BoundingSphere,

    /// Bit mask indicating which children exist
    #[serde(default = "default_child_mask")]
    pub child_tile_mask: u8,

    /// Whether this was created by upsampling
    #[serde(default)]
    pub created_by_upsampling: bool,
}

fn default_child_mask() -> u8 {
    15
}

impl HeightmapTerrainData {
    /// Creates a new heightmap terrain data.
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

    /// Gets the height at a specific grid position.
    pub fn get_height(&self, col: usize, row: usize) -> Option<f64> {
        if col < self.width && row < self.height {
            Some(self.heights[row * self.width + col])
        } else {
            None
        }
    }

    /// Interpolates height at a fractional grid position.
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

        // Bilinear interpolation
        let h0 = math_utils::lerp(h00, h10, du);
        let h1 = math_utils::lerp(h01, h11, du);
        math_utils::lerp(h0, h1, dv)
    }

    /// Creates a terrain mesh from the heightmap.
    ///
    /// # Arguments
    /// * `rectangle` - The tile rectangle
    /// * `ellipsoid` - The ellipsoid
    pub fn create_mesh(&self, rectangle: &Rectangle, ellipsoid: &Ellipsoid) -> TerrainMesh {
        let mut positions = Vec::with_capacity(self.width * self.height);
        let mut uvs = Vec::with_capacity(self.width * self.height);
        let mut indices = Vec::new();

        // Generate vertices
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

        // Generate indices
        for row in 0..self.height - 1 {
            for col in 0..self.width - 1 {
                let i0 = (row * self.width + col) as u32;
                let i1 = i0 + 1;
                let i2 = i0 + self.width as u32;
                let i3 = i2 + 1;

                // Two triangles per quad
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

    /// Checks if a specific child tile exists.
    pub fn is_child_available(&self, child: usize) -> bool {
        (self.child_tile_mask & (1 << child)) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_heightmap() -> HeightmapTerrainData {
        // 3x3 heightmap
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
        // Center should be 200
        assert!((data.interpolate_height(0.5, 0.5) - 200.0).abs() < 0.01);
        // Corner should be 0
        assert!((data.interpolate_height(0.0, 0.0) - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_create_mesh() {
        let data = create_test_heightmap();
        let rectangle = Rectangle::from_degrees(-1.0, -1.0, 1.0, 1.0);
        let ellipsoid = Ellipsoid::WGS84;

        let mesh = data.create_mesh(&rectangle, &ellipsoid);

        assert_eq!(mesh.positions.len(), 9); // 3x3
        assert_eq!(mesh.indices.len(), 24); // 4 quads * 2 triangles * 3 indices
        assert!(mesh.normals.is_some());
    }

    #[test]
    fn test_child_availability() {
        let data = create_test_heightmap();
        assert!(data.is_child_available(0));
        assert!(data.is_child_available(3));
    }
}
