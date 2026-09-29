//! Quantized-mesh 地形格式解码器。
//!
//! 二进制格式规范：
//! - 头部（88 字节）：
//!   - center：3 x f64（24 字节）
//!   - minimumHeight：f32（4 字节）
//!   - maximumHeight：f32（4 字节）
//!   - boundingSphere：4 x f64（32 字节）
//!   - horizonOcclusionPoint：3 x f64（24 字节）
//! - 顶点数据：
//!   - vertexCount：u32
//!   - u、v、height：vertexCount * 3 x u16（zigzag 增量编码）
//! - 索引数据：
//!   - triangleCount：u32
//!   - indices：triangleCount * 3 x u16/u32（高水位标记编码）
//! - 边缘索引：
//!   - 西/南/东/北 顶点数量与索引
//! - 扩展（可选）：
//!   - OCT_VERTEX_NORMALS（id=1）
//!   - WATER_MASK（id=2）
//!   - METADATA（id=4）

use cesium_geospatial::bounding::BoundingSphere;
use cesium_terrain::QuantizedMeshTerrainData;
use glam::DVec3;
use thiserror::Error;

/// Quantized-mesh 解码过程中可能出现的错误。
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

/// Quantized-mesh 格式的扩展 ID。
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantizedMeshExtensionId {
    OctVertexNormals = 1,
    WaterMask = 2,
    Metadata = 4,
}

/// 头部大小（字节）。
const HEADER_SIZE: usize = 88;

/// 从二进制数据解码一个 quantized-mesh 地形瓦片。
///
/// # 参数
/// * `buffer` - 原始二进制数据
/// * `skirt_height` - 该瓦片使用的裙边高度
///
/// # 返回
/// 包含已解码地形数据的 `QuantizedMeshTerrainData`
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

    // 解析头部
    let _center = read_cartesian3(buffer, &mut pos);
    let minimum_height = read_f32(buffer, &mut pos) as f64;
    let maximum_height = read_f32(buffer, &mut pos) as f64;
    let bounding_sphere_center = read_cartesian3(buffer, &mut pos);
    let bounding_sphere_radius = read_f64(buffer, &mut pos);
    let horizon_occlusion_point = read_cartesian3(buffer, &mut pos);

    let bounding_sphere = BoundingSphere::new(bounding_sphere_center, bounding_sphere_radius);

    // 解析顶点数据
    let vertex_count = read_u32(buffer, &mut pos) as usize;
    if vertex_count == 0 {
        return Err(QuantizedMeshError::InvalidVertexCount(0));
    }

    let vertex_buffer_size = vertex_count * 3 * 2; // 3 个分量 * 每个 2 字节
    if pos + vertex_buffer_size > buffer.len() {
        return Err(QuantizedMeshError::BufferTooSmall {
            expected: pos + vertex_buffer_size,
            actual: buffer.len(),
        });
    }

    // 读取 u、v、height 缓冲区
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

    // Zigzag 增量解码
    zigzag_delta_decode(&mut u_buffer);
    zigzag_delta_decode(&mut v_buffer);
    zigzag_delta_decode(&mut height_buffer);

    // 合并为 quantized_vertices 格式 [u0, u1, ..., v0, v1, ..., h0, h1, ...]
    let mut quantized_vertices = Vec::with_capacity(vertex_count * 3);
    quantized_vertices.extend_from_slice(&u_buffer);
    quantized_vertices.extend_from_slice(&v_buffer);
    quantized_vertices.extend_from_slice(&height_buffer);

    // 对齐到索引大小
    let bytes_per_index = if vertex_count > 64 * 1024 { 4 } else { 2 };
    if pos % bytes_per_index != 0 {
        pos += bytes_per_index - (pos % bytes_per_index);
    }

    // 解析三角形索引
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

    // 高水位标记解码
    high_water_mark_decode(&mut indices);

    // 解析边缘索引
    let west_indices = read_edge_indices(buffer, &mut pos, bytes_per_index)?;
    let south_indices = read_edge_indices(buffer, &mut pos, bytes_per_index)?;
    let east_indices = read_edge_indices(buffer, &mut pos, bytes_per_index)?;
    let north_indices = read_edge_indices(buffer, &mut pos, bytes_per_index)?;

    // 解析扩展
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
                // 未知扩展，跳过
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

/// 从缓冲区读取边缘索引。
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

/// 就地对一个缓冲区进行 zigzag 增量解码。
///
/// 该编码使用 zigzag 编码存储相邻值之间的差，从而高效地
/// 表示正负增量。
fn zigzag_delta_decode(buffer: &mut [u16]) {
    let mut value: u16 = 0;

    for item in buffer.iter_mut() {
        let encoded = *item;
        // Zigzag 解码：(n >> 1) ^ -(n & 1)
        let delta = ((encoded >> 1) as i32) ^ -((encoded & 1) as i32);
        value = (value as i32 + delta) as u16;
        *item = value;
    }
}

/// 就地对索引进行高水位标记解码。
///
/// 这是一种压缩技术：索引以相对于“高水位标记”的偏移量存储，
/// 当遇到新顶点时该标记会递增。
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

// 读取二进制数据的辅助函数（小端序）

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
        // 测试 zigzag 编码：0 -> 0, 1 -> -1, 2 -> 1, 3 -> -2, 4 -> 2
        let mut buffer = vec![0, 2, 2, 2]; // 0, +1, +1, +1
        zigzag_delta_decode(&mut buffer);
        assert_eq!(buffer, vec![0, 1, 2, 3]);

        let mut buffer2 = vec![0, 1, 1, 1]; // 0, -1, -1, -1
        zigzag_delta_decode(&mut buffer2);
        assert_eq!(buffer2, vec![0, 65535, 65534, 65533]); // 发生了回绕
    }

    #[test]
    fn test_high_water_mark_decode() {
        // 简单测试：[0, 0, 0] -> [0, 1, 2]
        let mut indices = vec![0, 0, 0];
        high_water_mark_decode(&mut indices);
        assert_eq!(indices, vec![0, 1, 2]);

        // [0, 0, 1] -> [0, 1, 1]（第三个索引引用顶点 1）
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
