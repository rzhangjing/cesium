//! Quantized-mesh terrain format decoder.
//!
//! Binary format specification:
//! - Header (88 bytes):
//!   - center: 3 x f64 (24 bytes)
//!   - minimumHeight: f32 (4 bytes)
//!   - maximumHeight: f32 (4 bytes)
//!   - boundingSphere: 4 x f64 (32 bytes)
//!   - horizonOcclusionPoint: 3 x f64 (24 bytes)
//! - Vertex data:
//!   - vertexCount: u32
//!   - u, v, height: vertexCount * 3 x u16 (zigzag delta encoded)
//! - Index data:
//!   - triangleCount: u32
//!   - indices: triangleCount * 3 x u16/u32 (high water mark encoded)
//! - Edge indices:
//!   - west/south/east/north vertex counts and indices
//! - Extensions (optional):
//!   - OCT_VERTEX_NORMALS (id=1)
//!   - WATER_MASK (id=2)
//!   - METADATA (id=4)

use cesium_geospatial::bounding::BoundingSphere;
use cesium_terrain::QuantizedMeshTerrainData;
use glam::DVec3;
use thiserror::Error;

/// Errors that can occur during quantized-mesh decoding.
#[derive(Debug, Error)]
pub enum QuantizedMeshError {
    #[error("Buffer too small: expected at least {expected} bytes, got {actual}")]
    BufferTooSmall { expected: usize, actual: usize },

    #[error("Invalid vertex count: {0}")]
    InvalidVertexCount(usize),

    #[error("Invalid triangle count: {0}")]
    InvalidTriangleCount(usize),

    #[error("Invalid index value: {0}")]
    InvalidIndex(u32),
}

/// Extension IDs for quantized-mesh format.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantizedMeshExtensionId {
    OctVertexNormals = 1,
    WaterMask = 2,
    Metadata = 4,
}

/// Header size in bytes.
const HEADER_SIZE: usize = 88;

/// Decodes a quantized-mesh terrain tile from binary data.
///
/// # Arguments
/// * `buffer` - The raw binary data
/// * `skirt_height` - The skirt height to use for the tile
///
/// # Returns
/// A `QuantizedMeshTerrainData` containing the decoded terrain data
pub fn decode_quantized_mesh(
    buffer: &[u8],
    skirt_height: f64,
) -> Result<QuantizedMeshTerrainData, QuantizedMeshError> {
    if buffer.len() < HEADER_SIZE {
        return Err(QuantizedMeshError::BufferTooSmall {
            expected: HEADER_SIZE,
            actual: buffer.len(),
        });
    }

    let mut pos = 0;

    // Parse header
    let _center = read_cartesian3(buffer, &mut pos);
    let minimum_height = read_f32(buffer, &mut pos) as f64;
    let maximum_height = read_f32(buffer, &mut pos) as f64;
    let bounding_sphere_center = read_cartesian3(buffer, &mut pos);
    let bounding_sphere_radius = read_f64(buffer, &mut pos);
    let horizon_occlusion_point = read_cartesian3(buffer, &mut pos);

    let bounding_sphere = BoundingSphere::new(bounding_sphere_center, bounding_sphere_radius);

    // Parse vertex data
    let vertex_count = read_u32(buffer, &mut pos) as usize;
    if vertex_count == 0 {
        return Err(QuantizedMeshError::InvalidVertexCount(0));
    }

    let vertex_buffer_size = vertex_count * 3 * 2; // 3 components * 2 bytes each
    if pos + vertex_buffer_size > buffer.len() {
        return Err(QuantizedMeshError::BufferTooSmall {
            expected: pos + vertex_buffer_size,
            actual: buffer.len(),
        });
    }

    // Read u, v, height buffers
    let mut u_buffer = Vec::with_capacity(vertex_count);
    let mut v_buffer = Vec::with_capacity(vertex_count);
    let mut height_buffer = Vec::with_capacity(vertex_count);

    for _ in 0..vertex_count {
        u_buffer.push(read_u16(buffer, &mut pos));
    }
    for _ in 0..vertex_count {
        v_buffer.push(read_u16(buffer, &mut pos));
    }
    for _ in 0..vertex_count {
        height_buffer.push(read_u16(buffer, &mut pos));
    }

    // Zigzag delta decode
    zigzag_delta_decode(&mut u_buffer);
    zigzag_delta_decode(&mut v_buffer);
    zigzag_delta_decode(&mut height_buffer);

    // Combine into quantized_vertices format [u0, u1, ..., v0, v1, ..., h0, h1, ...]
    let mut quantized_vertices = Vec::with_capacity(vertex_count * 3);
    quantized_vertices.extend_from_slice(&u_buffer);
    quantized_vertices.extend_from_slice(&v_buffer);
    quantized_vertices.extend_from_slice(&height_buffer);

    // Align to index size
    let bytes_per_index = if vertex_count > 64 * 1024 { 4 } else { 2 };
    if pos % bytes_per_index != 0 {
        pos += bytes_per_index - (pos % bytes_per_index);
    }

    // Parse triangle indices
    let triangle_count = read_u32(buffer, &mut pos) as usize;
    let index_count = triangle_count * 3;

    let mut indices = Vec::with_capacity(index_count);
    for _ in 0..index_count {
        let idx = if bytes_per_index == 4 {
            read_u32(buffer, &mut pos)
        } else {
            read_u16(buffer, &mut pos) as u32
        };
        indices.push(idx);
    }

    // High water mark decode
    high_water_mark_decode(&mut indices);

    // Parse edge indices
    let west_indices = read_edge_indices(buffer, &mut pos, bytes_per_index)?;
    let south_indices = read_edge_indices(buffer, &mut pos, bytes_per_index)?;
    let east_indices = read_edge_indices(buffer, &mut pos, bytes_per_index)?;
    let north_indices = read_edge_indices(buffer, &mut pos, bytes_per_index)?;

    // Parse extensions
    let mut encoded_normals = None;
    let mut water_mask = None;

    while pos < buffer.len() {
        if pos + 5 > buffer.len() {
            break;
        }

        let extension_id = buffer[pos];
        pos += 1;
        let extension_length = read_u32(buffer, &mut pos) as usize;

        if pos + extension_length > buffer.len() {
            break;
        }

        match extension_id {
            1 => {
                // OCT_VERTEX_NORMALS
                encoded_normals = Some(buffer[pos..pos + vertex_count * 2].to_vec());
            }
            2 => {
                // WATER_MASK
                water_mask = Some(buffer[pos..pos + extension_length].to_vec());
            }
            _ => {
                // Unknown extension, skip
            }
        }

        pos += extension_length;
    }

    Ok(QuantizedMeshTerrainData {
        quantized_vertices,
        indices,
        minimum_height,
        maximum_height,
        bounding_sphere,
        horizon_occlusion_point,
        west_indices,
        south_indices,
        east_indices,
        north_indices,
        west_skirt_height: skirt_height,
        south_skirt_height: skirt_height,
        east_skirt_height: skirt_height,
        north_skirt_height: skirt_height,
        child_tile_mask: 15,
        created_by_upsampling: false,
        encoded_normals,
        water_mask,
    })
}

/// Reads edge indices from the buffer.
fn read_edge_indices(
    buffer: &[u8],
    pos: &mut usize,
    bytes_per_index: usize,
) -> Result<Vec<u32>, QuantizedMeshError> {
    let count = read_u32(buffer, pos) as usize;
    let mut indices = Vec::with_capacity(count);

    for _ in 0..count {
        let idx = if bytes_per_index == 4 {
            read_u32(buffer, pos)
        } else {
            read_u16(buffer, pos) as u32
        };
        indices.push(idx);
    }

    Ok(indices)
}

/// Zigzag delta decodes a buffer in place.
///
/// The encoding stores differences between consecutive values using zigzag encoding
/// to efficiently represent both positive and negative deltas.
fn zigzag_delta_decode(buffer: &mut [u16]) {
    let mut value: u16 = 0;

    for item in buffer.iter_mut() {
        let encoded = *item;
        // Zigzag decode: (n >> 1) ^ -(n & 1)
        let delta = ((encoded >> 1) as i32) ^ -((encoded & 1) as i32);
        value = (value as i32 + delta) as u16;
        *item = value;
    }
}

/// High water mark decodes indices in place.
///
/// This is a compression technique where indices are stored as offsets from
/// a "high water mark" that increases when a new vertex is encountered.
fn high_water_mark_decode(indices: &mut [u32]) {
    let mut highest: u32 = 0;

    for idx in indices.iter_mut() {
        let code = *idx;
        *idx = highest - code;
        if code == 0 {
            highest += 1;
        }
    }
}

// Helper functions for reading binary data (little-endian)

fn read_u16(buffer: &[u8], pos: &mut usize) -> u16 {
    let value = u16::from_le_bytes([buffer[*pos], buffer[*pos + 1]]);
    *pos += 2;
    value
}

fn read_u32(buffer: &[u8], pos: &mut usize) -> u32 {
    let value = u32::from_le_bytes([
        buffer[*pos],
        buffer[*pos + 1],
        buffer[*pos + 2],
        buffer[*pos + 3],
    ]);
    *pos += 4;
    value
}

fn read_f32(buffer: &[u8], pos: &mut usize) -> f32 {
    let value = f32::from_le_bytes([
        buffer[*pos],
        buffer[*pos + 1],
        buffer[*pos + 2],
        buffer[*pos + 3],
    ]);
    *pos += 4;
    value
}

fn read_f64(buffer: &[u8], pos: &mut usize) -> f64 {
    let value = f64::from_le_bytes([
        buffer[*pos],
        buffer[*pos + 1],
        buffer[*pos + 2],
        buffer[*pos + 3],
        buffer[*pos + 4],
        buffer[*pos + 5],
        buffer[*pos + 6],
        buffer[*pos + 7],
    ]);
    *pos += 8;
    value
}

fn read_cartesian3(buffer: &[u8], pos: &mut usize) -> DVec3 {
    let x = read_f64(buffer, pos);
    let y = read_f64(buffer, pos);
    let z = read_f64(buffer, pos);
    DVec3::new(x, y, z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zigzag_delta_decode() {
        // Test zigzag encoding: 0 -> 0, 1 -> -1, 2 -> 1, 3 -> -2, 4 -> 2
        let mut buffer = vec![0, 2, 2, 2]; // 0, +1, +1, +1
        zigzag_delta_decode(&mut buffer);
        assert_eq!(buffer, vec![0, 1, 2, 3]);

        let mut buffer2 = vec![0, 1, 1, 1]; // 0, -1, -1, -1
        zigzag_delta_decode(&mut buffer2);
        assert_eq!(buffer2, vec![0, 65535, 65534, 65533]); // Wrapped around
    }

    #[test]
    fn test_high_water_mark_decode() {
        // Simple test: [0, 0, 0] -> [0, 1, 2]
        let mut indices = vec![0, 0, 0];
        high_water_mark_decode(&mut indices);
        assert_eq!(indices, vec![0, 1, 2]);

        // [0, 0, 1] -> [0, 1, 1] (third index references vertex 1)
        let mut indices2 = vec![0, 0, 1];
        high_water_mark_decode(&mut indices2);
        assert_eq!(indices2, vec![0, 1, 1]);
    }

    #[test]
    fn test_read_helpers() {
        let buffer = [0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04, 0x00];
        let mut pos = 0;
        assert_eq!(read_u16(&buffer, &mut pos), 1);
        assert_eq!(read_u16(&buffer, &mut pos), 2);
        assert_eq!(pos, 4);
    }

    #[test]
    fn test_buffer_too_small() {
        let buffer = [0u8; 10];
        let result = decode_quantized_mesh(&buffer, 100.0);
        assert!(matches!(result, Err(QuantizedMeshError::BufferTooSmall { .. })));
    }
}
