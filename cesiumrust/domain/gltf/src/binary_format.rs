//! GLB 二进制容器与 b3dm 格式解析。
//!
//! 提供 GLB 二进制容器解析与 b3dm（Batched 3D Model）内容拆解：
//! - GLB 容器解析（JSON chunk + BIN chunk）
//! - b3dm 的 feature table / batch table / 内嵌 GLB
//!
//! # GLB 格式
//! ```text
//! [12 字节 header]
//!   magic: u32 (0x46546C67 = "glTF")
//!   version: u32 (2)
//!   length: u32（文件总长度）
//! [chunks...]
//!   chunk_length: u32
//!   chunk_type: u32 (0x4E4F534A = JSON, 0x004E4942 = BIN)
//!   chunk_data: [u8; chunk_length]
//! ```
//!
//! # b3dm 格式
//! ```text
//! [28 字节 header]
//!   magic: [u8; 4] ("b3dm")
//!   version: u32 (1)
//!   byte_length: u32
//!   feature_table_json_byte_length: u32
//!   feature_table_binary_byte_length: u32
//!   batch_table_json_byte_length: u32
//!   batch_table_binary_byte_length: u32
//! [feature table JSON]
//! [feature table binary]
//! [batch table JSON]
//! [batch table binary]
//! [GLB 数据]
//! ```

use crate::gltf_model::GltfModel;
use thiserror::Error;

/// 二进制格式解析期间可能发生的错误。
#[derive(Debug, Error)]
pub enum BinaryFormatError {
    /// 用于 header 的 buffer 太短。
    #[error("Buffer too short: expected at least {expected} bytes, got {actual}")]
    BufferTooShort { expected: usize, actual: usize },

    /// 无效的 magic 数。
    #[error("Invalid magic: expected {expected:#010X}, got {actual:#010X}")]
    InvalidMagic { expected: u32, actual: u32 },

    /// 不支持的版本。
    #[error("Unsupported version: {0}")]
    UnsupportedVersion(u32),

    /// 无效的 chunk 类型。
    #[error("Invalid chunk type: {0:#010X}")]
    InvalidChunkType(u32),

    /// JSON 解析错误。
    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// 无效的字符串编码。
    #[error("Invalid UTF-8: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}

/// GLB magic 数：小端序的 "glTF"。
pub const GLB_MAGIC: u32 = 0x46546C67;

/// JSON 内容的 GLB chunk 类型。
pub const GLB_CHUNK_JSON: u32 = 0x4E4F534A;

/// 二进制内容的 GLB chunk 类型。
pub const GLB_CHUNK_BIN: u32 = 0x004E4942;

/// b3dm magic：作为字节的 "b3dm"。
pub const B3DM_MAGIC: &[u8; 4] = b"b3dm";

/// 一个已解析的 GLB 文件。
#[derive(Debug, Clone)]
pub struct GlbData {
    /// glTF JSON model。
    pub model: GltfModel,

    /// 二进制 buffer 数据（若存在）。
    pub binary_chunk: Option<Vec<u8>>,
}

/// 从 GLB 容器提取出的 `(json_chunk, binary_chunk)` 对。
type GlbChunks = (Option<Vec<u8>>, Option<Vec<u8>>);

/// 遍历 GLB header + chunks，返回 `(json_chunk, binary_chunk)`
/// 而不反序列化 JSON。由 [`GlbData::from_bytes`]（强类型）
/// 与 [`parse_glb_container`]（无类型，用于 glTF 1.0 → 2.0 升级路径）共享。
fn read_glb_chunks(data: &[u8]) -> Result<GlbChunks, BinaryFormatError> {
    // header 最小尺寸：12 字节
    if data.len() < 12 {
        return Err(BinaryFormatError::BufferTooShort {
            expected: 12,
            actual: data.len(),
        });
    }

    // 读取 4 字节 magic，必须等于 "glTF" 小端表示
    let magic = read_u32_le(&data[0..4]);
    if magic != GLB_MAGIC {
        return Err(BinaryFormatError::InvalidMagic {
            expected: GLB_MAGIC,
            actual: magic,
        });
    }

    // GLB 版本字段，本解析器仅接受 v2
    let version = read_u32_le(&data[4..8]);
    if version != 2 {
        return Err(BinaryFormatError::UnsupportedVersion(version));
    }

    // 文件总长度（声明值）：分块遍历以实际字节为准，故此处忽略
    let _total_length = read_u32_le(&data[8..12]);

    // 解析 chunks
    let mut json_chunk: Option<Vec<u8>> = None;
    let mut binary_chunk: Option<Vec<u8>> = None;
    let mut offset = 12;

    // 逐块迭代：每块有 8 字节头（长度 + 类型），后接 chunk_length 字节数据
    while offset + 8 <= data.len() {
        let chunk_length = read_u32_le(&data[offset..offset + 4]) as usize;
        let chunk_type = read_u32_le(&data[offset + 4..offset + 8]);
        offset += 8;

        // 块声明长度超出 buffer 实际末尾：视为截断，停止遍历
        if offset + chunk_length > data.len() {
            break;
        }

        let chunk_data = data[offset..offset + chunk_length].to_vec();
        offset += chunk_length;

        // 按类型分派：JSON 块与 BIN 块各存一份，其余忽略
        match chunk_type {
            GLB_CHUNK_JSON => json_chunk = Some(chunk_data),
            GLB_CHUNK_BIN => binary_chunk = Some(chunk_data),
            _ => {
                // 未知 chunk 类型，跳过
            }
        }
    }

    Ok((json_chunk, binary_chunk))
}

/// 将 GLB 容器解析为其原始 JSON [`serde_json::Value`] + 二进制
/// chunk，**不**进行 [`GlbData::from_bytes`] 所执行的强类型 [`GltfModel`]
/// 反序列化。这是 glTF 1.0 → 2.0 升级路径的入口点：
/// 一个 1.0 的 JSON payload（对象键集合）
/// 在 [`crate::gltf_upgrade::update_version_with_buffers`] 运行之前无法被反序列化为基于数组的强类型 model。
///
/// # 错误
/// 返回与 [`GlbData::from_bytes`] 相同的 header/chunk 错误，外加当 JSON
/// chunk 缺失或格式错误时的 JSON 解析错误。
pub fn parse_glb_container(
    data: &[u8],
) -> Result<(serde_json::Value, Option<Vec<u8>>), BinaryFormatError> {
    // 复用统一的 chunk 遍历，仅取出 JSON 块解析为无类型 Value
    let (json_chunk, binary_chunk) = read_glb_chunks(data)?;
    // GLB 必须含 JSON 块，缺失时用 InvalidChunkType(0) 表示
    let json_data = json_chunk.ok_or(BinaryFormatError::InvalidChunkType(0))?;
    let value = serde_json::from_slice(&json_data)?;
    Ok((value, binary_chunk))
}

impl GlbData {
    /// 从字节解析一个 GLB 文件。
    pub fn from_bytes(data: &[u8]) -> Result<Self, BinaryFormatError> {
        // 取出 JSON 块后直接反序列化为强类型 model（要求 JSON 为 glTF 2.0）
        let (json_chunk, binary_chunk) = read_glb_chunks(data)?;
        let json_data = json_chunk.ok_or(BinaryFormatError::InvalidChunkType(0))?;
        let model = GltfModel::from_bytes(&json_data)?;

        Ok(Self {
            model,
            binary_chunk,
        })
    }

    /// 若此 GLB 包含嵌入式二进制数据则返回 true。
    pub fn has_binary(&self) -> bool {
        self.binary_chunk.is_some()
    }
}

/// b3dm 的 feature table（包含 BATCH_LENGTH）。
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct B3dmFeatureTable {
    /// 批量 feature 的数量。
    #[serde(default)]
    pub batch_length: u32,

    /// 可选的 RTC（Relative-To-Center）中心。
    #[serde(default)]
    pub rtc_center: Option<[f64; 3]>,
}

/// 一个已解析的 b3dm（Batched 3D Model）文件。
#[derive(Debug, Clone)]
pub struct B3dmData {
    /// feature table。
    pub feature_table: B3dmFeatureTable,

    /// 原始的 feature table 二进制数据。
    pub feature_table_binary: Option<Vec<u8>>,

    /// batch table JSON（任意属性）。
    pub batch_table_json: Option<serde_json::Value>,

    /// 原始的 batch table 二进制数据。
    pub batch_table_binary: Option<Vec<u8>>,

    /// 嵌入的 GLB 数据。
    pub glb: GlbData,
}

impl B3dmData {
    /// 从字节解析一个 b3dm 文件。
    pub fn from_bytes(data: &[u8]) -> Result<Self, BinaryFormatError> {
        // header 最小尺寸：28 字节
        if data.len() < 28 {
            return Err(BinaryFormatError::BufferTooShort {
                expected: 28,
                actual: data.len(),
            });
        }

        // 检查 magic
        if &data[0..4] != B3DM_MAGIC {
            let magic = read_u32_le(&data[0..4]);
            return Err(BinaryFormatError::InvalidMagic {
                expected: 0x6D643362, // "b3dm" 作为 u32
                actual: magic,
            });
        }

        // b3dm 版本字段，本解析器仅接受 v1
        let version = read_u32_le(&data[4..8]);
        if version != 1 {
            return Err(BinaryFormatError::UnsupportedVersion(version));
        }

        // 后续 4 个字段依次给出各段字节长度，用于顺序切片
        let _byte_length = read_u32_le(&data[8..12]);
        let ft_json_length = read_u32_le(&data[12..16]) as usize;
        let ft_binary_length = read_u32_le(&data[16..20]) as usize;
        let bt_json_length = read_u32_le(&data[20..24]) as usize;
        let bt_binary_length = read_u32_le(&data[24..28]) as usize;

        let mut offset = 28;

        // 解析 feature table JSON
        let feature_table = if ft_json_length > 0 {
            let ft_json_bytes = &data[offset..offset + ft_json_length];
            offset += ft_json_length;
            let ft_str = String::from_utf8(ft_json_bytes.to_vec())?;
            serde_json::from_str(ft_str.trim()).unwrap_or_default()
        } else {
            B3dmFeatureTable::default()
        };

        // 解析 feature table 二进制
        let feature_table_binary = if ft_binary_length > 0 {
            let ft_bin = data[offset..offset + ft_binary_length].to_vec();
            offset += ft_binary_length;
            Some(ft_bin)
        } else {
            None
        };

        // 解析 batch table JSON
        let batch_table_json = if bt_json_length > 0 {
            let bt_json_bytes = &data[offset..offset + bt_json_length];
            offset += bt_json_length;
            let bt_str = String::from_utf8(bt_json_bytes.to_vec())?;
            serde_json::from_str(bt_str.trim()).ok()
        } else {
            None
        };

        // 解析 batch table 二进制
        let batch_table_binary = if bt_binary_length > 0 {
            let bt_bin = data[offset..offset + bt_binary_length].to_vec();
            offset += bt_binary_length;
            Some(bt_bin)
        } else {
            None
        };

        // 剩余数据是 GLB
        let glb_data = &data[offset..];
        let glb = GlbData::from_bytes(glb_data)?;

        Ok(Self {
            feature_table,
            feature_table_binary,
            batch_table_json,
            batch_table_binary,
            glb,
        })
    }

    /// 返回批量 feature 的数量。
    pub fn batch_length(&self) -> u32 {
        self.feature_table.batch_length
    }

    /// 若存在则返回 RTC 中心。
    pub fn rtc_center(&self) -> Option<[f64; 3]> {
        self.feature_table.rtc_center
    }
}

/// 从字节切片读取一个小端序 u32。
fn read_u32_le(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_minimal_glb() -> Vec<u8> {
        let json = r#"{"asset":{"version":"2.0"}}"#;
        let json_bytes = json.as_bytes();
        let json_length = json_bytes.len() as u32;
        // 填充到 4 字节对齐
        let json_padded_length = (json_length + 3) & !3;

        let total_length = 12 + 8 + json_padded_length as usize;

        let mut data = Vec::with_capacity(total_length);

        // header
        data.extend_from_slice(&GLB_MAGIC.to_le_bytes());
        data.extend_from_slice(&2u32.to_le_bytes());
        data.extend_from_slice(&(total_length as u32).to_le_bytes());

        // JSON chunk
        data.extend_from_slice(&json_padded_length.to_le_bytes());
        data.extend_from_slice(&GLB_CHUNK_JSON.to_le_bytes());
        data.extend_from_slice(json_bytes);
        // 填充
        data.extend(std::iter::repeat_n(0x20u8, (json_padded_length - json_length) as usize));

        data
    }

    fn create_minimal_b3dm() -> Vec<u8> {
        let glb = create_minimal_glb();
        let ft_json = r#"{"BATCH_LENGTH":10}"#;
        let ft_json_bytes = ft_json.as_bytes();
        let ft_json_length = ft_json_bytes.len() as u32;

        let total_length = 28 + ft_json_length as usize + glb.len();

        let mut data = Vec::with_capacity(total_length);

        // header
        data.extend_from_slice(B3DM_MAGIC);
        data.extend_from_slice(&1u32.to_le_bytes()); // 版本
        data.extend_from_slice(&(total_length as u32).to_le_bytes());
        data.extend_from_slice(&ft_json_length.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes()); // ft 二进制长度
        data.extend_from_slice(&0u32.to_le_bytes()); // bt json 长度
        data.extend_from_slice(&0u32.to_le_bytes()); // bt 二进制长度

        // feature table JSON
        data.extend_from_slice(ft_json_bytes);

        // GLB 数据
        data.extend_from_slice(&glb);

        data
    }

    #[test]
    fn test_glb_magic_validation() {
        let data = vec![0u8; 12];
        let result = GlbData::from_bytes(&data);
        assert!(matches!(result, Err(BinaryFormatError::InvalidMagic { .. })));
    }

    #[test]
    fn test_glb_buffer_too_short() {
        let data = vec![0u8; 8];
        let result = GlbData::from_bytes(&data);
        assert!(matches!(result, Err(BinaryFormatError::BufferTooShort { .. })));
    }

    #[test]
    fn test_glb_parse_minimal() {
        let data = create_minimal_glb();
        let glb = GlbData::from_bytes(&data).unwrap();

        assert_eq!(glb.model.asset.version, "2.0");
        assert!(!glb.has_binary());
    }

    #[test]
    fn test_glb_with_binary_chunk() {
        let json = r#"{"asset":{"version":"2.0"},"buffers":[{"byteLength":4}]}"#;
        let json_bytes = json.as_bytes();
        let json_length = json_bytes.len() as u32;
        let json_padded_length = (json_length + 3) & !3;

        let bin_data: [u8; 4] = [1, 2, 3, 4];
        let bin_length = bin_data.len() as u32;

        let total_length = 12 + 8 + json_padded_length as usize + 8 + bin_length as usize;

        let mut data = Vec::with_capacity(total_length);

        // header
        data.extend_from_slice(&GLB_MAGIC.to_le_bytes());
        data.extend_from_slice(&2u32.to_le_bytes());
        data.extend_from_slice(&(total_length as u32).to_le_bytes());

        // JSON chunk
        data.extend_from_slice(&json_padded_length.to_le_bytes());
        data.extend_from_slice(&GLB_CHUNK_JSON.to_le_bytes());
        data.extend_from_slice(json_bytes);
        data.extend(std::iter::repeat_n(0x20u8, (json_padded_length - json_length) as usize));

        // BIN chunk
        data.extend_from_slice(&bin_length.to_le_bytes());
        data.extend_from_slice(&GLB_CHUNK_BIN.to_le_bytes());
        data.extend_from_slice(&bin_data);

        let glb = GlbData::from_bytes(&data).unwrap();
        assert!(glb.has_binary());
        assert_eq!(glb.binary_chunk.unwrap(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_b3dm_parse_minimal() {
        let data = create_minimal_b3dm();
        let b3dm = B3dmData::from_bytes(&data).unwrap();

        assert_eq!(b3dm.batch_length(), 10);
        assert_eq!(b3dm.glb.model.asset.version, "2.0");
    }

    #[test]
    fn test_b3dm_magic_validation() {
        let data = vec![0u8; 28];
        let result = B3dmData::from_bytes(&data);
        assert!(matches!(result, Err(BinaryFormatError::InvalidMagic { .. })));
    }

    #[test]
    fn test_b3dm_with_rtc_center() {
        let glb = create_minimal_glb();
        let ft_json = r#"{"BATCH_LENGTH":5,"RTC_CENTER":[1.0,2.0,3.0]}"#;
        let ft_json_bytes = ft_json.as_bytes();
        let ft_json_length = ft_json_bytes.len() as u32;

        let total_length = 28 + ft_json_length as usize + glb.len();

        let mut data = Vec::with_capacity(total_length);
        data.extend_from_slice(B3DM_MAGIC);
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&(total_length as u32).to_le_bytes());
        data.extend_from_slice(&ft_json_length.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(ft_json_bytes);
        data.extend_from_slice(&glb);

        let b3dm = B3dmData::from_bytes(&data).unwrap();
        assert_eq!(b3dm.batch_length(), 5);
        assert_eq!(b3dm.rtc_center(), Some([1.0, 2.0, 3.0]));
    }

    #[test]
    fn test_b3dm_with_batch_table() {
        let glb = create_minimal_glb();
        let ft_json = r#"{"BATCH_LENGTH":2}"#;
        let bt_json = r#"{"name":["Building A","Building B"],"height":[10.5,20.3]}"#;
        let ft_json_bytes = ft_json.as_bytes();
        let bt_json_bytes = bt_json.as_bytes();
        let ft_json_length = ft_json_bytes.len() as u32;
        let bt_json_length = bt_json_bytes.len() as u32;

        let total_length = 28 + ft_json_length as usize + bt_json_length as usize + glb.len();

        let mut data = Vec::with_capacity(total_length);
        data.extend_from_slice(B3DM_MAGIC);
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&(total_length as u32).to_le_bytes());
        data.extend_from_slice(&ft_json_length.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&bt_json_length.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(ft_json_bytes);
        data.extend_from_slice(bt_json_bytes);
        data.extend_from_slice(&glb);

        let b3dm = B3dmData::from_bytes(&data).unwrap();
        assert!(b3dm.batch_table_json.is_some());

        let bt = b3dm.batch_table_json.unwrap();
        assert_eq!(bt["name"][0], "Building A");
        assert_eq!(bt["height"][1], 20.3);
    }
}
