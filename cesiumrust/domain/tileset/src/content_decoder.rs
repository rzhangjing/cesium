//! 3D Tiles binary content decoding (b3dm, i3dm, pnts, cmpt).
//!
//! Maps to CesiumJS:
//! - `Scene/B3dmParser.js`
//! - `Scene/I3dmParser.js`
//! - `Scene/PntsParser.js`
//! - `Scene/Composite3DTileContent.js`
//! - `Scene/Cesium3DTileContentType.js`

use serde_json::Value;

/// The type of 3D Tile content, identified by the magic bytes.
///
/// Maps to CesiumJS `Scene/Cesium3DTileContentType.js`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TileContentType {
    /// Batched 3D Model (`b3dm`)
    Batched3DModel,
    /// Instanced 3D Model (`i3dm`)
    Instanced3DModel,
    /// Point Cloud (`pnts`)
    PointCloud,
    /// Composite (`cmpt`)
    Composite,
    /// Binary glTF (`glTF` → `glb`)
    GltfBinary,
    /// Implicit subtree (`subt`)
    ImplicitSubtree,
    /// Geometry (`geom`)
    Geometry,
    /// Vector (`vctr`)
    Vector,
    /// Unknown content type
    Unknown,
}

impl TileContentType {
    /// Returns true if this is a binary format.
    pub fn is_binary(&self) -> bool {
        matches!(
            self,
            Self::Batched3DModel
                | Self::Instanced3DModel
                | Self::PointCloud
                | Self::Composite
                | Self::GltfBinary
                | Self::ImplicitSubtree
                | Self::Geometry
                | Self::Vector
        )
    }
}

/// Detects the content type from the first 4 bytes (magic) of a binary buffer.
///
/// Maps to CesiumJS `Core/getMagic.js`
pub fn detect_content_type(data: &[u8]) -> TileContentType {
    if data.len() < 4 {
        return TileContentType::Unknown;
    }
    match &data[0..4] {
        b"b3dm" => TileContentType::Batched3DModel,
        b"i3dm" => TileContentType::Instanced3DModel,
        b"pnts" => TileContentType::PointCloud,
        b"cmpt" => TileContentType::Composite,
        b"glTF" => TileContentType::GltfBinary,
        b"subt" => TileContentType::ImplicitSubtree,
        b"geom" => TileContentType::Geometry,
        b"vctr" => TileContentType::Vector,
        _ => TileContentType::Unknown,
    }
}

/// Error type for content decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// Buffer too small for the header.
    BufferTooSmall { needed: usize, actual: usize },
    /// Unsupported version.
    UnsupportedVersion { format: &'static str, version: u32 },
    /// Invalid magic bytes.
    InvalidMagic { expected: &'static str, actual: [u8; 4] },
    /// Feature table JSON byte length is zero (required for pnts/i3dm).
    EmptyFeatureTable,
    /// Invalid JSON in feature/batch table.
    InvalidJson(String),
    /// glTF byte length is zero.
    EmptyGltf,
    /// Invalid gltf format (i3dm only, must be 0 or 1).
    InvalidGltfFormat(u32),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BufferTooSmall { needed, actual } => {
                write!(f, "Buffer too small: need {needed} bytes, have {actual}")
            }
            Self::UnsupportedVersion { format, version } => {
                write!(f, "Only {format} version 1 is supported, got {version}")
            }
            Self::InvalidMagic { expected, actual } => {
                write!(
                    f,
                    "Invalid magic: expected '{expected}', got {:?}",
                    String::from_utf8_lossy(actual)
                )
            }
            Self::EmptyFeatureTable => {
                write!(f, "Feature table must have a byte length greater than zero")
            }
            Self::InvalidJson(msg) => write!(f, "Invalid JSON: {msg}"),
            Self::EmptyGltf => write!(f, "glTF byte length must be greater than 0"),
            Self::InvalidGltfFormat(fmt) => {
                write!(f, "Only glTF format 0 (uri) or 1 (embedded) are supported, got {fmt}")
            }
        }
    }
}

impl std::error::Error for DecodeError {}

/// Parsed result of a b3dm (Batched 3D Model) tile.
///
/// Maps to CesiumJS `Scene/B3dmParser.js` return value.
#[derive(Debug, Clone)]
pub struct B3dmContent {
    /// The batch length (number of features).
    pub batch_length: u32,
    /// Feature table JSON (parsed).
    pub feature_table_json: Option<Value>,
    /// Feature table binary body.
    pub feature_table_binary: Vec<u8>,
    /// Batch table JSON (parsed).
    pub batch_table_json: Option<Value>,
    /// Batch table binary body.
    pub batch_table_binary: Vec<u8>,
    /// Embedded glTF (GLB) bytes.
    pub gltf: Vec<u8>,
}

/// Parsed result of an i3dm (Instanced 3D Model) tile.
///
/// Maps to CesiumJS `Scene/I3dmParser.js` return value.
#[derive(Debug, Clone)]
pub struct I3dmContent {
    /// Feature table JSON (parsed).
    pub feature_table_json: Option<Value>,
    /// Feature table binary body.
    pub feature_table_binary: Vec<u8>,
    /// Batch table JSON (parsed).
    pub batch_table_json: Option<Value>,
    /// Batch table binary body.
    pub batch_table_binary: Vec<u8>,
    /// glTF format: 0 = URI, 1 = embedded GLB.
    pub gltf_format: u32,
    /// glTF data (URI string bytes or embedded GLB).
    pub gltf: Vec<u8>,
}

/// Parsed result of a pnts (Point Cloud) tile.
///
/// Maps to CesiumJS `Scene/PntsParser.js` return value.
#[derive(Debug, Clone)]
pub struct PntsContent {
    /// Feature table JSON (parsed).
    pub feature_table_json: Option<Value>,
    /// Feature table binary body.
    pub feature_table_binary: Vec<u8>,
    /// Batch table JSON (parsed).
    pub batch_table_json: Option<Value>,
    /// Batch table binary body.
    pub batch_table_binary: Vec<u8>,
}

/// Parsed result of a cmpt (Composite) tile.
///
/// Maps to CesiumJS `Scene/Composite3DTileContent.js`
#[derive(Debug, Clone)]
pub struct CmptContent {
    /// Inner tiles (each is raw binary of the inner tile).
    pub inner_tiles: Vec<DecodedTile>,
}

/// A decoded 3D Tile content (union of all formats).
#[derive(Debug, Clone)]
pub enum DecodedTile {
    /// Batched 3D Model
    B3dm(B3dmContent),
    /// Instanced 3D Model
    I3dm(I3dmContent),
    /// Point Cloud
    Pnts(PntsContent),
    /// Composite (contains inner tiles)
    Cmpt(CmptContent),
    /// Raw glTF binary
    Glb(Vec<u8>),
}

impl DecodedTile {
    /// Returns the content type of this decoded tile.
    pub fn content_type(&self) -> TileContentType {
        match self {
            Self::B3dm(_) => TileContentType::Batched3DModel,
            Self::I3dm(_) => TileContentType::Instanced3DModel,
            Self::Pnts(_) => TileContentType::PointCloud,
            Self::Cmpt(_) => TileContentType::Composite,
            Self::Glb(_) => TileContentType::GltfBinary,
        }
    }
}

/// Helper: read a little-endian u32 from a byte slice at offset.
fn read_u32_le(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

/// Helper: parse JSON from a byte slice region.
fn parse_json(data: &[u8], offset: usize, length: usize) -> Result<Option<Value>, DecodeError> {
    if length == 0 {
        return Ok(None);
    }
    let end = offset + length;
    if end > data.len() {
        return Err(DecodeError::BufferTooSmall {
            needed: end,
            actual: data.len(),
        });
    }
    let json_bytes = &data[offset..end];
    // Trim trailing whitespace/null padding
    let trimmed = trim_padding(json_bytes);
    if trimmed.is_empty() {
        return Ok(None);
    }
    serde_json::from_slice(trimmed)
        .map(Some)
        .map_err(|e| DecodeError::InvalidJson(e.to_string()))
}

/// Trims trailing whitespace and null bytes from JSON bytes.
fn trim_padding(bytes: &[u8]) -> &[u8] {
    let mut end = bytes.len();
    while end > 0 && (bytes[end - 1] == b' ' || bytes[end - 1] == 0) {
        end -= 1;
    }
    &bytes[..end]
}

/// Parses a b3dm (Batched 3D Model) binary buffer.
///
/// Header layout (28 bytes):
/// - magic: 4 bytes ("b3dm")
/// - version: u32 (must be 1)
/// - byteLength: u32
/// - featureTableJsonByteLength: u32
/// - featureTableBinaryByteLength: u32
/// - batchTableJsonByteLength: u32
/// - batchTableBinaryByteLength: u32
///
/// Maps to CesiumJS `B3dmParser.parse`
pub fn parse_b3dm(data: &[u8]) -> Result<B3dmContent, DecodeError> {
    parse_b3dm_at(data, 0)
}

/// Parses a b3dm at a specific byte offset.
pub fn parse_b3dm_at(data: &[u8], byte_offset: usize) -> Result<B3dmContent, DecodeError> {
    const HEADER_SIZE: usize = 28;
    let byte_start = byte_offset;

    if data.len() < byte_start + HEADER_SIZE {
        return Err(DecodeError::BufferTooSmall {
            needed: byte_start + HEADER_SIZE,
            actual: data.len(),
        });
    }

    // Validate magic
    let magic: [u8; 4] = [
        data[byte_start],
        data[byte_start + 1],
        data[byte_start + 2],
        data[byte_start + 3],
    ];
    if &magic != b"b3dm" {
        return Err(DecodeError::InvalidMagic {
            expected: "b3dm",
            actual: magic,
        });
    }

    let mut offset = byte_start + 4;
    let version = read_u32_le(data, offset);
    if version != 1 {
        return Err(DecodeError::UnsupportedVersion {
            format: "Batched 3D Model",
            version,
        });
    }
    offset += 4;

    let byte_length = read_u32_le(data, offset) as usize;
    offset += 4;

    let mut ft_json_len = read_u32_le(data, offset) as usize;
    offset += 4;
    let mut ft_bin_len = read_u32_le(data, offset) as usize;
    offset += 4;
    let mut bt_json_len = read_u32_le(data, offset) as usize;
    offset += 4;
    let mut bt_bin_len = read_u32_le(data, offset) as usize;
    offset += 4;

    // Legacy header detection (from CesiumJS B3dmParser)
    let mut batch_length: Option<u32> = None;
    if bt_json_len >= 570_425_344 {
        // Legacy format #1: [batchLength] [batchTableByteLength]
        offset -= 8;
        batch_length = Some(ft_json_len as u32);
        bt_json_len = ft_bin_len;
        bt_bin_len = 0;
        ft_json_len = 0;
        ft_bin_len = 0;
    } else if bt_bin_len >= 570_425_344 {
        // Legacy format #2: [batchTableJsonByteLength] [batchTableBinaryByteLength] [batchLength]
        offset -= 4;
        batch_length = Some(bt_json_len as u32);
        bt_json_len = ft_json_len;
        bt_bin_len = ft_bin_len;
        ft_json_len = 0;
        ft_bin_len = 0;
    }

    // Parse feature table JSON
    let feature_table_json = if ft_json_len == 0 {
        // Create default with BATCH_LENGTH
        let bl = batch_length.unwrap_or(0);
        Some(serde_json::json!({ "BATCH_LENGTH": bl }))
    } else {
        parse_json(data, offset, ft_json_len)?
    };
    offset += ft_json_len;

    // Feature table binary
    let ft_bin_end = offset + ft_bin_len;
    if ft_bin_end > data.len() {
        return Err(DecodeError::BufferTooSmall {
            needed: ft_bin_end,
            actual: data.len(),
        });
    }
    let feature_table_binary = data[offset..ft_bin_end].to_vec();
    offset = ft_bin_end;

    // Batch table JSON
    let batch_table_json = if bt_json_len > 0 {
        parse_json(data, offset, bt_json_len)?
    } else {
        None
    };
    offset += bt_json_len;

    // Batch table binary
    let batch_table_binary = if bt_bin_len > 0 {
        let bt_bin_end = offset + bt_bin_len;
        if bt_bin_end > data.len() {
            return Err(DecodeError::BufferTooSmall {
                needed: bt_bin_end,
                actual: data.len(),
            });
        }
        let bt_bin = data[offset..bt_bin_end].to_vec();
        offset = bt_bin_end;
        bt_bin
    } else {
        Vec::new()
    };

    // glTF body
    let gltf_end = byte_start + byte_length;
    let gltf_byte_length = gltf_end.saturating_sub(offset);
    if gltf_byte_length == 0 {
        return Err(DecodeError::EmptyGltf);
    }
    if gltf_end > data.len() {
        return Err(DecodeError::BufferTooSmall {
            needed: gltf_end,
            actual: data.len(),
        });
    }
    let gltf = data[offset..gltf_end].to_vec();

    // Extract BATCH_LENGTH from feature table if not from legacy header
    let final_batch_length = batch_length.unwrap_or_else(|| {
        feature_table_json
            .as_ref()
            .and_then(|v| v.get("BATCH_LENGTH"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32
    });

    Ok(B3dmContent {
        batch_length: final_batch_length,
        feature_table_json,
        feature_table_binary,
        batch_table_json,
        batch_table_binary,
        gltf,
    })
}

/// Parses an i3dm (Instanced 3D Model) binary buffer.
///
/// Header layout (32 bytes):
/// - magic: 4 bytes ("i3dm")
/// - version: u32 (must be 1)
/// - byteLength: u32
/// - featureTableJsonByteLength: u32
/// - featureTableBinaryByteLength: u32
/// - batchTableJsonByteLength: u32
/// - batchTableBinaryByteLength: u32
/// - gltfFormat: u32 (0 = URI, 1 = embedded)
///
/// Maps to CesiumJS `I3dmParser.parse`
pub fn parse_i3dm(data: &[u8]) -> Result<I3dmContent, DecodeError> {
    parse_i3dm_at(data, 0)
}

/// Parses an i3dm at a specific byte offset.
pub fn parse_i3dm_at(data: &[u8], byte_offset: usize) -> Result<I3dmContent, DecodeError> {
    const HEADER_SIZE: usize = 32;
    let byte_start = byte_offset;

    if data.len() < byte_start + HEADER_SIZE {
        return Err(DecodeError::BufferTooSmall {
            needed: byte_start + HEADER_SIZE,
            actual: data.len(),
        });
    }

    let magic: [u8; 4] = [
        data[byte_start],
        data[byte_start + 1],
        data[byte_start + 2],
        data[byte_start + 3],
    ];
    if &magic != b"i3dm" {
        return Err(DecodeError::InvalidMagic {
            expected: "i3dm",
            actual: magic,
        });
    }

    let mut offset = byte_start + 4;
    let version = read_u32_le(data, offset);
    if version != 1 {
        return Err(DecodeError::UnsupportedVersion {
            format: "Instanced 3D Model",
            version,
        });
    }
    offset += 4;

    let byte_length = read_u32_le(data, offset) as usize;
    offset += 4;

    let ft_json_len = read_u32_le(data, offset) as usize;
    if ft_json_len == 0 {
        return Err(DecodeError::EmptyFeatureTable);
    }
    offset += 4;

    let ft_bin_len = read_u32_le(data, offset) as usize;
    offset += 4;
    let bt_json_len = read_u32_le(data, offset) as usize;
    offset += 4;
    let bt_bin_len = read_u32_le(data, offset) as usize;
    offset += 4;
    let gltf_format = read_u32_le(data, offset);
    if gltf_format != 0 && gltf_format != 1 {
        return Err(DecodeError::InvalidGltfFormat(gltf_format));
    }
    offset += 4;

    // Feature table JSON
    let feature_table_json = parse_json(data, offset, ft_json_len)?;
    offset += ft_json_len;

    // Feature table binary
    let ft_bin_end = offset + ft_bin_len;
    if ft_bin_end > data.len() {
        return Err(DecodeError::BufferTooSmall {
            needed: ft_bin_end,
            actual: data.len(),
        });
    }
    let feature_table_binary = data[offset..ft_bin_end].to_vec();
    offset = ft_bin_end;

    // Batch table JSON
    let batch_table_json = if bt_json_len > 0 {
        parse_json(data, offset, bt_json_len)?
    } else {
        None
    };
    offset += bt_json_len;

    // Batch table binary
    let batch_table_binary = if bt_bin_len > 0 {
        let bt_bin_end = offset + bt_bin_len;
        if bt_bin_end > data.len() {
            return Err(DecodeError::BufferTooSmall {
                needed: bt_bin_end,
                actual: data.len(),
            });
        }
        let bt_bin = data[offset..bt_bin_end].to_vec();
        offset = bt_bin_end;
        bt_bin
    } else {
        Vec::new()
    };

    // glTF body
    let gltf_end = byte_start + byte_length;
    let gltf_byte_length = gltf_end.saturating_sub(offset);
    if gltf_byte_length == 0 {
        return Err(DecodeError::EmptyGltf);
    }
    if gltf_end > data.len() {
        return Err(DecodeError::BufferTooSmall {
            needed: gltf_end,
            actual: data.len(),
        });
    }
    let gltf = data[offset..gltf_end].to_vec();

    Ok(I3dmContent {
        feature_table_json,
        feature_table_binary,
        batch_table_json,
        batch_table_binary,
        gltf_format,
        gltf,
    })
}

/// Parses a pnts (Point Cloud) binary buffer.
///
/// Header layout (28 bytes):
/// - magic: 4 bytes ("pnts")
/// - version: u32 (must be 1)
/// - byteLength: u32
/// - featureTableJsonByteLength: u32
/// - featureTableBinaryByteLength: u32
/// - batchTableJsonByteLength: u32
/// - batchTableBinaryByteLength: u32
///
/// Maps to CesiumJS `PntsParser.parse`
pub fn parse_pnts(data: &[u8]) -> Result<PntsContent, DecodeError> {
    parse_pnts_at(data, 0)
}

/// Parses a pnts at a specific byte offset.
pub fn parse_pnts_at(data: &[u8], byte_offset: usize) -> Result<PntsContent, DecodeError> {
    const HEADER_SIZE: usize = 28;
    let byte_start = byte_offset;

    if data.len() < byte_start + HEADER_SIZE {
        return Err(DecodeError::BufferTooSmall {
            needed: byte_start + HEADER_SIZE,
            actual: data.len(),
        });
    }

    let magic: [u8; 4] = [
        data[byte_start],
        data[byte_start + 1],
        data[byte_start + 2],
        data[byte_start + 3],
    ];
    if &magic != b"pnts" {
        return Err(DecodeError::InvalidMagic {
            expected: "pnts",
            actual: magic,
        });
    }

    let mut offset = byte_start + 4;
    let version = read_u32_le(data, offset);
    if version != 1 {
        return Err(DecodeError::UnsupportedVersion {
            format: "Point Cloud",
            version,
        });
    }
    offset += 4;

    // Skip byteLength
    offset += 4;

    let ft_json_len = read_u32_le(data, offset) as usize;
    if ft_json_len == 0 {
        return Err(DecodeError::EmptyFeatureTable);
    }
    offset += 4;

    let ft_bin_len = read_u32_le(data, offset) as usize;
    offset += 4;
    let bt_json_len = read_u32_le(data, offset) as usize;
    offset += 4;
    let bt_bin_len = read_u32_le(data, offset) as usize;
    offset += 4;

    // Feature table JSON
    let feature_table_json = parse_json(data, offset, ft_json_len)?;
    offset += ft_json_len;

    // Feature table binary
    let ft_bin_end = offset + ft_bin_len;
    if ft_bin_end > data.len() {
        return Err(DecodeError::BufferTooSmall {
            needed: ft_bin_end,
            actual: data.len(),
        });
    }
    let feature_table_binary = data[offset..ft_bin_end].to_vec();
    offset = ft_bin_end;

    // Batch table JSON
    let batch_table_json = if bt_json_len > 0 {
        parse_json(data, offset, bt_json_len)?
    } else {
        None
    };
    offset += bt_json_len;

    // Batch table binary
    let batch_table_binary = if bt_bin_len > 0 {
        let bt_bin_end = offset + bt_bin_len;
        if bt_bin_end > data.len() {
            return Err(DecodeError::BufferTooSmall {
                needed: bt_bin_end,
                actual: data.len(),
            });
        }
        data[offset..bt_bin_end].to_vec()
    } else {
        Vec::new()
    };

    Ok(PntsContent {
        feature_table_json,
        feature_table_binary,
        batch_table_json,
        batch_table_binary,
    })
}

/// Parses a cmpt (Composite) binary buffer, recursively decoding inner tiles.
///
/// Header layout (16 bytes):
/// - magic: 4 bytes ("cmpt")
/// - version: u32 (must be 1)
/// - byteLength: u32
/// - tilesLength: u32
///
/// Maps to CesiumJS `Composite3DTileContent.fromTileType`
pub fn parse_cmpt(data: &[u8]) -> Result<CmptContent, DecodeError> {
    parse_cmpt_at(data, 0)
}

/// Parses a cmpt at a specific byte offset.
pub fn parse_cmpt_at(data: &[u8], byte_offset: usize) -> Result<CmptContent, DecodeError> {
    const HEADER_SIZE: usize = 16;
    let byte_start = byte_offset;

    if data.len() < byte_start + HEADER_SIZE {
        return Err(DecodeError::BufferTooSmall {
            needed: byte_start + HEADER_SIZE,
            actual: data.len(),
        });
    }

    let magic: [u8; 4] = [
        data[byte_start],
        data[byte_start + 1],
        data[byte_start + 2],
        data[byte_start + 3],
    ];
    if &magic != b"cmpt" {
        return Err(DecodeError::InvalidMagic {
            expected: "cmpt",
            actual: magic,
        });
    }

    let mut offset = byte_start + 4;
    let version = read_u32_le(data, offset);
    if version != 1 {
        return Err(DecodeError::UnsupportedVersion {
            format: "Composite",
            version,
        });
    }
    offset += 4;

    // Skip byteLength
    offset += 4;

    let tiles_length = read_u32_le(data, offset) as usize;
    offset += 4;

    let mut inner_tiles = Vec::with_capacity(tiles_length);
    for _ in 0..tiles_length {
        if offset + 12 > data.len() {
            break;
        }
        // Each inner tile has: magic(4) + version(4) + byteLength(4)
        let tile_byte_length = read_u32_le(data, offset + 8) as usize;
        if tile_byte_length == 0 || offset + tile_byte_length > data.len() {
            break;
        }

        let tile_data = &data[offset..offset + tile_byte_length];
        let content_type = detect_content_type(tile_data);
        let decoded = match content_type {
            TileContentType::Batched3DModel => {
                DecodedTile::B3dm(parse_b3dm_at(data, offset)?)
            }
            TileContentType::Instanced3DModel => {
                DecodedTile::I3dm(parse_i3dm_at(data, offset)?)
            }
            TileContentType::PointCloud => {
                DecodedTile::Pnts(parse_pnts_at(data, offset)?)
            }
            TileContentType::Composite => {
                DecodedTile::Cmpt(parse_cmpt_at(data, offset)?)
            }
            TileContentType::GltfBinary => DecodedTile::Glb(tile_data.to_vec()),
            _ => DecodedTile::Glb(tile_data.to_vec()),
        };
        inner_tiles.push(decoded);
        offset += tile_byte_length;
    }

    Ok(CmptContent { inner_tiles })
}

/// Decodes any supported 3D Tiles binary content from raw bytes.
///
/// Automatically detects the content type from magic bytes and dispatches
/// to the appropriate parser.
pub fn decode_tile_content(data: &[u8]) -> Result<DecodedTile, DecodeError> {
    let content_type = detect_content_type(data);
    match content_type {
        TileContentType::Batched3DModel => Ok(DecodedTile::B3dm(parse_b3dm(data)?)),
        TileContentType::Instanced3DModel => Ok(DecodedTile::I3dm(parse_i3dm(data)?)),
        TileContentType::PointCloud => Ok(DecodedTile::Pnts(parse_pnts(data)?)),
        TileContentType::Composite => Ok(DecodedTile::Cmpt(parse_cmpt(data)?)),
        TileContentType::GltfBinary => Ok(DecodedTile::Glb(data.to_vec())),
        _ => Err(DecodeError::InvalidMagic {
            expected: "b3dm/i3dm/pnts/cmpt/glTF",
            actual: [
                data.first().copied().unwrap_or(0),
                data.get(1).copied().unwrap_or(0),
                data.get(2).copied().unwrap_or(0),
                data.get(3).copied().unwrap_or(0),
            ],
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a minimal valid b3dm buffer for testing.
    fn build_b3dm(
        batch_length: u32,
        ft_json: &str,
        ft_bin: &[u8],
        bt_json: &str,
        bt_bin: &[u8],
        gltf: &[u8],
    ) -> Vec<u8> {
        let ft_json_bytes = ft_json.as_bytes();
        let bt_json_bytes = bt_json.as_bytes();
        let byte_length = 28
            + ft_json_bytes.len()
            + ft_bin.len()
            + bt_json_bytes.len()
            + bt_bin.len()
            + gltf.len();

        let mut buf = Vec::with_capacity(byte_length);
        buf.extend_from_slice(b"b3dm");
        buf.extend_from_slice(&1u32.to_le_bytes()); // version
        buf.extend_from_slice(&(byte_length as u32).to_le_bytes());
        buf.extend_from_slice(&(ft_json_bytes.len() as u32).to_le_bytes());
        buf.extend_from_slice(&(ft_bin.len() as u32).to_le_bytes());
        buf.extend_from_slice(&(bt_json_bytes.len() as u32).to_le_bytes());
        buf.extend_from_slice(&(bt_bin.len() as u32).to_le_bytes());
        buf.extend_from_slice(ft_json_bytes);
        buf.extend_from_slice(ft_bin);
        buf.extend_from_slice(bt_json_bytes);
        buf.extend_from_slice(bt_bin);
        buf.extend_from_slice(gltf);
        let _ = batch_length; // batch_length is in ft_json
        buf
    }

    /// Builds a minimal valid pnts buffer for testing.
    fn build_pnts(ft_json: &str, ft_bin: &[u8], bt_json: &str, bt_bin: &[u8]) -> Vec<u8> {
        let ft_json_bytes = ft_json.as_bytes();
        let bt_json_bytes = bt_json.as_bytes();
        let byte_length = 28 + ft_json_bytes.len() + ft_bin.len() + bt_json_bytes.len() + bt_bin.len();

        let mut buf = Vec::with_capacity(byte_length);
        buf.extend_from_slice(b"pnts");
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&(byte_length as u32).to_le_bytes());
        buf.extend_from_slice(&(ft_json_bytes.len() as u32).to_le_bytes());
        buf.extend_from_slice(&(ft_bin.len() as u32).to_le_bytes());
        buf.extend_from_slice(&(bt_json_bytes.len() as u32).to_le_bytes());
        buf.extend_from_slice(&(bt_bin.len() as u32).to_le_bytes());
        buf.extend_from_slice(ft_json_bytes);
        buf.extend_from_slice(ft_bin);
        buf.extend_from_slice(bt_json_bytes);
        buf.extend_from_slice(bt_bin);
        buf
    }

    /// Builds a minimal valid i3dm buffer for testing.
    fn build_i3dm(
        ft_json: &str,
        ft_bin: &[u8],
        bt_json: &str,
        bt_bin: &[u8],
        gltf_format: u32,
        gltf: &[u8],
    ) -> Vec<u8> {
        let ft_json_bytes = ft_json.as_bytes();
        let bt_json_bytes = bt_json.as_bytes();
        let byte_length = 32
            + ft_json_bytes.len()
            + ft_bin.len()
            + bt_json_bytes.len()
            + bt_bin.len()
            + gltf.len();

        let mut buf = Vec::with_capacity(byte_length);
        buf.extend_from_slice(b"i3dm");
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&(byte_length as u32).to_le_bytes());
        buf.extend_from_slice(&(ft_json_bytes.len() as u32).to_le_bytes());
        buf.extend_from_slice(&(ft_bin.len() as u32).to_le_bytes());
        buf.extend_from_slice(&(bt_json_bytes.len() as u32).to_le_bytes());
        buf.extend_from_slice(&(bt_bin.len() as u32).to_le_bytes());
        buf.extend_from_slice(&gltf_format.to_le_bytes());
        buf.extend_from_slice(ft_json_bytes);
        buf.extend_from_slice(ft_bin);
        buf.extend_from_slice(bt_json_bytes);
        buf.extend_from_slice(bt_bin);
        buf.extend_from_slice(gltf);
        buf
    }

    #[test]
    fn test_detect_content_type() {
        assert_eq!(detect_content_type(b"b3dm...."), TileContentType::Batched3DModel);
        assert_eq!(detect_content_type(b"i3dm...."), TileContentType::Instanced3DModel);
        assert_eq!(detect_content_type(b"pnts...."), TileContentType::PointCloud);
        assert_eq!(detect_content_type(b"cmpt...."), TileContentType::Composite);
        assert_eq!(detect_content_type(b"glTF...."), TileContentType::GltfBinary);
        assert_eq!(detect_content_type(b"subt...."), TileContentType::ImplicitSubtree);
        assert_eq!(detect_content_type(b"geom...."), TileContentType::Geometry);
        assert_eq!(detect_content_type(b"vctr...."), TileContentType::Vector);
        assert_eq!(detect_content_type(b"xxxx...."), TileContentType::Unknown);
        assert_eq!(detect_content_type(b"ab"), TileContentType::Unknown);
    }

    #[test]
    fn test_content_type_is_binary() {
        assert!(TileContentType::Batched3DModel.is_binary());
        assert!(TileContentType::PointCloud.is_binary());
        assert!(!TileContentType::Unknown.is_binary());
    }

    #[test]
    fn test_parse_b3dm_basic() {
        let ft_json = r#"{"BATCH_LENGTH": 10}"#;
        let gltf = b"glTF fake data here";
        let buf = build_b3dm(10, ft_json, &[], "", &[], gltf);

        let result = parse_b3dm(&buf).unwrap();
        assert_eq!(result.batch_length, 10);
        assert_eq!(
            result.feature_table_json.unwrap()["BATCH_LENGTH"],
            serde_json::json!(10)
        );
        assert!(result.feature_table_binary.is_empty());
        assert!(result.batch_table_json.is_none());
        assert!(result.batch_table_binary.is_empty());
        assert_eq!(result.gltf, gltf.to_vec());
    }

    #[test]
    fn test_parse_b3dm_with_batch_table() {
        let ft_json = r#"{"BATCH_LENGTH": 2}"#;
        let bt_json = r#"{"height": [10.5, 20.3], "name": ["A", "B"]}"#;
        let bt_bin = vec![1u8, 2, 3, 4];
        let gltf = b"glTF data";
        let buf = build_b3dm(2, ft_json, &[], bt_json, &bt_bin, gltf);

        let result = parse_b3dm(&buf).unwrap();
        assert_eq!(result.batch_length, 2);
        let bt = result.batch_table_json.unwrap();
        assert_eq!(bt["height"][0], serde_json::json!(10.5));
        assert_eq!(bt["name"][1], serde_json::json!("B"));
        assert_eq!(result.batch_table_binary, bt_bin);
    }

    #[test]
    fn test_parse_b3dm_with_feature_table_binary() {
        let ft_json = r#"{"BATCH_LENGTH": 1, "POSITION": {"byteOffset": 0}}"#;
        let ft_bin = vec![0u8; 12]; // 3 floats
        let gltf = b"glTF";
        let buf = build_b3dm(1, ft_json, &ft_bin, "", &[], gltf);

        let result = parse_b3dm(&buf).unwrap();
        assert_eq!(result.feature_table_binary.len(), 12);
    }

    #[test]
    fn test_parse_b3dm_invalid_magic() {
        let buf = build_b3dm(0, "{}", &[], "", &[], b"glTF");
        let mut bad = buf.clone();
        bad[0] = b'x';
        assert!(matches!(
            parse_b3dm(&bad),
            Err(DecodeError::InvalidMagic { .. })
        ));
    }

    #[test]
    fn test_parse_b3dm_invalid_version() {
        let mut buf = build_b3dm(0, r#"{"BATCH_LENGTH":0}"#, &[], "", &[], b"glTF");
        buf[4] = 2; // version = 2
        assert!(matches!(
            parse_b3dm(&buf),
            Err(DecodeError::UnsupportedVersion { version: 2, .. })
        ));
    }

    #[test]
    fn test_parse_b3dm_buffer_too_small() {
        let buf = vec![0u8; 10];
        assert!(matches!(
            parse_b3dm(&buf),
            Err(DecodeError::BufferTooSmall { .. })
        ));
    }

    #[test]
    fn test_parse_pnts_basic() {
        let ft_json = r#"{"POINTS_LENGTH": 3, "POSITION": {"byteOffset": 0}}"#;
        let ft_bin = vec![0u8; 36]; // 3 points × 3 floats × 4 bytes
        let buf = build_pnts(ft_json, &ft_bin, "", &[]);

        let result = parse_pnts(&buf).unwrap();
        let ft = result.feature_table_json.unwrap();
        assert_eq!(ft["POINTS_LENGTH"], serde_json::json!(3));
        assert_eq!(result.feature_table_binary.len(), 36);
        assert!(result.batch_table_json.is_none());
    }

    #[test]
    fn test_parse_pnts_with_batch_table() {
        let ft_json = r#"{"POINTS_LENGTH": 2}"#;
        let bt_json = r#"{"intensity": [100, 200]}"#;
        let bt_bin = vec![10u8, 20];
        let buf = build_pnts(ft_json, &[], bt_json, &bt_bin);

        let result = parse_pnts(&buf).unwrap();
        let bt = result.batch_table_json.unwrap();
        assert_eq!(bt["intensity"][0], serde_json::json!(100));
        assert_eq!(result.batch_table_binary, bt_bin);
    }

    #[test]
    fn test_parse_pnts_empty_feature_table() {
        // featureTableJsonByteLength = 0 should error
        let mut buf = build_pnts("", &[], "", &[]);
        // Manually set ft_json_len to 0 (it already is since "" has 0 bytes)
        // Actually the builder uses the string length, so "" gives 0
        // But we need at least the header to be valid
        buf[12] = 0; // ft_json_len = 0
        buf[13] = 0;
        buf[14] = 0;
        buf[15] = 0;
        assert!(matches!(parse_pnts(&buf), Err(DecodeError::EmptyFeatureTable)));
    }

    #[test]
    fn test_parse_i3dm_basic() {
        let ft_json = r#"{"INSTANCES_LENGTH": 5, "POSITION": {"byteOffset": 0}}"#;
        let ft_bin = vec![0u8; 60]; // 5 instances × 3 floats × 4 bytes
        let gltf = b"glTF embedded model";
        let buf = build_i3dm(ft_json, &ft_bin, "", &[], 1, gltf);

        let result = parse_i3dm(&buf).unwrap();
        let ft = result.feature_table_json.unwrap();
        assert_eq!(ft["INSTANCES_LENGTH"], serde_json::json!(5));
        assert_eq!(result.gltf_format, 1);
        assert_eq!(result.gltf, gltf.to_vec());
    }

    #[test]
    fn test_parse_i3dm_uri_format() {
        let ft_json = r#"{"INSTANCES_LENGTH": 1}"#;
        let uri = b"model.glb";
        let buf = build_i3dm(ft_json, &[], "", &[], 0, uri);

        let result = parse_i3dm(&buf).unwrap();
        assert_eq!(result.gltf_format, 0);
        assert_eq!(result.gltf, uri.to_vec());
    }

    #[test]
    fn test_parse_i3dm_invalid_gltf_format() {
        let ft_json = r#"{"INSTANCES_LENGTH": 1}"#;
        let buf = build_i3dm(ft_json, &[], "", &[], 2, b"data");
        assert!(matches!(
            parse_i3dm(&buf),
            Err(DecodeError::InvalidGltfFormat(2))
        ));
    }

    #[test]
    fn test_parse_cmpt_basic() {
        // Build two inner b3dm tiles
        let inner1 = build_b3dm(1, r#"{"BATCH_LENGTH":1}"#, &[], "", &[], b"glTF1");
        let inner2 = build_b3dm(2, r#"{"BATCH_LENGTH":2}"#, &[], "", &[], b"glTF2");

        let byte_length = 16 + inner1.len() + inner2.len();
        let mut buf = Vec::new();
        buf.extend_from_slice(b"cmpt");
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&(byte_length as u32).to_le_bytes());
        buf.extend_from_slice(&2u32.to_le_bytes()); // tilesLength
        buf.extend_from_slice(&inner1);
        buf.extend_from_slice(&inner2);

        let result = parse_cmpt(&buf).unwrap();
        assert_eq!(result.inner_tiles.len(), 2);
        assert_eq!(result.inner_tiles[0].content_type(), TileContentType::Batched3DModel);
        assert_eq!(result.inner_tiles[1].content_type(), TileContentType::Batched3DModel);

        if let DecodedTile::B3dm(b3dm) = &result.inner_tiles[0] {
            assert_eq!(b3dm.batch_length, 1);
        }
        if let DecodedTile::B3dm(b3dm) = &result.inner_tiles[1] {
            assert_eq!(b3dm.batch_length, 2);
        }
    }

    #[test]
    fn test_parse_cmpt_mixed_content() {
        let inner_b3dm = build_b3dm(1, r#"{"BATCH_LENGTH":1}"#, &[], "", &[], b"glTF");
        let inner_pnts = build_pnts(r#"{"POINTS_LENGTH":10}"#, &[], "", &[]);

        let byte_length = 16 + inner_b3dm.len() + inner_pnts.len();
        let mut buf = Vec::new();
        buf.extend_from_slice(b"cmpt");
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&(byte_length as u32).to_le_bytes());
        buf.extend_from_slice(&2u32.to_le_bytes());
        buf.extend_from_slice(&inner_b3dm);
        buf.extend_from_slice(&inner_pnts);

        let result = parse_cmpt(&buf).unwrap();
        assert_eq!(result.inner_tiles.len(), 2);
        assert_eq!(result.inner_tiles[0].content_type(), TileContentType::Batched3DModel);
        assert_eq!(result.inner_tiles[1].content_type(), TileContentType::PointCloud);
    }

    #[test]
    fn test_decode_tile_content_dispatch() {
        let b3dm = build_b3dm(5, r#"{"BATCH_LENGTH":5}"#, &[], "", &[], b"glTF data");
        let decoded = decode_tile_content(&b3dm).unwrap();
        assert_eq!(decoded.content_type(), TileContentType::Batched3DModel);

        let pnts = build_pnts(r#"{"POINTS_LENGTH":1}"#, &[0u8; 12], "", &[]);
        let decoded = decode_tile_content(&pnts).unwrap();
        assert_eq!(decoded.content_type(), TileContentType::PointCloud);
    }

    #[test]
    fn test_decode_tile_content_unknown() {
        let data = b"unknown format data";
        assert!(decode_tile_content(data).is_err());
    }

    #[test]
    fn test_b3dm_legacy_header_format1() {
        // Legacy format #1: [magic(4)] [version(4)] [byteLength(4)] [batchLength(4)] [batchTableByteLength(4)]
        // Total header = 20 bytes, then batch table JSON immediately follows.
        // Detection: the value at offset 20 (first byte of JSON = '"' = 0x22)
        // when read as uint32 LE gives >= 570425344.
        let bt_json = r#"{"id": [1, 2]}"#;
        let bt_json_bytes = bt_json.as_bytes();
        let gltf = b"glTF legacy";

        // byteLength covers the whole tile
        let byte_length = 20 + bt_json_bytes.len() + gltf.len();
        let mut buf = Vec::new();
        buf.extend_from_slice(b"b3dm");
        buf.extend_from_slice(&1u32.to_le_bytes()); // version
        buf.extend_from_slice(&(byte_length as u32).to_le_bytes());
        // batchLength = 3 (stored in ft_json_len slot)
        buf.extend_from_slice(&3u32.to_le_bytes());
        // batchTableByteLength (stored in ft_bin_len slot)
        buf.extend_from_slice(&(bt_json_bytes.len() as u32).to_le_bytes());
        // Batch table JSON starts immediately at offset 20
        buf.extend_from_slice(bt_json_bytes);
        buf.extend_from_slice(gltf);

        let result = parse_b3dm(&buf).unwrap();
        assert_eq!(result.batch_length, 3);
        let bt = result.batch_table_json.unwrap();
        assert_eq!(bt["id"][0], serde_json::json!(1));
    }
}
