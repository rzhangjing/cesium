use bevy::prelude::*;
use cesium_vector::{
    decode_mvt_geometry, MvtFeature, MvtGeometryType, MvtLayer, MvtValue,
};
use glam::DVec3;

#[derive(Resource, Debug, Clone)]
pub struct VectorTileConfig {
    pub enabled: bool,
    pub clamp_to_ground: bool,
    pub default_point_size: f64,
    pub default_polyline_width: f64,
}

impl Default for VectorTileConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            clamp_to_ground: false,
            default_point_size: 5.0,
            default_polyline_width: 2.0,
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct MvtTileData {
    pub layers: Vec<MvtLayer>,
    pub tile_x: u32,
    pub tile_y: u32,
    pub tile_z: u32,
}

#[derive(Component, Debug, Clone)]
pub struct PointGraphics {
    pub positions: Vec<DVec3>,
    pub colors: Vec<[f64; 4]>,
    pub sizes: Vec<f64>,
}

#[derive(Component, Debug, Clone)]
pub struct PolylineGraphics {
    pub positions: Vec<DVec3>,
    pub colors: Vec<[f64; 4]>,
    pub widths: Vec<f64>,
    pub polyline_counts: Vec<usize>,
}

#[derive(Component, Debug, Clone)]
pub struct PolygonGraphics {
    pub positions: Vec<DVec3>,
    pub indices: Vec<u32>,
    pub colors: Vec<[f64; 4]>,
}

#[derive(Event)]
pub struct MvtTileLoaded {
    pub entity: Entity,
}

pub struct CesiumVectorTilePlugin;

impl Plugin for CesiumVectorTilePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VectorTileConfig>()
            .add_event::<MvtTileLoaded>()
            .add_systems(
                Update,
                (decode_mvt_system, clamp_to_ground_system, render_vector_system),
            );
    }
}

pub fn decode_mvt_system(
    config: Res<VectorTileConfig>,
    mvt_query: Query<(Entity, &MvtTileData), Added<MvtTileData>>,
    mut commands: Commands,
) {
    if !config.enabled {
        return;
    }

    for (entity, tile_data) in mvt_query.iter() {
        for layer in &tile_data.layers {
            for feature in &layer.features {
                let rings = decode_mvt_geometry(&feature.geometry, layer.extent.max(1));
                let properties = extract_properties(feature, &layer.keys, &layer.values);

                let color = properties
                    .get("color")
                    .and_then(|v| parse_color(v))
                    .unwrap_or([1.0, 1.0, 1.0, 1.0]);

                match feature.geometry_type {
                    MvtGeometryType::Point => {
                        let positions: Vec<DVec3> = rings
                            .iter()
                            .filter_map(|r| r.first().copied())
                            .collect();

                        let sizes = vec![config.default_point_size; positions.len()];
                        let colors = vec![color; positions.len()];

                        commands.entity(entity).insert(PointGraphics {
                            positions,
                            colors,
                            sizes,
                        });
                    }
                    MvtGeometryType::LineString => {
                        let mut positions: Vec<DVec3> = Vec::new();
                        let mut polyline_counts: Vec<usize> = Vec::new();

                        for ring in &rings {
                            if ring.len() >= 2 {
                                polyline_counts.push(ring.len());
                                positions.extend_from_slice(ring);
                            }
                        }

                        if !positions.is_empty() {
                            let widths = vec![config.default_polyline_width; polyline_counts.len()];
                            let colors = vec![color; positions.len()];

                            commands.entity(entity).insert(PolylineGraphics {
                                positions,
                                colors,
                                widths,
                                polyline_counts,
                            });
                        }
                    }
                    MvtGeometryType::Polygon => {
                        let mut all_positions: Vec<DVec3> = Vec::new();
                        let mut all_indices: Vec<u32> = Vec::new();

                        for ring in &rings {
                            if ring.len() < 3 {
                                continue;
                            }
                            let base = all_positions.len() as u32;
                            all_positions.extend_from_slice(ring);
                            for i in 1..(ring.len() - 1) {
                                all_indices.push(base);
                                all_indices.push(base + i as u32);
                                all_indices.push(base + i as u32 + 1);
                            }
                        }

                        if !all_positions.is_empty() {
                            let colors = vec![color; all_positions.len()];

                            commands.entity(entity).insert(PolygonGraphics {
                                positions: all_positions,
                                indices: all_indices,
                                colors,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn extract_properties(
    feature: &MvtFeature,
    keys: &[String],
    values: &[MvtValue],
) -> std::collections::HashMap<String, String> {
    let mut props = std::collections::HashMap::new();
    let mut i = 0;
    while i + 1 < feature.tags.len() {
        let key_idx = feature.tags[i] as usize;
        let val_idx = feature.tags[i + 1] as usize;
        if let (Some(key), Some(val)) = (keys.get(key_idx), values.get(val_idx)) {
            props.insert(key.clone(), mvt_value_to_string(val));
        }
        i += 2;
    }
    props
}

fn mvt_value_to_string(value: &MvtValue) -> String {
    match value {
        MvtValue::String(s) => s.clone(),
        MvtValue::Float(f) => format!("{}", f),
        MvtValue::Double(d) => format!("{}", d),
        MvtValue::Int(i) => format!("{}", i),
        MvtValue::Uint(u) => format!("{}", u),
        MvtValue::Sint(i) => format!("{}", i),
        MvtValue::Bool(b) => format!("{}", b),
    }
}

fn parse_color(color_str: &str) -> Option<[f64; 4]> {
    let parts: Vec<f64> = color_str
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    if parts.len() >= 3 {
        let a = parts.get(3).copied().unwrap_or(1.0);
        Some([parts[0], parts[1], parts[2], a])
    } else {
        None
    }
}

pub fn clamp_to_ground_system(
    config: Res<VectorTileConfig>,
    mut point_query: Query<&mut PointGraphics>,
    mut polyline_query: Query<&mut PolylineGraphics>,
    mut polygon_query: Query<&mut PolygonGraphics>,
) {
    if !config.enabled || !config.clamp_to_ground {
        return;
    }

    for mut points in point_query.iter_mut() {
        for pos in &mut points.positions {
            pos.z = 0.0;
        }
    }

    for mut polylines in polyline_query.iter_mut() {
        for pos in &mut polylines.positions {
            pos.z = 0.0;
        }
    }

    for mut polygons in polygon_query.iter_mut() {
        for pos in &mut polygons.positions {
            pos.z = 0.0;
        }
    }
}

pub fn render_vector_system(
    _config: Res<VectorTileConfig>,
    point_query: Query<(Entity, &PointGraphics), Added<PointGraphics>>,
    polyline_query: Query<(Entity, &PolylineGraphics), Added<PolylineGraphics>>,
    polygon_query: Query<(Entity, &PolygonGraphics), Added<PolygonGraphics>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, points) in point_query.iter() {
        let mut mesh = Mesh::new(
            bevy::render::mesh::PrimitiveTopology::PointList,
            bevy::render::render_asset::RenderAssetUsages::default(),
        );
        let positions: Vec<[f32; 3]> = points
            .positions
            .iter()
            .map(|p| [p.x as f32, p.y as f32, p.z as f32])
            .collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        let handle = meshes.add(mesh);
        let mat = materials.add(StandardMaterial {
            base_color: Color::linear_rgb(1.0, 1.0, 1.0),
            unlit: true,
            ..default()
        });
        commands.entity(entity).insert(Mesh3d(handle)).insert(MeshMaterial3d(mat));
    }

    for (entity, polylines) in polyline_query.iter() {
        let mut mesh = Mesh::new(
            bevy::render::mesh::PrimitiveTopology::LineList,
            bevy::render::render_asset::RenderAssetUsages::default(),
        );
        let positions: Vec<[f32; 3]> = polylines
            .positions
            .iter()
            .map(|p| [p.x as f32, p.y as f32, p.z as f32])
            .collect();

        let mut indices: Vec<u32> = Vec::new();
        let mut offset = 0u32;
        for &count in &polylines.polyline_counts {
            for i in 0..(count.saturating_sub(1)) {
                indices.push(offset + i as u32);
                indices.push(offset + i as u32 + 1);
            }
            offset += count as u32;
        }

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
        let handle = meshes.add(mesh);
        let mat = materials.add(StandardMaterial {
            base_color: Color::linear_rgb(1.0, 1.0, 1.0),
            unlit: true,
            ..default()
        });
        commands.entity(entity).insert(Mesh3d(handle)).insert(MeshMaterial3d(mat));
    }

    for (entity, polygons) in polygon_query.iter() {
        let mut mesh = Mesh::new(
            bevy::render::mesh::PrimitiveTopology::TriangleList,
            bevy::render::render_asset::RenderAssetUsages::default(),
        );
        let positions: Vec<[f32; 3]> = polygons
            .positions
            .iter()
            .map(|p| [p.x as f32, p.y as f32, p.z as f32])
            .collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_indices(bevy::render::mesh::Indices::U32(polygons.indices.clone()));
        let handle = meshes.add(mesh);
        let mat = materials.add(StandardMaterial {
            base_color: Color::linear_rgb(0.8, 0.8, 0.8),
            ..default()
        });
        commands.entity(entity).insert(Mesh3d(handle)).insert(MeshMaterial3d(mat));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_tile_config_default() {
        let config = VectorTileConfig::default();
        assert!(config.enabled);
        assert!(!config.clamp_to_ground);
        assert_eq!(config.default_point_size, 5.0);
        assert_eq!(config.default_polyline_width, 2.0);
    }

    #[test]
    fn test_parse_color() {
        assert_eq!(parse_color("1.0,0.5,0.2"), Some([1.0, 0.5, 0.2, 1.0]));
        assert_eq!(parse_color("1.0,0.5,0.2,0.8"), Some([1.0, 0.5, 0.2, 0.8]));
        assert_eq!(parse_color("not a color"), None);
    }

    #[test]
    fn test_extract_properties() {
        let feat = MvtFeature {
            id: Some(1),
            geometry_type: MvtGeometryType::Point,
            geometry: vec![],
            tags: vec![0, 0, 1, 1],
        };
        let keys = vec!["name".to_string(), "value".to_string()];
        let values = vec![
            MvtValue::String("test".to_string()),
            MvtValue::Float(42.0),
        ];
        let props = extract_properties(&feat, &keys, &values);
        assert_eq!(props.get("name").unwrap(), "test");
        assert_eq!(props.get("value").unwrap(), "42");
    }

    #[test]
    fn test_decode_mvt_geometry_basic() {
        let commands = vec![
            (1 << 3) | 1, // MoveTo, count=1
            24,           // zigzag(12)
            16,           // zigzag(8)
        ];
        let rings = decode_mvt_geometry(&commands, 4096);
        assert_eq!(rings.len(), 1);
        assert_eq!(rings[0].len(), 1);
    }
}
