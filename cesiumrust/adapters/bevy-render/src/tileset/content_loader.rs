use std::collections::HashMap;

use bevy::prelude::*;
use cesium_network::HttpTileFetcher;
use cesium_ports_driven::TileFetcher;
use cesium_tileset::content_decoder::{
    decode_tile_content, DecodedTile,
};
#[cfg(test)]
use cesium_tileset::content_decoder::{detect_content_type, TileContentType};

use crate::components::{CesiumTileNode, TileContent, TileContentState};
use crate::resources::TileLoadStats;

use super::loader::LoadedTileset;
use super::traversal_system::TileSelection;

#[derive(Resource, Default)]
pub struct PendingTileLoads {
    pub pending: HashMap<String, TileLoadRequest>,
}

pub struct TileLoadRequest {
    pub path: Vec<usize>,
    pub url: String,
}

pub fn tile_content_load_system(
    mut commands: Commands,
    loaded: Option<Res<LoadedTileset>>,
    mut selection: ResMut<TileSelection>,
    mut pending_loads: ResMut<PendingTileLoads>,
    mut stats: ResMut<TileLoadStats>,
    tile_query: Query<(Entity, &CesiumTileNode)>,
) {
    let loaded = match loaded {
        Some(l) => l,
        None => return,
    };

    let tileset_json = match &loaded.tileset_json {
        Some(ts) => ts,
        None => return,
    };

    let base_path = &loaded.state;

    for path in selection.tiles_to_load.drain(..) {
        let tile = match cesium_tileset::lod_selection::get_tile_by_path(&tileset_json.root, &path) {
            Some(t) => t,
            None => continue,
        };

        let content_uri = match &tile.content {
            Some(c) => &c.uri,
            None => continue,
        };

        let full_url = base_path.resolve_uri(content_uri);

        if pending_loads.pending.contains_key(&full_url) {
            continue;
        }

        pending_loads.pending.insert(
            full_url.clone(),
            TileLoadRequest {
                path: path.clone(),
                url: full_url.clone(),
            },
        );

        match fetch_and_decode_tile(&full_url) {
            Ok(glb_bytes) => {
                match parse_glb_to_geometry(&glb_bytes) {
                    Ok(geometry_data) => {
                        let _bevy_mesh = crate::geometry_to_mesh(&geometry_data);

                        let existing = tile_query
                            .iter()
                            .find(|(_, node)| node.path == path)
                            .map(|(e, _)| e);

                        if let Some(entity) = existing {
                            commands.entity(entity).insert((
                                TileContent {
                                    mesh_handle: None,
                                    material_handle: None,
                                    has_batch_table: false,
                                },
                                CesiumTileNode {
                                    state: TileContentState::Ready,
                                    geometric_error: tile.geometric_error,
                                    screen_space_error: 0.0,
                                    path: path.clone(),
                                    bounding_sphere_center: None,
                                    bounding_sphere_radius: None,
                                },
                            ));
                        } else {
                            commands.spawn((
                                CesiumTileNode {
                                    path: path.clone(),
                                    screen_space_error: 0.0,
                                    geometric_error: tile.geometric_error,
                                    state: TileContentState::Ready,
                                    bounding_sphere_center: None,
                                    bounding_sphere_radius: None,
                                },
                                TileContent {
                                    mesh_handle: None,
                                    material_handle: None,
                                    has_batch_table: false,
                                },
                                Transform::default(),
                                Visibility::default(),
                            ));
                        }

                        stats.tiles_loaded += 1;
                    }
                    Err(e) => {
                        error!("Failed to parse glTF for tile {:?}: {}", path, e);
                        stats.tiles_failed += 1;
                    }
                }
            }
            Err(e) => {
                error!("Failed to fetch tile {:?}: {}", path, e);
                stats.tiles_failed += 1;

                let existing = tile_query
                    .iter()
                    .find(|(_, node)| node.path == path)
                    .map(|(e, _)| e);

                if let Some(entity) = existing {
                    commands.entity(entity).insert(CesiumTileNode {
                        state: TileContentState::Failed,
                        geometric_error: tile.geometric_error,
                        screen_space_error: 0.0,
                        path: path.clone(),
                        bounding_sphere_center: None,
                        bounding_sphere_radius: None,
                    });
                }
            }
        }

        pending_loads.pending.remove(&full_url);
    }
}

fn fetch_and_decode_tile(url: &str) -> Result<Vec<u8>, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("Failed to create runtime: {}", e))?;

    let fetcher = HttpTileFetcher::new(url);
    let raw_bytes = runtime
        .block_on(async { fetcher.fetch(url, 0.5).await })
        .map_err(|e| format!("Fetch error: {:?}", e))?;

    let decoded = decode_tile_content(&raw_bytes).map_err(|e| format!("Decode error: {}", e))?;

    match decoded {
        DecodedTile::B3dm(b3dm) => Ok(b3dm.gltf),
        DecodedTile::Glb(glb) => Ok(glb),
        _ => Err("Unsupported tile content type".to_string()),
    }
}

fn parse_glb_to_geometry(glb: &[u8]) -> Result<cesium_geospatial::geometry::GeometryData, String> {
    if glb.len() < 12 || &glb[0..4] != b"glTF" {
        return Err("Not a valid GLB".to_string());
    }

    let version = u32::from_le_bytes([glb[4], glb[5], glb[6], glb[7]]);
    if version != 2 {
        return Err(format!("Unsupported glTF version: {}", version));
    }

    let json_chunk_length = u32::from_le_bytes([glb[12], glb[13], glb[14], glb[15]]) as usize;
    let json_start = 16;
    let json_end = json_start + json_chunk_length;
    if json_end > glb.len() {
        return Err("JSON chunk exceeds buffer".to_string());
    }

    let json_bytes = &glb[json_start..json_end];
    let gltf: cesium_gltf::gltf_model::GltfModel =
        serde_json::from_slice(json_bytes).map_err(|e| format!("glTF JSON parse: {}", e))?;

    let bin_offset = json_end;
    let bin_chunk_header = 8;
    let bin_data_start = bin_offset + bin_chunk_header;
    let bin_data = if bin_data_start < glb.len() {
        glb[bin_data_start..].to_vec()
    } else {
        vec![]
    };

    let buffers: Vec<Vec<u8>> = vec![bin_data];

    let mut positions: Vec<[f64; 3]> = Vec::new();
    let mut normals: Vec<[f64; 3]> = Vec::new();
    let mut tex_coords: Vec<[f64; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut index_offset: u32 = 0;

    for mesh in &gltf.meshes {
        for prim in &mesh.primitives {
            let pos_count = prim
                .attributes
                .get("POSITION")
                .and_then(|&idx| gltf.accessors.get(idx))
                .map(|a| a.count)
                .unwrap_or(0);

            if let Some(&pos_idx) = prim.attributes.get("POSITION") {
                if let Some(acc) = gltf.accessors.get(pos_idx) {
                    let float_data = acc.read_f32_data(&buffers, &gltf.buffer_views);
                    for chunk in float_data.chunks_exact(3) {
                        positions.push([chunk[0] as f64, chunk[1] as f64, chunk[2] as f64]);
                    }
                }
            }

            if let Some(&nrm_idx) = prim.attributes.get("NORMAL") {
                if let Some(acc) = gltf.accessors.get(nrm_idx) {
                    let float_data = acc.read_f32_data(&buffers, &gltf.buffer_views);
                    for chunk in float_data.chunks_exact(3) {
                        normals.push([chunk[0] as f64, chunk[1] as f64, chunk[2] as f64]);
                    }
                }
            }

            if let Some(&uv_idx) = prim.attributes.get("TEXCOORD_0") {
                if let Some(acc) = gltf.accessors.get(uv_idx) {
                    let float_data = acc.read_f32_data(&buffers, &gltf.buffer_views);
                    for chunk in float_data.chunks_exact(2) {
                        tex_coords.push([chunk[0] as f64, chunk[1] as f64]);
                    }
                }
            }

            if let Some(idx_idx) = prim.indices {
                if let Some(acc) = gltf.accessors.get(idx_idx) {
                    let use_u16 = matches!(
                        acc.component_type,
                        cesium_gltf::gltf_model::ComponentType::U16
                    );
                    if use_u16 {
                        let idx_data = acc.read_u16_data(&buffers, &gltf.buffer_views);
                        for idx in &idx_data {
                            indices.push(*idx as u32 + index_offset);
                        }
                    } else {
                        let idx_data = acc.read_u32_data(&buffers, &gltf.buffer_views);
                        for idx in &idx_data {
                            indices.push(*idx + index_offset);
                        }
                    }
                }
            }

            index_offset += pos_count as u32;
        }
    }

    if positions.is_empty() {
        return Err("No vertex positions found".to_string());
    }

    let bounding_sphere = cesium_geospatial::bounding::BoundingSphere::from_points(
        &positions
            .iter()
            .map(|p| glam::DVec3::new(p[0], p[1], p[2]))
            .collect::<Vec<_>>(),
    );

    Ok(cesium_geospatial::geometry::GeometryData {
        positions,
        normals: if normals.is_empty() { None } else { Some(normals) },
        tex_coords: if tex_coords.is_empty() { None } else { Some(tex_coords) },
        tangents: None,
        bitangents: None,
        indices,
        bounding_sphere,
        primitive_type: cesium_geospatial::geometry::PrimitiveType::Triangles,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_content_types() {
        assert_eq!(
            detect_content_type(b"b3dm...."),
            TileContentType::Batched3DModel
        );
        assert_eq!(
            detect_content_type(b"i3dm...."),
            TileContentType::Instanced3DModel
        );
        assert_eq!(
            detect_content_type(b"glTF...."),
            TileContentType::GltfBinary
        );
        assert_eq!(detect_content_type(b"xxxx...."), TileContentType::Unknown);
    }

    #[test]
    fn test_decode_b3dm_to_gltf() {
        let gltf_content = b"glTF test glb data here".to_vec();
        let b3dm = cesium_tileset::content_decoder::B3dmContent {
            batch_length: 1,
            feature_table_json: None,
            feature_table_binary: vec![],
            batch_table_json: None,
            batch_table_binary: vec![],
            gltf: gltf_content.clone(),
        };
        let decoded = DecodedTile::B3dm(b3dm);
        match decoded {
            DecodedTile::B3dm(c) => assert_eq!(c.gltf, gltf_content),
            _ => panic!("Expected B3dm"),
        }
    }

    #[test]
    fn test_pending_loads_default() {
        let pending = PendingTileLoads::default();
        assert!(pending.pending.is_empty());
    }
}
