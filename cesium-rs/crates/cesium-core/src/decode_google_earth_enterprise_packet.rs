//! Ported from `packages/engine/Source/Workers/decodeGoogleEarthEnterprisePacket.js`.
//!
//! # Alignment table
//!
//! | JS | Rust | Notes |
//! |---|---|---|
//! | `Types` | [`GeePacketType`] | `fromString` → [`GeePacketType::from_str`] |
//! | `decodeGoogleEarthEnterprisePacket` | [`decode_google_earth_enterprise_packet`] | synchronous, in-process |
//! | `processMetadata` | [`process_metadata`] | |
//! | `processTerrain` | [`process_terrain`] | |
//! | `uncompressPacket` | [`uncompress_packet`] | pako `inflate` → raw-deflate via `flate2` |
//! | `qtMagic` / `compressedMagic` / `compressedMagicSwap` | module constants | identical values |
//!
//! # DEVIATIONS
//!
//! 1. In CesiumJS this logic runs in a Web Worker driven by `TaskProcessor`.
//!    `cesium-core` cannot depend on `cesium-workers`, so the decoder is
//!    materialized here and invoked synchronously/in-process by
//!    `GoogleEarthEnterpriseMetadata` and `GoogleEarthEnterpriseTerrainProvider`.
//! 2. JS `tileInfo` maps quadkeys to `GoogleEarthEnterpriseTileInformation`
//!    or `null`; the Rust port models that with
//!    `HashMap<String, Option<GoogleEarthEnterpriseTileInformation>>`
//!    (missing key mirrors JS `undefined`).
//! 3. The worker's `transferableObjects` bookkeeping has no Rust analogue.

use std::collections::HashMap;
use std::io::Read;

use flate2::bufread::ZlibDecoder;

use crate::decode_google_earth_enterprise_data::decode_google_earth_enterprise_data;
use crate::google_earth_enterprise_tile_information::GoogleEarthEnterpriseTileInformation;
use crate::runtime_error::RuntimeError;

const SIZE_OF_UINT16: usize = 2;
const SIZE_OF_INT32: usize = 4;
const SIZE_OF_UINT32: usize = 4;

/// Mirrors the JS `Types` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeePacketType {
    /// `Types.METADATA`.
    Metadata,
    /// `Types.TERRAIN`.
    Terrain,
    /// `Types.DBROOT`.
    DbRoot,
}

impl GeePacketType {
    /// Mirrors `Types.fromString`. Returns `None` for unknown strings
    /// (JS returns `undefined`).
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Metadata" => Some(Self::Metadata),
            "Terrain" => Some(Self::Terrain),
            "DbRoot" => Some(Self::DbRoot),
            _ => None,
        }
    }
}

/// The result of decoding a Google Earth Enterprise packet.
pub enum GeePacketResult {
    /// `processMetadata` result: quadkey → tile info (`None` mirrors JS `null`).
    Metadata(HashMap<String, Option<GoogleEarthEnterpriseTileInformation>>),
    /// `processTerrain` result: the 5 terrain mesh buffers.
    Terrain(Vec<Vec<u8>>),
    /// `Types.DBROOT`: the decoded buffer returned as-is.
    DbRoot(Vec<u8>),
}

/// Mirrors `decodeGoogleEarthEnterprisePacket(parameters, transferableObjects)`.
///
/// Decrypts `buffer` in place with `key`, uncompresses the packet, then
/// dispatches on `packet_type`.
pub fn decode_google_earth_enterprise_packet(
    key: &[u8],
    buffer: &mut Vec<u8>,
    packet_type: GeePacketType,
    quad_key: &str,
) -> Result<GeePacketResult, RuntimeError> {
    decode_google_earth_enterprise_data(key, buffer);

    let uncompressed = uncompress_packet(buffer)?;

    match packet_type {
        GeePacketType::Metadata => {
            let len = uncompressed.len();
            Ok(GeePacketResult::Metadata(process_metadata(
                &uncompressed,
                len,
                quad_key,
            )?))
        }
        GeePacketType::Terrain => {
            let len = uncompressed.len();
            Ok(GeePacketResult::Terrain(process_terrain(&uncompressed, len)?))
        }
        GeePacketType::DbRoot => Ok(GeePacketResult::DbRoot(uncompressed)),
    }
}

const QT_MAGIC: u32 = 32301;

fn read_u8(buffer: &[u8], offset: usize) -> Result<u8, RuntimeError> {
    buffer.get(offset).copied().ok_or_else(|| RuntimeError::new(Some("Invalid packet offsets")))
}

fn read_u16_le(buffer: &[u8], offset: usize) -> Result<u16, RuntimeError> {
    let bytes: [u8; 2] = buffer
        .get(offset..offset + SIZE_OF_UINT16)
        .ok_or_else(|| RuntimeError::new(Some("Invalid packet offsets")))?
        .try_into()
        .unwrap();
    Ok(u16::from_le_bytes(bytes))
}

fn read_u32(buffer: &[u8], offset: usize, little_endian: bool) -> Result<u32, RuntimeError> {
    let bytes: [u8; 4] = buffer
        .get(offset..offset + SIZE_OF_UINT32)
        .ok_or_else(|| RuntimeError::new(Some("Invalid packet offsets")))?
        .try_into()
        .unwrap();
    Ok(if little_endian {
        u32::from_le_bytes(bytes)
    } else {
        u32::from_be_bytes(bytes)
    })
}

fn read_i32_le(buffer: &[u8], offset: usize) -> Result<i32, RuntimeError> {
    let bytes: [u8; 4] = buffer
        .get(offset..offset + SIZE_OF_INT32)
        .ok_or_else(|| RuntimeError::new(Some("Invalid packet offsets")))?
        .try_into()
        .unwrap();
    Ok(i32::from_le_bytes(bytes))
}

/// Mirrors `processMetadata(buffer, totalSize, quadKey)`.
pub fn process_metadata(
    buffer: &[u8],
    total_size: usize,
    quad_key: &str,
) -> Result<HashMap<String, Option<GoogleEarthEnterpriseTileInformation>>, RuntimeError> {
    let mut offset = 0usize;
    let magic = read_u32(buffer, offset, true)?;
    offset += SIZE_OF_UINT32;
    if magic != QT_MAGIC {
        return Err(RuntimeError::new(Some("Invalid magic")));
    }

    let data_type_id = read_u32(buffer, offset, true)?;
    offset += SIZE_OF_UINT32;
    if data_type_id != 1 {
        return Err(RuntimeError::new(Some(
            "Invalid data type. Must be 1 for QuadTreePacket",
        )));
    }

    // Tile format version
    let quad_version = read_u32(buffer, offset, true)?;
    offset += SIZE_OF_UINT32;
    if quad_version != 2 {
        return Err(RuntimeError::new(Some(
            "Invalid QuadTreePacket version. Only version 2 is supported.",
        )));
    }

    let num_instances = read_i32_le(buffer, offset)?;
    offset += SIZE_OF_INT32;

    let data_instance_size = read_i32_le(buffer, offset)?;
    offset += SIZE_OF_INT32;
    if data_instance_size != 32 {
        return Err(RuntimeError::new(Some("Invalid instance size.")));
    }

    let data_buffer_offset = read_i32_le(buffer, offset)?;
    offset += SIZE_OF_INT32;

    let data_buffer_size = read_i32_le(buffer, offset)?;
    offset += SIZE_OF_INT32;

    let meta_buffer_size = read_i32_le(buffer, offset)?;
    offset += SIZE_OF_INT32;

    // Offset from beginning of packet (instances + current offset)
    let num_instances_usize = num_instances.max(0) as usize;
    if data_buffer_offset != (num_instances * data_instance_size as i32) + offset as i32 {
        return Err(RuntimeError::new(Some("Invalid dataBufferOffset")));
    }

    // Verify the packets is all there header + instances + dataBuffer + metaBuffer
    if data_buffer_offset + data_buffer_size + meta_buffer_size != total_size as i32 {
        return Err(RuntimeError::new(Some("Invalid packet offsets")));
    }

    // Read all the instances
    let mut instances = Vec::with_capacity(num_instances_usize);
    for _ in 0..num_instances_usize {
        let bitfield = read_u8(buffer, offset)?;
        offset += 1;

        offset += 1; // 2 byte align

        let cnode_version = read_u16_le(buffer, offset)?;
        offset += SIZE_OF_UINT16;

        let image_version = read_u16_le(buffer, offset)?;
        offset += SIZE_OF_UINT16;

        let terrain_version = read_u16_le(buffer, offset)?;
        offset += SIZE_OF_UINT16;

        // Number of channels stored in the dataBuffer
        offset += SIZE_OF_UINT16;

        offset += SIZE_OF_UINT16; // 4 byte align

        // Channel type offset into dataBuffer
        offset += SIZE_OF_INT32;

        // Channel version offset into dataBuffer
        offset += SIZE_OF_INT32;

        offset += 8; // Ignore image neighbors for now

        // Data providers
        let image_provider = read_u8(buffer, offset)?;
        offset += 1;
        let terrain_provider = read_u8(buffer, offset)?;
        offset += 1;
        offset += SIZE_OF_UINT16; // 4 byte align

        instances.push(GoogleEarthEnterpriseTileInformation::new(
            bitfield as u32,
            cnode_version as u32,
            image_version as u32,
            terrain_version as u32,
            image_provider as u32,
            terrain_provider as u32,
        ));
    }

    let mut tile_info: HashMap<String, Option<GoogleEarthEnterpriseTileInformation>> =
        HashMap::new();
    let mut index = 0usize;

    let mut level = 0i32;
    if index >= instances.len() {
        return Err(RuntimeError::new(Some("Invalid packet offsets")));
    }
    let root = instances[index].clone();
    index += 1;
    if quad_key.is_empty() {
        // Root tile has data at its root and one less level
        level += 1;
    } else {
        // This will only contain the child bitmask
        tile_info.insert(quad_key.to_string(), Some(root.clone()));
    }

    populate_tiles(
        quad_key,
        &root,
        level,
        &instances,
        &mut index,
        &mut tile_info,
    );

    Ok(tile_info)
}

/// Mirrors the nested `populateTiles(parentKey, parent, level)` closure.
fn populate_tiles(
    parent_key: &str,
    parent: &GoogleEarthEnterpriseTileInformation,
    level: i32,
    instances: &[GoogleEarthEnterpriseTileInformation],
    index: &mut usize,
    tile_info: &mut HashMap<String, Option<GoogleEarthEnterpriseTileInformation>>,
) {
    let mut is_leaf = false;
    if level == 4 {
        if parent.has_subtree() {
            return; // We have a subtree, so just return
        }

        is_leaf = true; // No subtree, so set all children to null
    }
    for i in 0..4usize {
        let child_key = format!("{parent_key}{i}");
        if is_leaf {
            // No subtree so set all children to null
            tile_info.insert(child_key, None);
        } else if level < 4 {
            // We are still in the middle of the subtree, so add child
            //  only if their bits are set, otherwise set child to null.
            if !parent.has_child(i) {
                tile_info.insert(child_key, None);
            } else {
                if *index == instances.len() {
                    println!("Incorrect number of instances");
                    return;
                }

                let instance = instances[*index].clone();
                *index += 1;
                tile_info.insert(child_key.clone(), Some(instance.clone()));
                populate_tiles(&child_key, &instance, level + 1, instances, index, tile_info);
            }
        }
    }
}

const NUM_MESHES_PER_PACKET: usize = 5;
const NUM_SUB_MESHES_PER_MESH: usize = 4;

/// Each terrain packet will have 5 meshes - each contain 4 sub-meshes:
///    1 even level mesh and its 4 odd level children.
/// Any remaining bytes after the 20 sub-meshes contains water surface meshes,
/// which are ignored.
///
/// Mirrors `processTerrain(buffer, totalSize, transferableObjects)`.
pub fn process_terrain(buffer: &[u8], total_size: usize) -> Result<Vec<Vec<u8>>, RuntimeError> {
    // Find the sub-meshes.
    let advance_mesh = |mut pos: usize| -> Result<usize, RuntimeError> {
        for _ in 0..NUM_SUB_MESHES_PER_MESH {
            let size = read_u32(buffer, pos, true)? as usize;
            pos += SIZE_OF_UINT32;
            pos += size;
            if pos > total_size {
                return Err(RuntimeError::new(Some("Malformed terrain packet found.")));
            }
        }
        Ok(pos)
    };

    let mut offset = 0usize;
    let mut terrain_meshes: Vec<Vec<u8>> = Vec::new();
    while terrain_meshes.len() < NUM_MESHES_PER_PACKET {
        let start = offset;
        offset = advance_mesh(offset)?;
        let mesh = buffer[start..offset].to_vec();
        terrain_meshes.push(mesh);
    }

    Ok(terrain_meshes)
}

const COMPRESSED_MAGIC: u32 = 0x7468_dead;
const COMPRESSED_MAGIC_SWAP: u32 = 0xadde_6874;

/// Mirrors `uncompressPacket(data)`.
///
/// The layout of the decoded data is:
/// Magic Uint32, Size Uint32, [compressed chunk of Size bytes].
pub fn uncompress_packet(data: &[u8]) -> Result<Vec<u8>, RuntimeError> {
    // Pullout magic and verify we have the correct data
    let mut offset = 0usize;
    let magic = read_u32(data, offset, true)?;
    offset += SIZE_OF_UINT32;
    if magic != COMPRESSED_MAGIC && magic != COMPRESSED_MAGIC_SWAP {
        return Err(RuntimeError::new(Some("Invalid magic")));
    }

    // Get the size of the compressed buffer - the endianness depends on which
    // magic was used
    let size = read_u32(data, offset, magic == COMPRESSED_MAGIC)?;
    offset += SIZE_OF_UINT32;

    let compressed_packet = &data[offset.min(data.len())..];
    // pako's `inflate` defaults to the zlib format (windowBits 15, with the
    // 2-byte zlib header and adler32 trailer), not raw deflate.
    let mut decoder = ZlibDecoder::new(compressed_packet);
    let mut uncompressed_packet = Vec::new();
    decoder
        .read_to_end(&mut uncompressed_packet)
        .map_err(|e| RuntimeError::new(Some(&format!("{e}"))))?;

    if uncompressed_packet.len() != size as usize {
        return Err(RuntimeError::new(Some(
            "Size of packet doesn't match header",
        )));
    }

    Ok(uncompressed_packet)
}
