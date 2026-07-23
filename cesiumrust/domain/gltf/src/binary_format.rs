//! GLB binary container and b3dm format parsing.
//!
//! Maps to CesiumJS:
//! - `Scene/GltfLoader.js` (GLB parsing)
//! - `Scene/Batched3DModel3DTileContent.js` (b3dm)
//!
//! # GLB Format
//! ```text
//! [12-byte header]
//!   magic: u32 (0x46546C67 = "glTF")
//!   version: u32 (2)
//!   length: u32 (total file length)
//! [chunks...]
//!   chunk_length: u32
//!   chunk_type: u32 (0x4E4F534A = JSON, 0x004E4942 = BIN)
//!   chunk_data: [u8; chunk_length]
//! ```
//!
//! # b3dm Format
//! ```text
//! [28-byte header]
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
//! [GLB data]
//! ```

use crate::gltf_model::GltfModel;
use thiserror::Error;

/// Errors that can occur during binary format parsing.
#[derive(Debug, Error)]
pub enum BinaryFormatError {
    /// Buffer too short for the header.
    #[error("Buffer too short: expected at least {expected} bytes, got {actual}")]
    BufferTooShort { expected: usize, actual: usize },

    /// Invalid magic number.
    #[error("Invalid magic: expected {expected:#010X}, got {actual:#010X}")]
    InvalidMagic { expected: u32, actual: u32 },

    /// Unsupported version.
    #[error("Unsupported version: {0}")]
    UnsupportedVersion(u32),

    /// Invalid chunk type.
    #[error("Invalid chunk type: {0:#010X}")]
    InvalidChunkType(u32),

    /// JSON parsing error.
    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Invalid string encoding.
    #[error("Invalid UTF-8: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}

/// GLB magic number: "glTF" in little-endian.
pub const GLB_MAGIC: u32 = 0x46546C67;

/// GLB chunk type for JSON content.
pub const GLB_CHUNK_JSON: u32 = 0x4E4F534A;

/// GLB chunk type for binary content.
pub const GLB_CHUNK_BIN: u32 = 0x004E4942;

/// b3dm magic: "b3dm" as bytes.
pub const B3DM_MAGIC: &[u8; 4] = b"b3dm";

/// A parsed GLB file.
#[derive(Debug, Clone)]
pub struct GlbData {
    /// The glTF JSON model.
    pub model: GltfModel,

    /// The binary buffer data (if present).
    pub binary_chunk: Option<Vec<u8>>,
}

impl GlbData {
    /// Parses a GLB file from bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, BinaryFormatError> {
        // Minimum header size: 12 bytes
        if data.len() < 12 {
            return Err(BinaryFormatError::BufferTooShort {
                expected: 12,
                actual: data.len(),
            });
        }

        let magic = read_u32_le(&data[0..4]);
        if magic != GLB_MAGIC {
            return Err(BinaryFormatError::InvalidMagic {
                expected: GLB_MAGIC,
                actual: magic,
            });
        }

        let version = read_u32_le(&data[4..8]);
        if version != 2 {
            return Err(BinaryFormatError::UnsupportedVersion(version));
        }

        let _total_length = read_u32_le(&data[8..12]);

        // Parse chunks
        let mut json_chunk: Option<Vec<u8>> = None;
        let mut binary_chunk: Option<Vec<u8>> = None;
        let mut offset = 12;

        while offset + 8 <= data.len() {
            let chunk_length = read_u32_le(&data[offset..offset + 4]) as usize;
            let chunk_type = read_u32_le(&data[offset + 4..offset + 8]);
            offset += 8;

            if offset + chunk_length > data.len() {
                break;
            }

            let chunk_data = data[offset..offset + chunk_length].to_vec();
            offset += chunk_length;

            match chunk_type {
                GLB_CHUNK_JSON => json_chunk = Some(chunk_data),
                GLB_CHUNK_BIN => binary_chunk = Some(chunk_data),
                _ => {
                    // Unknown chunk type, skip
                }
            }
        }

        let json_data = json_chunk.ok_or(BinaryFormatError::InvalidChunkType(0))?;
        let model = GltfModel::from_bytes(&json_data)?;

        Ok(Self {
            model,
            binary_chunk,
        })
    }

    /// Returns true if this GLB has embedded binary data.
    pub fn has_binary(&self) -> bool {
        self.binary_chunk.is_some()
    }
}

/// Feature table for b3dm (contains BATCH_LENGTH).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct B3dmFeatureTable {
    /// Number of batched features.
    #[serde(default)]
    pub batch_length: u32,

    /// Optional RTC (Relative-To-Center) center.
    #[serde(default)]
    pub rtc_center: Option<[f64; 3]>,
}

/// A parsed b3dm (Batched 3D Model) file.
#[derive(Debug, Clone)]
pub struct B3dmData {
    /// The feature table.
    pub feature_table: B3dmFeatureTable,

    /// Raw feature table binary data.
    pub feature_table_binary: Option<Vec<u8>>,

    /// Batch table JSON (arbitrary properties).
    pub batch_table_json: Option<serde_json::Value>,

    /// Raw batch table binary data.
    pub batch_table_binary: Option<Vec<u8>>,

    /// The embedded GLB data.
    pub glb: GlbData,
}

impl B3dmData {
    /// Parses a b3dm file from bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, BinaryFormatError> {
        // Minimum header size: 28 bytes
        if data.len() < 28 {
            return Err(BinaryFormatError::BufferTooShort {
                expected: 28,
                actual: data.len(),
            });
        }

        // Check magic
        if &data[0..4] != B3DM_MAGIC {
            let magic = read_u32_le(&data[0..4]);
            return Err(BinaryFormatError::InvalidMagic {
                expected: 0x6D643362, // "b3dm" as u32
                actual: magic,
            });
        }

        let version = read_u32_le(&data[4..8]);
        if version != 1 {
            return Err(BinaryFormatError::UnsupportedVersion(version));
        }

        let _byte_length = read_u32_le(&data[8..12]);
        let ft_json_length = read_u32_le(&data[12..16]) as usize;
        let ft_binary_length = read_u32_le(&data[16..20]) as usize;
        let bt_json_length = read_u32_le(&data[20..24]) as usize;
        let bt_binary_length = read_u32_le(&data[24..28]) as usize;

        let mut offset = 28;

        // Parse feature table JSON
        let feature_table = if ft_json_length > 0 {
            let ft_json_bytes = &data[offset..offset + ft_json_length];
            offset += ft_json_length;
            let ft_str = String::from_utf8(ft_json_bytes.to_vec())?;
            serde_json::from_str(ft_str.trim()).unwrap_or_default()
        } else {
            B3dmFeatureTable::default()
        };

        // Parse feature table binary
        let feature_table_binary = if ft_binary_length > 0 {
            let ft_bin = data[offset..offset + ft_binary_length].to_vec();
            offset += ft_binary_length;
            Some(ft_bin)
        } else {
            None
        };

        // Parse batch table JSON
        let batch_table_json = if bt_json_length > 0 {
            let bt_json_bytes = &data[offset..offset + bt_json_length];
            offset += bt_json_length;
            let bt_str = String::from_utf8(bt_json_bytes.to_vec())?;
            serde_json::from_str(bt_str.trim()).ok()
        } else {
            None
        };

        // Parse batch table binary
        let batch_table_binary = if bt_binary_length > 0 {
            let bt_bin = data[offset..offset + bt_binary_length].to_vec();
            offset += bt_binary_length;
            Some(bt_bin)
        } else {
            None
        };

        // Remaining data is GLB
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

    /// Returns the number of batched features.
    pub fn batch_length(&self) -> u32 {
        self.feature_table.batch_length
    }

    /// Returns the RTC center if present.
    pub fn rtc_center(&self) -> Option<[f64; 3]> {
        self.feature_table.rtc_center
    }
}

/// Reads a little-endian u32 from a byte slice.
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
        // Pad to 4-byte alignment
        let json_padded_length = (json_length + 3) & !3;

        let total_length = 12 + 8 + json_padded_length as usize;

        let mut data = Vec::with_capacity(total_length);

        // Header
        data.extend_from_slice(&GLB_MAGIC.to_le_bytes());
        data.extend_from_slice(&2u32.to_le_bytes());
        data.extend_from_slice(&(total_length as u32).to_le_bytes());

        // JSON chunk
        data.extend_from_slice(&json_padded_length.to_le_bytes());
        data.extend_from_slice(&GLB_CHUNK_JSON.to_le_bytes());
        data.extend_from_slice(json_bytes);
        // Padding
        for _ in json_length..json_padded_length {
            data.push(0x20); // Space padding for JSON
        }

        data
    }

    fn create_minimal_b3dm() -> Vec<u8> {
        let glb = create_minimal_glb();
        let ft_json = r#"{"BATCH_LENGTH":10}"#;
        let ft_json_bytes = ft_json.as_bytes();
        let ft_json_length = ft_json_bytes.len() as u32;

        let total_length = 28 + ft_json_length as usize + glb.len();

        let mut data = Vec::with_capacity(total_length);

        // Header
        data.extend_from_slice(B3DM_MAGIC);
        data.extend_from_slice(&1u32.to_le_bytes()); // version
        data.extend_from_slice(&(total_length as u32).to_le_bytes());
        data.extend_from_slice(&ft_json_length.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes()); // ft binary length
        data.extend_from_slice(&0u32.to_le_bytes()); // bt json length
        data.extend_from_slice(&0u32.to_le_bytes()); // bt binary length

        // Feature table JSON
        data.extend_from_slice(ft_json_bytes);

        // GLB
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

        // Header
        data.extend_from_slice(&GLB_MAGIC.to_le_bytes());
        data.extend_from_slice(&2u32.to_le_bytes());
        data.extend_from_slice(&(total_length as u32).to_le_bytes());

        // JSON chunk
        data.extend_from_slice(&json_padded_length.to_le_bytes());
        data.extend_from_slice(&GLB_CHUNK_JSON.to_le_bytes());
        data.extend_from_slice(json_bytes);
        for _ in json_length..json_padded_length {
            data.push(0x20);
        }

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
