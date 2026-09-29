//! 3D Tiles 二进制内容解码（b3dm、i3dm、pnts、cmpt）。
//!
//! 镜像 CesiumJS：
//! - `Scene/B3dmParser.js`
//! - `Scene/I3dmParser.js`
//! - `Scene/PntsParser.js`
//! - `Scene/Composite3DTileContent.js`
//! - `Scene/Cesium3DTileContentType.js`

use serde_json::Value;

/// 3D Tile 内容的类型，通过 magic 字节识别。
///
/// 映射到 CesiumJS `Scene/Cesium3DTileContentType.js`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TileContentType {
    /// 批量 3D 模型（`b3dm`）
    Batched3DModel,
    /// 实例化 3D 模型（`i3dm`）
    Instanced3DModel,
    /// 点云（`pnts`）
    PointCloud,
    /// 复合（`cmpt`）
    Composite,
    /// 二进制 glTF（`glTF` → `glb`）
    GltfBinary,
    /// 隐式子树（`subt`）
    ImplicitSubtree,
    /// 几何（`geom`）
    Geometry,
    /// 向量（`vctr`）
    Vector,
    /// 未知内容类型
    Unknown,
}

impl TileContentType {
    /// 若这是一个二进制格式则返回 true。
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

/// 从二进制缓冲区的前 4 个字节（magic）检测内容类型。
///
/// 映射到 CesiumJS `Core/getMagic.js`
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

/// 内容解码的错误类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// 缓冲区对于头部而言太小。
    BufferTooSmall { needed: usize, actual: usize },
    /// 不支持的版本。
    UnsupportedVersion { format: &'static str, version: u32 },
    /// 无效的 magic 字节。
    InvalidMagic { expected: &'static str, actual: [u8; 4] },
    /// feature table JSON 字节长度为零（pnts/i3dm 必需）。
    EmptyFeatureTable,
    /// feature/batch table 中的 JSON 无效。
    InvalidJson(String),
    /// glTF 字节长度为零。
    EmptyGltf,
    /// 无效的 gltf 格式（仅 i3dm，必须为 0 或 1）。
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

/// b3dm（批量 3D 模型）瓦片的解析结果。
///
/// 映射到 CesiumJS `Scene/B3dmParser.js` 的返回值。
#[derive(Debug, Clone)]
pub struct B3dmContent {
    /// batch length（feature 数量）。
    pub batch_length: u32,
    /// feature table JSON（已解析）。
    pub feature_table_json: Option<Value>,
    /// feature table 二进制主体。
    pub feature_table_binary: Vec<u8>,
    /// batch table JSON（已解析）。
    pub batch_table_json: Option<Value>,
    /// batch table 二进制主体。
    pub batch_table_binary: Vec<u8>,
    /// 内嵌的 glTF（GLB）字节。
    pub gltf: Vec<u8>,
}

/// i3dm（实例化 3D 模型）瓦片的解析结果。
///
/// 映射到 CesiumJS `Scene/I3dmParser.js` 的返回值。
#[derive(Debug, Clone)]
pub struct I3dmContent {
    /// feature table JSON（已解析）。
    pub feature_table_json: Option<Value>,
    /// feature table 二进制主体。
    pub feature_table_binary: Vec<u8>,
    /// batch table JSON（已解析）。
    pub batch_table_json: Option<Value>,
    /// batch table 二进制主体。
    pub batch_table_binary: Vec<u8>,
    /// glTF 格式：0 = URI，1 = 内嵌 GLB。
    pub gltf_format: u32,
    /// glTF 数据（URI 字符串字节或内嵌 GLB）。
    pub gltf: Vec<u8>,
}

/// pnts（点云）瓦片的解析结果。
///
/// 映射到 CesiumJS `Scene/PntsParser.js` 的返回值。
#[derive(Debug, Clone)]
pub struct PntsContent {
    /// feature table JSON（已解析）。
    pub feature_table_json: Option<Value>,
    /// feature table 二进制主体。
    pub feature_table_binary: Vec<u8>,
    /// batch table JSON（已解析）。
    pub batch_table_json: Option<Value>,
    /// batch table 二进制主体。
    pub batch_table_binary: Vec<u8>,
}

/// cmpt（复合）瓦片的解析结果。
///
/// 映射到 CesiumJS `Scene/Composite3DTileContent.js`
#[derive(Debug, Clone)]
pub struct CmptContent {
    /// 内部瓦片（每个均为内部瓦片的原始二进制）。
    pub inner_tiles: Vec<DecodedTile>,
}

/// 一个已解码的 3D Tile 内容（所有格式的并集）。
#[derive(Debug, Clone)]
pub enum DecodedTile {
    /// 批量 3D 模型
    B3dm(B3dmContent),
    /// 实例化 3D 模型
    I3dm(I3dmContent),
    /// 点云
    Pnts(PntsContent),
    /// 复合（包含内部瓦片）
    Cmpt(CmptContent),
    /// 原始 glTF 二进制
    Glb(Vec<u8>),
}

impl DecodedTile {
    /// 返回本已解码瓦片的内容类型。
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

/// 辅助函数：从字节切片的偏移处读取一个Little-Endian u32。
fn read_u32_le(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

/// 辅助函数：从字节切片区域解析 JSON。
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
    // 剪除尾部的空白/null 填充
    let trimmed = trim_padding(json_bytes);
    if trimmed.is_empty() {
        return Ok(None);
    }
    serde_json::from_slice(trimmed)
        .map(Some)
        .map_err(|e| DecodeError::InvalidJson(e.to_string()))
}

/// 从 JSON 字节中剪除尾部的空白和 null 字节。
fn trim_padding(bytes: &[u8]) -> &[u8] {
    let mut end = bytes.len();
    while end > 0 && (bytes[end - 1] == b' ' || bytes[end - 1] == 0) {
        end -= 1;
    }
    &bytes[..end]
}

/// 解析一个 b3dm（批量 3D 模型）二进制缓冲区。
///
/// 头部布局（28 字节）：
/// - magic：4 字节（"b3dm"）
/// - version：u32（必须为 1）
/// - byteLength：u32
/// - featureTableJsonByteLength：u32
/// - featureTableBinaryByteLength：u32
/// - batchTableJsonByteLength：u32
/// - batchTableBinaryByteLength：u32
///
/// 映射到 CesiumJS `B3dmParser.parse`
pub fn parse_b3dm(data: &[u8]) -> Result<B3dmContent, DecodeError> {
    parse_b3dm_at(data, 0)
}

/// 在指定的字节偏移处解析一个 b3dm。
pub fn parse_b3dm_at(data: &[u8], byte_offset: usize) -> Result<B3dmContent, DecodeError> {
    const HEADER_SIZE: usize = 28;
    let byte_start = byte_offset;

    if data.len() < byte_start + HEADER_SIZE {
        return Err(DecodeError::BufferTooSmall {
            needed: byte_start + HEADER_SIZE,
            actual: data.len(),
        });
    }

    // 校验 magic
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

    // 遗留头部检测（来自 CesiumJS B3dmParser）
    let mut batch_length: Option<u32> = None;
    if bt_json_len >= 570_425_344 {
        // 遗留格式 #1：[batchLength] [batchTableByteLength]
        offset -= 8;
        batch_length = Some(ft_json_len as u32);
        bt_json_len = ft_bin_len;
        bt_bin_len = 0;
        ft_json_len = 0;
        ft_bin_len = 0;
    } else if bt_bin_len >= 570_425_344 {
        // 遗留格式 #2：[batchTableJsonByteLength] [batchTableBinaryByteLength] [batchLength]
        offset -= 4;
        batch_length = Some(bt_json_len as u32);
        bt_json_len = ft_json_len;
        bt_bin_len = ft_bin_len;
        ft_json_len = 0;
        ft_bin_len = 0;
    }

    // 解析 feature table JSON
    let feature_table_json = if ft_json_len == 0 {
        // 创建带 BATCH_LENGTH 的默认值
        let bl = batch_length.unwrap_or(0);
        Some(serde_json::json!({ "BATCH_LENGTH": bl }))
    } else {
        parse_json(data, offset, ft_json_len)?
    };
    offset += ft_json_len;

    // feature table 二进制主体 部分
    let ft_bin_end = offset + ft_bin_len;
    if ft_bin_end > data.len() {
        return Err(DecodeError::BufferTooSmall {
            needed: ft_bin_end,
            actual: data.len(),
        });
    }
    let feature_table_binary = data[offset..ft_bin_end].to_vec();
    offset = ft_bin_end;

    // batch table JSON 部分
    let batch_table_json = if bt_json_len > 0 {
        parse_json(data, offset, bt_json_len)?
    } else {
        None
    };
    offset += bt_json_len;

    // batch table 二进制主体 部分
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

    // glTF 主体
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

    // 提取 BATCH_LENGTH（若并非来自遗留头部）
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

/// 解析一个 i3dm（实例化 3D 模型）二进制缓冲区。
///
/// 头部布局（32 字节）：
/// - magic：4 字节（"i3dm"）
/// - version：u32（必须为 1）
/// - byteLength：u32
/// - featureTableJsonByteLength：u32
/// - featureTableBinaryByteLength：u32
/// - batchTableJsonByteLength：u32
/// - batchTableBinaryByteLength：u32
/// - gltfFormat：u32（0 = URI，1 = 内嵌）
///
/// 映射到 CesiumJS `I3dmParser.parse`
pub fn parse_i3dm(data: &[u8]) -> Result<I3dmContent, DecodeError> {
    parse_i3dm_at(data, 0)
}

/// 在指定的字节偏移处解析一个 i3dm。
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

    // feature table JSON 部分
    let feature_table_json = parse_json(data, offset, ft_json_len)?;
    offset += ft_json_len;

    // feature table 二进制主体 部分
    let ft_bin_end = offset + ft_bin_len;
    if ft_bin_end > data.len() {
        return Err(DecodeError::BufferTooSmall {
            needed: ft_bin_end,
            actual: data.len(),
        });
    }
    let feature_table_binary = data[offset..ft_bin_end].to_vec();
    offset = ft_bin_end;

    // batch table JSON 部分
    let batch_table_json = if bt_json_len > 0 {
        parse_json(data, offset, bt_json_len)?
    } else {
        None
    };
    offset += bt_json_len;

    // batch table 二进制主体 部分
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

    // glTF 主体
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

/// 解析一个 pnts（点云）二进制缓冲区。
///
/// 头部布局（28 字节）：
/// - magic：4 字节（"pnts"）
/// - version：u32（必须为 1）
/// - byteLength：u32
/// - featureTableJsonByteLength：u32
/// - featureTableBinaryByteLength：u32
/// - batchTableJsonByteLength：u32
/// - batchTableBinaryByteLength：u32
///
/// 映射到 CesiumJS `PntsParser.parse`
pub fn parse_pnts(data: &[u8]) -> Result<PntsContent, DecodeError> {
    parse_pnts_at(data, 0)
}

/// 在指定的字节偏移处解析一个 pnts。
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

    // 跳过 byteLength
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

    // feature table JSON 部分
    let feature_table_json = parse_json(data, offset, ft_json_len)?;
    offset += ft_json_len;

    // feature table 二进制主体 部分
    let ft_bin_end = offset + ft_bin_len;
    if ft_bin_end > data.len() {
        return Err(DecodeError::BufferTooSmall {
            needed: ft_bin_end,
            actual: data.len(),
        });
    }
    let feature_table_binary = data[offset..ft_bin_end].to_vec();
    offset = ft_bin_end;

    // batch table JSON 部分
    let batch_table_json = if bt_json_len > 0 {
        parse_json(data, offset, bt_json_len)?
    } else {
        None
    };
    offset += bt_json_len;

    // batch table 二进制主体 部分
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

/// 解析一个 cmpt（复合）二进制缓冲区，递归解码内部瓦片。
///
/// 头部布局（16 字节）：
/// - magic：4 字节（"cmpt"）
/// - version：u32（必须为 1）
/// - byteLength：u32
/// - tilesLength：u32
///
/// 映射到 CesiumJS `Composite3DTileContent.fromTileType`
pub fn parse_cmpt(data: &[u8]) -> Result<CmptContent, DecodeError> {
    parse_cmpt_at(data, 0)
}

/// 在指定的字节偏移处解析一个 cmpt。
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

    // 跳过 byteLength
    offset += 4;

    let tiles_length = read_u32_le(data, offset) as usize;
    offset += 4;

    let mut inner_tiles = Vec::with_capacity(tiles_length);
    for _ in 0..tiles_length {
        if offset + 12 > data.len() {
            break;
        }
        // 每个内部瓦片包含：magic(4) + version(4) + byteLength(4)
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

/// 从原始字节解码任意受支持的 3D Tiles 二进制内容。
///
/// 自动从 magic 字节检测内容类型并分发
/// 到相应的解析器。
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

    /// 为测试构建一个最小的有效 b3dm 缓冲区。
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
        buf.extend_from_slice(&1u32.to_le_bytes()); // 版本
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
        let _ = batch_length; // batch_length 在 ft_json 中
        buf
    }

    /// 为测试构建一个最小的有效 pnts 缓冲区。
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

    /// 为测试构建一个最小的有效 i3dm 缓冲区。
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
        let ft_bin = vec![0u8; 12]; // 3 个浮点数
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
        let ft_bin = vec![0u8; 36]; // 3 点 × 3 浮点 × 4 字节
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
        // featureTableJsonByteLength = 0 应报错
        let mut buf = build_pnts("", &[], "", &[]);
        // 手动将 ft_json_len 设为 0（由于 "" 有 0 字节，它已经是 0）
        // 实际上 builder 使用字符串长度，所以 "" 得到 0
        // 但至少需要头部有效
        buf[12] = 0; // ft_json_len = 0
        buf[13] = 0;
        buf[14] = 0;
        buf[15] = 0;
        assert!(matches!(parse_pnts(&buf), Err(DecodeError::EmptyFeatureTable)));
    }

    #[test]
    fn test_parse_i3dm_basic() {
        let ft_json = r#"{"INSTANCES_LENGTH": 5, "POSITION": {"byteOffset": 0}}"#;
        let ft_bin = vec![0u8; 60]; // 5 实例 × 3 浮点 × 4 字节
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
        // 构建两个内部 b3dm 瓦片
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
        // 遗留格式 #1：[magic(4)] [version(4)] [byteLength(4)] [batchLength(4)] [batchTableByteLength(4)]
        // 总头部 = 20 字节，随后紧跟 batch table JSON。
        // 检测：偏移 20 处的值（JSON 的首字节 = '"' = 0x22）
        // 作为 uint32 LE 读取时 >= 570425344。
        let bt_json = r#"{"id": [1, 2]}"#;
        let bt_json_bytes = bt_json.as_bytes();
        let gltf = b"glTF legacy";

        // byteLength 覆盖整个瓦片
        let byte_length = 20 + bt_json_bytes.len() + gltf.len();
        let mut buf = Vec::new();
        buf.extend_from_slice(b"b3dm");
        buf.extend_from_slice(&1u32.to_le_bytes()); // 版本
        buf.extend_from_slice(&(byte_length as u32).to_le_bytes());
        // batchLength = 3（存于 ft_json_len 槽位）
        buf.extend_from_slice(&3u32.to_le_bytes());
        // batchTableByteLength（存于 ft_bin_len 槽位）
        buf.extend_from_slice(&(bt_json_bytes.len() as u32).to_le_bytes());
        // batch table JSON 从偏移 20 处立即开始
        buf.extend_from_slice(bt_json_bytes);
        buf.extend_from_slice(gltf);

        let result = parse_b3dm(&buf).unwrap();
        assert_eq!(result.batch_length, 3);
        let bt = result.batch_table_json.unwrap();
        assert_eq!(bt["id"][0], serde_json::json!(1));
    }
}
