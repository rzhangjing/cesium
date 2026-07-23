//! Quantized mesh terrain data.
//! Maps to CesiumJS `Core/QuantizedMeshTerrainData.js`

use cesium_geospatial::bounding::BoundingSphere;
use cesium_geospatial::cartographic::Cartographic;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::math_utils;
use cesium_geospatial::rectangle::Rectangle;
use glam::DVec3;
use serde::{Deserialize, Serialize};

use crate::terrain_mesh::TerrainMesh;
use crate::MAX_SHORT;

/// Terrain data for a single tile where the terrain is represented as a quantized mesh.
///
/// A quantized mesh consists of three vertex attributes: longitude (u), latitude (v),
/// and height. All attributes are expressed as 16-bit values in the range 0 to 32767.
///
/// - u: 0 at west edge, 32767 at east edge
/// - v: 0 at south edge, 32767 at north edge
/// - height: 0 at minimum height, 32767 at maximum height
///
/// Maps to CesiumJS `QuantizedMeshTerrainData`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizedMeshTerrainData {
    /// Quantized vertex data: [u0, u1, ..., v0, v1, ..., h0, h1, ...]
    /// Each component is a u16 in range [0, 32767]
    pub quantized_vertices: Vec<u16>,

    /// Triangle indices (u16 or u32 depending on vertex count)
    pub indices: Vec<u32>,

    /// Minimum terrain height in meters above the ellipsoid
    pub minimum_height: f64,

    /// Maximum terrain height in meters above the ellipsoid
    pub maximum_height: f64,

    /// Bounding sphere for the tile
    pub bounding_sphere: BoundingSphere,

    /// Horizon occlusion point in ellipsoid-scaled coordinates
    pub horizon_occlusion_point: DVec3,

    /// Indices of vertices on the western edge
    pub west_indices: Vec<u32>,

    /// Indices of vertices on the southern edge
    pub south_indices: Vec<u32>,

    /// Indices of vertices on the eastern edge
    pub east_indices: Vec<u32>,

    /// Indices of vertices on the northern edge
    pub north_indices: Vec<u32>,

    /// Skirt height on western edge
    pub west_skirt_height: f64,

    /// Skirt height on southern edge
    pub south_skirt_height: f64,

    /// Skirt height on eastern edge
    pub east_skirt_height: f64,

    /// Skirt height on northern edge
    pub north_skirt_height: f64,

    /// Bit mask indicating which children exist (bit 0=SW, 1=SE, 2=NW, 3=NE)
    #[serde(default = "default_child_tile_mask")]
    pub child_tile_mask: u8,

    /// Whether this was created by upsampling
    #[serde(default)]
    pub created_by_upsampling: bool,

    /// Oct-encoded normals (optional)
    #[serde(default)]
    pub encoded_normals: Option<Vec<u8>>,

    /// Water mask (optional)
    #[serde(default)]
    pub water_mask: Option<Vec<u8>>,
}

fn default_child_tile_mask() -> u8 {
    15 // All children exist by default
}

impl QuantizedMeshTerrainData {
    /// Returns the number of vertices in the mesh.
    pub fn vertex_count(&self) -> usize {
        self.quantized_vertices.len() / 3
    }

    /// Returns the u values (longitude quantization) for all vertices.
    pub fn u_values(&self) -> &[u16] {
        let count = self.vertex_count();
        &self.quantized_vertices[0..count]
    }

    /// Returns the v values (latitude quantization) for all vertices.
    pub fn v_values(&self) -> &[u16] {
        let count = self.vertex_count();
        &self.quantized_vertices[count..2 * count]
    }

    /// Returns the height values for all vertices.
    pub fn height_values(&self) -> &[u16] {
        let count = self.vertex_count();
        &self.quantized_vertices[2 * count..3 * count]
    }

    /// Checks if a specific child tile exists.
    ///
    /// # Arguments
    /// * `child` - Child index (0=SW, 1=SE, 2=NW, 3=NE)
    pub fn is_child_available(&self, child: usize) -> bool {
        (self.child_tile_mask & (1 << child)) != 0
    }

    /// Creates terrain mesh from quantized data.
    ///
    /// This is the main method that converts quantized mesh data into
    /// actual 3D positions using the tile rectangle and ellipsoid.
    ///
    /// Maps to CesiumJS `createVerticesFromQuantizedTerrainMesh`
    ///
    /// # Arguments
    /// * `rectangle` - The tile rectangle (west, south, east, north in radians)
    /// * `ellipsoid` - The ellipsoid to use for coordinate conversion
    /// * `exaggeration` - Vertical exaggeration factor (1.0 = no exaggeration)
    ///
    /// # Returns
    /// A TerrainMesh with actual 3D positions
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

            // Compute geodetic surface normal if exaggeration is applied
            if has_exaggeration {
                let normal = ellipsoid
                    .geodetic_surface_normal(position)
                    .unwrap_or(DVec3::Z);
                normals.push([normal.x, normal.y, normal.z]);
            }
        }

        // Decode oct-encoded normals if available
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

    /// Creates terrain mesh with skirts for seamless tile boundaries.
    ///
    /// # Arguments
    /// * `rectangle` - The tile rectangle
    /// * `ellipsoid` - The ellipsoid
    /// * `exaggeration` - Vertical exaggeration factor
    pub fn create_mesh_with_skirts(
        &self,
        rectangle: &Rectangle,
        ellipsoid: &Ellipsoid,
        exaggeration: f64,
    ) -> TerrainMesh {
        let mut mesh = self.create_mesh(rectangle, ellipsoid, exaggeration);

        // Add skirt vertices
        self.add_skirts(&mut mesh, rectangle, ellipsoid);

        mesh
    }

    /// Adds skirt vertices to the mesh for seamless boundaries.
    fn add_skirts(&self, mesh: &mut TerrainMesh, _rectangle: &Rectangle, ellipsoid: &Ellipsoid) {
        let base_vertex_count = mesh.positions.len();

        // Helper to add skirt for an edge
        let mut add_edge_skirt = |edge_indices: &[u32], skirt_height: f64| {
            for &idx in edge_indices {
                let idx = idx as usize;
                if idx < base_vertex_count {
                    let pos = mesh.positions[idx];
                    let position = DVec3::new(pos[0], pos[1], pos[2]);

                    // Get the cartographic coordinates
                    if let Some(carto) = ellipsoid.cartesian_to_cartographic(position) {
                        // Lower the height by skirt amount
                        let skirt_carto = Cartographic::from_radians(
                            carto.longitude,
                            carto.latitude,
                            carto.height - skirt_height,
                        );
                        let skirt_pos = ellipsoid.cartographic_to_cartesian(&skirt_carto);
                        mesh.positions.push([skirt_pos.x, skirt_pos.y, skirt_pos.z]);

                        // Copy UV and normal
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

        // Add skirts for each edge
        add_edge_skirt(&self.west_indices, self.west_skirt_height);
        add_edge_skirt(&self.south_indices, self.south_skirt_height);
        add_edge_skirt(&self.east_indices, self.east_skirt_height);
        add_edge_skirt(&self.north_indices, self.north_skirt_height);

        // Add skirt triangles
        let mut add_skirt_indices = |edge_indices: &[u32], offset: usize| {
            for i in 0..edge_indices.len().saturating_sub(1) {
                let v0 = edge_indices[i];
                let v1 = edge_indices[i + 1];
                let v2 = (offset + i) as u32;
                let v3 = (offset + i + 1) as u32;

                // Two triangles for the skirt quad
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

/// Decodes oct-encoded normals.
///
/// Oct encoding maps a unit vector to two bytes using an octahedral projection.
/// Maps to CesiumJS `AttributeCompression.octDecode`
fn decode_oct_normals(encoded: &[u8], vertex_count: usize) -> Vec<[f64; 3]> {
    let mut normals = Vec::with_capacity(vertex_count);

    for i in 0..vertex_count {
        let x = encoded.get(i * 2).copied().unwrap_or(128);
        let y = encoded.get(i * 2 + 1).copied().unwrap_or(128);

        // Decode from [0, 255] to [-1, 1]
        let mut decoded_x = (x as f64 / 255.0) * 2.0 - 1.0;
        let mut decoded_y = (y as f64 / 255.0) * 2.0 - 1.0;

        // Oct decode
        let z = 1.0 - decoded_x.abs() - decoded_y.abs();
        if z < 0.0 {
            let old_x = decoded_x;
            decoded_x = (1.0 - decoded_y.abs()) * old_x.signum();
            decoded_y = (1.0 - old_x.abs()) * decoded_y.signum();
        }

        // Normalize
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
        // Simple 4-vertex quad (SW, NW, SE, NE)
        QuantizedMeshTerrainData {
            quantized_vertices: vec![
                // u values
                0, 0, 32767, 32767,
                // v values
                0, 32767, 0, 32767,
                // height values
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

        // Should have more vertices due to skirts
        assert!(mesh.positions.len() > 4);
        // Should have more indices due to skirt triangles
        assert!(mesh.indices.len() > 6);
    }

    #[test]
    fn test_decode_oct_normals() {
        // Test with encoded normal pointing up (128, 128 = center)
        let encoded = vec![128, 128];
        let normals = decode_oct_normals(&encoded, 1);

        assert_eq!(normals.len(), 1);
        // Should be approximately [0, 0, 1]
        assert!((normals[0][2] - 1.0).abs() < 0.1);
    }
}
