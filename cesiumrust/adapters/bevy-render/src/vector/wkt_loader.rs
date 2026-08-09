use bevy::prelude::*;
use cesium_vector::{parse_wkt, WktGeometry};
use glam::DVec2;

#[derive(Resource, Debug, Clone, Default)]
pub struct WktLoadQueue {
    pub pending: Vec<(Entity, String)>,
}

#[derive(Event)]
pub struct WktLoaded {
    pub entity: Entity,
}

pub struct CesiumWktPlugin;

impl Plugin for CesiumWktPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WktLoadQueue>()
            .add_event::<WktLoaded>()
            .add_systems(Update, wkt_load_system);
    }
}

pub fn wkt_load_system(
    mut queue: ResMut<WktLoadQueue>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut events: EventWriter<WktLoaded>,
) {
    let pending = std::mem::take(&mut queue.pending);

    for (entity, wkt_str) in pending {
        if let Ok(geometry) = parse_wkt(&wkt_str) {
            create_entity_from_wkt(entity, &geometry, &mut commands, &mut meshes, &mut materials);
            events.send(WktLoaded { entity });
        }
    }
}

fn create_entity_from_wkt(
    entity: Entity,
    geometry: &WktGeometry,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    match geometry {
        WktGeometry::Point(p) => {
            let positions = vec![[p.x as f32, p.y as f32, 0.0f32]];
            let mut mesh = Mesh::new(
                bevy::render::mesh::PrimitiveTopology::PointList,
                bevy::render::render_asset::RenderAssetUsages::default(),
            );
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            let handle = meshes.add(mesh);
            let mat = materials.add(StandardMaterial {
                base_color: Color::linear_rgb(0.0, 1.0, 1.0),
                unlit: true,
                ..default()
            });
            commands.entity(entity).insert(Mesh3d(handle)).insert(MeshMaterial3d(mat));
        }
        WktGeometry::LineString(coords) => {
            let positions: Vec<[f32; 3]> = coords
                .iter()
                .map(|c| [c.x as f32, c.y as f32, 0.0f32])
                .collect();
            let mut indices = Vec::new();
            for i in 0..(positions.len().saturating_sub(1)) {
                indices.push(i as u32);
                indices.push(i as u32 + 1);
            }
            let mut mesh = Mesh::new(
                bevy::render::mesh::PrimitiveTopology::LineList,
                bevy::render::render_asset::RenderAssetUsages::default(),
            );
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
            let handle = meshes.add(mesh);
            let mat = materials.add(StandardMaterial {
                base_color: Color::linear_rgb(1.0, 1.0, 0.0),
                unlit: true,
                ..default()
            });
            commands.entity(entity).insert(Mesh3d(handle)).insert(MeshMaterial3d(mat));
        }
        WktGeometry::Polygon { exterior, interiors } => {
            let mut positions: Vec<[f32; 3]> = exterior
                .iter()
                .map(|c| [c.x as f32, c.y as f32, 0.0f32])
                .collect();
            let mut indices = triangulate_ring(exterior, positions.len());
            positions.extend(
                interiors
                    .iter()
                    .flat_map(|ring| ring.iter().map(|c| [c.x as f32, c.y as f32, 0.0f32])),
            );
            for (i, ring) in interiors.iter().enumerate() {
                let interior_indices = triangulate_ring(ring, ring.len());
                let offset = exterior.len() as u32
                    + interiors.iter().take(i).map(|r| r.len() as u32).sum::<u32>();
                indices.extend(interior_indices.iter().map(|idx| idx + offset));
            }
            let mut mesh = Mesh::new(
                bevy::render::mesh::PrimitiveTopology::TriangleList,
                bevy::render::render_asset::RenderAssetUsages::default(),
            );
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
            let handle = meshes.add(mesh);
            let mat = materials.add(StandardMaterial {
                base_color: Color::linear_rgb(0.0, 0.5, 1.0),
                ..default()
            });
            commands.entity(entity).insert(Mesh3d(handle)).insert(MeshMaterial3d(mat));
        }
        WktGeometry::MultiPoint(points) => {
            let positions: Vec<[f32; 3]> = points
                .iter()
                .map(|p| [p.x as f32, p.y as f32, 0.0f32])
                .collect();
            let mut mesh = Mesh::new(
                bevy::render::mesh::PrimitiveTopology::PointList,
                bevy::render::render_asset::RenderAssetUsages::default(),
            );
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            let handle = meshes.add(mesh);
            let mat = materials.add(StandardMaterial {
                base_color: Color::linear_rgb(0.0, 1.0, 1.0),
                unlit: true,
                ..default()
            });
            commands.entity(entity).insert(Mesh3d(handle)).insert(MeshMaterial3d(mat));
        }
        WktGeometry::MultiLineString(lines) => {
            let mut positions: Vec<[f32; 3]> = Vec::new();
            let mut indices: Vec<u32> = Vec::new();
            let mut offset = 0u32;
            for line in lines {
                for coord in line {
                    positions.push([coord.x as f32, coord.y as f32, 0.0f32]);
                }
                for i in 0..(line.len().saturating_sub(1)) {
                    indices.push(offset + i as u32);
                    indices.push(offset + i as u32 + 1);
                }
                offset += line.len() as u32;
            }
            let mut mesh = Mesh::new(
                bevy::render::mesh::PrimitiveTopology::LineList,
                bevy::render::render_asset::RenderAssetUsages::default(),
            );
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
            mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
            let handle = meshes.add(mesh);
            let mat = materials.add(StandardMaterial {
                base_color: Color::linear_rgb(1.0, 1.0, 0.0),
                unlit: true,
                ..default()
            });
            commands.entity(entity).insert(Mesh3d(handle)).insert(MeshMaterial3d(mat));
        }
        WktGeometry::MultiPolygon(polygons) => {
            let mut all_positions: Vec<[f32; 3]> = Vec::new();
            let mut all_indices: Vec<u32> = Vec::new();
            for polygon in polygons {
                let offset = all_positions.len() as u32;
                if let WktGeometry::Polygon {
                    ref exterior,
                    ref interiors,
                } = polygon
                {
                    all_positions.extend(
                        exterior.iter().map(|c| [c.x as f32, c.y as f32, 0.0f32]),
                    );
                    let tri = triangulate_ring(exterior, exterior.len());
                    all_indices.extend(tri.iter().map(|idx| idx + offset));

                    let hole_offset = exterior.len() as u32;
                    for (i, ring) in interiors.iter().enumerate() {
                        let off = offset
                            + hole_offset
                            + interiors.iter().take(i).map(|r| r.len() as u32).sum::<u32>();
                        all_positions
                            .extend(ring.iter().map(|c| [c.x as f32, c.y as f32, 0.0f32]));
                        let hole_tri = triangulate_ring(ring, ring.len());
                        all_indices.extend(hole_tri.iter().map(|idx| idx + off));
                    }
                }
            }
            let mut mesh = Mesh::new(
                bevy::render::mesh::PrimitiveTopology::TriangleList,
                bevy::render::render_asset::RenderAssetUsages::default(),
            );
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, all_positions);
            mesh.insert_indices(bevy::render::mesh::Indices::U32(all_indices));
            let handle = meshes.add(mesh);
            let mat = materials.add(StandardMaterial {
                base_color: Color::linear_rgb(0.0, 0.5, 1.0),
                ..default()
            });
            commands.entity(entity).insert(Mesh3d(handle)).insert(MeshMaterial3d(mat));
        }
        WktGeometry::GeometryCollection(geoms) => {
            for geom in geoms {
                create_entity_from_wkt(entity, geom, commands, meshes, materials);
            }
        }
    }
}

fn triangulate_ring(ring: &[DVec2], _ring_len: usize) -> Vec<u32> {
    if ring.len() < 3 {
        return Vec::new();
    }
    let mut indices = Vec::new();
    for i in 1..(ring.len() - 1) {
        indices.push(0);
        indices.push(i as u32);
        indices.push(i as u32 + 1);
    }
    indices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wkt_point_to_entity() {
        let geom = parse_wkt("POINT (10 20)").unwrap();
        assert!(matches!(geom, WktGeometry::Point(_)));
        if let WktGeometry::Point(p) = &geom {
            assert_eq!(p.x, 10.0);
            assert_eq!(p.y, 20.0);
        }
    }

    #[test]
    fn test_wkt_linestring_to_entity() {
        let geom = parse_wkt("LINESTRING (0 0, 10 10, 20 0)").unwrap();
        assert!(matches!(geom, WktGeometry::LineString(_)));
    }

    #[test]
    fn test_wkt_polygon_to_entity() {
        let geom = parse_wkt("POLYGON ((0 0, 10 0, 10 10, 0 10, 0 0))").unwrap();
        assert!(matches!(geom, WktGeometry::Polygon { .. }));
    }

    #[test]
    fn test_wkt_multipoint_to_entity() {
        let geom = parse_wkt("MULTIPOINT ((0 0), (10 10), (20 20))").unwrap();
        match geom {
            WktGeometry::MultiPoint(points) => assert_eq!(points.len(), 3),
            _ => panic!("Expected MultiPoint"),
        }
    }

    #[test]
    fn test_wkt_multilinestring_to_entity() {
        let geom =
            parse_wkt("MULTILINESTRING ((0 0, 10 10), (20 20, 30 30))").unwrap();
        match geom {
            WktGeometry::MultiLineString(lines) => assert_eq!(lines.len(), 2),
            _ => panic!("Expected MultiLineString"),
        }
    }

    #[test]
    fn test_wkt_geometry_collection() {
        let geom =
            parse_wkt("GEOMETRYCOLLECTION (POINT (4 6), LINESTRING (4 6, 7 10))").unwrap();
        match geom {
            WktGeometry::GeometryCollection(geoms) => assert_eq!(geoms.len(), 2),
            _ => panic!("Expected GeometryCollection"),
        }
    }

    #[test]
    fn test_triangulate_ring_triangle() {
        let ring = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(1.0, 0.0),
            DVec2::new(0.0, 1.0),
        ];
        let indices = triangulate_ring(&ring, 3);
        assert_eq!(indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_triangulate_ring_quad() {
        let ring = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(1.0, 0.0),
            DVec2::new(1.0, 1.0),
            DVec2::new(0.0, 1.0),
        ];
        let indices = triangulate_ring(&ring, 4);
        assert_eq!(indices.len(), 6);
    }
}
