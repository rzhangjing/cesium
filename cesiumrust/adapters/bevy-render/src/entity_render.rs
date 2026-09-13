//! Entity visualization rendering.
//!
//! Converts domain Entity graphics to Bevy meshes and materials.
//! Maps to CesiumJS `DataSources/GeometryVisualizer.js`

use bevy::prelude::*;
use cesium_datasource::entity::{Entity, PolygonGraphics, PolylineGraphics};
use cesium_datasource::property::{Color, Property};
use cesium_geospatial::cartographic::Cartographic;
use cesium_geospatial::ellipsoid::Ellipsoid;

/// Component marking an entity visualization.
#[derive(Component)]
pub struct EntityVisual {
    /// The entity ID this visual represents.
    pub entity_id: String,
}

/// Converts a domain Color to a Bevy Color.
pub fn domain_color_to_bevy(color: &Color) -> bevy::prelude::Color {
    bevy::prelude::Color::srgba(
        color.red as f32,
        color.green as f32,
        color.blue as f32,
        color.alpha as f32,
    )
}

/// Resolves a color property at time 0.
fn resolve_color(prop: &Property<Color>, default: Color) -> Color {
    prop.get_value(0.0).copied().unwrap_or(default)
}

/// Creates a polyline mesh from positions on the ellipsoid.
///
/// Generates a triangle strip along the line with the given width.
pub fn create_polyline_mesh(
    polyline: &PolylineGraphics,
    ellipsoid: &Ellipsoid,
    time: f64,
) -> Option<Mesh> {
    let positions = polyline.positions.get_value(time)?;
    if positions.len() < 2 {
        return None;
    }

    let width = polyline.width.get_value(time).copied().unwrap_or(1.0);
    // Convert pixel width to approximate world width (rough heuristic)
    let world_width = width * 1000.0; // Approximate meters per pixel at medium zoom

    // Convert cartographic positions to ECEF
    let ecef_points: Vec<glam::DVec3> = positions
        .iter()
        .map(|p| {
            let carto = Cartographic::from_radians(p[0], p[1], p[2]);
            ellipsoid.cartographic_to_cartesian(&carto)
        })
        .collect();

    // Generate a flat ribbon (triangle strip) along the line
    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    for i in 0..ecef_points.len() {
        let point = ecef_points[i];
        let normal = point.normalize(); // Surface normal (approximate)

        // Compute tangent direction
        let tangent = if i < ecef_points.len() - 1 {
            (ecef_points[i + 1] - point).normalize()
        } else {
            (point - ecef_points[i - 1]).normalize()
        };

        // Side vector (perpendicular to tangent and normal)
        let side = tangent.cross(normal).normalize();

        // Two vertices per point (left and right of center)
        let half_width = world_width / 2.0;
        let left = point + side * half_width;
        let right = point - side * half_width;

        vertices.push([left.x as f32, left.y as f32, left.z as f32]);
        vertices.push([right.x as f32, right.y as f32, right.z as f32]);

        let n = [normal.x as f32, normal.y as f32, normal.z as f32];
        normals.push(n);
        normals.push(n);

        // Generate triangle indices
        if i < ecef_points.len() - 1 {
            let base = (i * 2) as u32;
            indices.extend_from_slice(&[base, base + 1, base + 2]);
            indices.extend_from_slice(&[base + 1, base + 3, base + 2]);
        }
    }

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));

    Some(mesh)
}

/// Creates a polygon mesh from positions on the ellipsoid.
///
/// Uses simple fan triangulation for convex polygons.
pub fn create_polygon_mesh(
    polygon: &PolygonGraphics,
    ellipsoid: &Ellipsoid,
    time: f64,
) -> Option<Mesh> {
    let positions = polygon.positions.get_value(time)?;
    if positions.len() < 3 {
        return None;
    }

    let height = polygon.height.get_value(time).copied().unwrap_or(0.0);

    // Convert to ECEF
    let ecef_points: Vec<glam::DVec3> = positions
        .iter()
        .map(|p| {
            let carto = Cartographic::from_radians(p[0], p[1], height);
            ellipsoid.cartographic_to_cartesian(&carto)
        })
        .collect();

    // Compute centroid for fan triangulation
    let centroid = ecef_points.iter().fold(glam::DVec3::ZERO, |acc, p| acc + *p)
        / ecef_points.len() as f64;
    let centroid_normal = centroid.normalize();

    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // Add centroid vertex
    vertices.push([centroid.x as f32, centroid.y as f32, centroid.z as f32]);
    normals.push([centroid_normal.x as f32, centroid_normal.y as f32, centroid_normal.z as f32]);

    // Add ring vertices
    for point in &ecef_points {
        vertices.push([point.x as f32, point.y as f32, point.z as f32]);
        let n = point.normalize();
        normals.push([n.x as f32, n.y as f32, n.z as f32]);
    }

    // Fan triangulation from centroid
    let n_points = ecef_points.len();
    for i in 0..n_points {
        let next = (i + 1) % n_points;
        indices.extend_from_slice(&[0, (i + 1) as u32, (next + 1) as u32]);
    }

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));

    Some(mesh)
}

/// Creates a Bevy material from entity graphics.
pub fn create_entity_material(entity: &Entity, _time: f64) -> StandardMaterial {
    // Try to get color from different graphics types
    let color = if let Some(ref point) = entity.point {
        resolve_color(&point.color, Color::WHITE)
    } else if let Some(ref polyline) = entity.polyline {
        resolve_color(&polyline.color, Color::WHITE)
    } else if let Some(ref polygon) = entity.polygon {
        resolve_color(&polygon.material, Color::WHITE)
    } else {
        Color::WHITE
    };

    StandardMaterial {
        base_color: domain_color_to_bevy(&color),
        ..default()
    }
}

/// Converts an entity's position to a Bevy Transform on the ellipsoid.
pub fn entity_position_to_transform(
    entity: &Entity,
    ellipsoid: &Ellipsoid,
    time: f64,
) -> Option<Transform> {
    let pos = entity.position.get_value(time)?;
    let carto = Cartographic::from_radians(pos[0], pos[1], pos[2]);
    let ecef = ellipsoid.cartographic_to_cartesian(&carto);

    Some(Transform::from_xyz(ecef.x as f32, ecef.y as f32, ecef.z as f32))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_datasource::entity::{PointGraphics, PolylineGraphics, PolygonGraphics};
    use cesium_datasource::property::Property;

    #[test]
    fn test_domain_color_to_bevy() {
        let color = Color::new(1.0, 0.5, 0.25, 1.0);
        let bevy_color = domain_color_to_bevy(&color);
        if let bevy::prelude::Color::Srgba(srgba) = bevy_color {
            assert!((srgba.red - 1.0).abs() < 1e-5);
            assert!((srgba.green - 0.5).abs() < 1e-5);
            assert!((srgba.blue - 0.25).abs() < 1e-5);
        } else {
            panic!("Expected Srgba color");
        }
    }

    #[test]
    fn test_create_polyline_mesh() {
        let polyline = PolylineGraphics {
            positions: Property::Constant(vec![
                [0.0, 0.0, 0.0],
                [0.01, 0.0, 0.0],
                [0.02, 0.0, 0.0],
            ]),
            width: Property::Constant(2.0),
            ..Default::default()
        };

        let mesh = create_polyline_mesh(&polyline, &Ellipsoid::WGS84, 0.0);
        assert!(mesh.is_some());

        let mesh = mesh.unwrap();
        assert!(mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
        assert!(mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some());
    }

    #[test]
    fn test_create_polygon_mesh() {
        let polygon = PolygonGraphics {
            positions: Property::Constant(vec![
                [0.0, 0.0, 0.0],
                [0.01, 0.0, 0.0],
                [0.01, 0.01, 0.0],
                [0.0, 0.01, 0.0],
            ]),
            ..Default::default()
        };

        let mesh = create_polygon_mesh(&polygon, &Ellipsoid::WGS84, 0.0);
        assert!(mesh.is_some());

        let mesh = mesh.unwrap();
        let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        if let bevy::render::mesh::VertexAttributeValues::Float32x3(pos) = positions {
            // Centroid + 4 ring vertices = 5
            assert_eq!(pos.len(), 5);
        }
    }

    #[test]
    fn test_create_entity_material() {
        let entity = Entity::new("test")
            .with_point(PointGraphics {
                color: Property::Constant(Color::RED),
                ..Default::default()
            });

        let material = create_entity_material(&entity, 0.0);
        if let bevy::prelude::Color::Srgba(srgba) = material.base_color {
            assert!((srgba.red - 1.0).abs() < 1e-5);
            assert!((srgba.green - 0.0).abs() < 1e-5);
        }
    }

    #[test]
    fn test_entity_position_to_transform() {
        let entity = Entity::new("test")
            .with_position(0.0, 0.0, 0.0); // lon=0, lat=0, h=0

        let transform = entity_position_to_transform(&entity, &Ellipsoid::WGS84, 0.0);
        assert!(transform.is_some());

        let t = transform.unwrap();
        // At lon=0, lat=0, the position should be on the X axis (approximately 6378137m)
        assert!(t.translation.x > 6_000_000.0);
        assert!(t.translation.y.abs() < 1.0);
        assert!(t.translation.z.abs() < 1.0);
    }

    #[test]
    fn test_polyline_too_few_points() {
        let polyline = PolylineGraphics {
            positions: Property::Constant(vec![[0.0, 0.0, 0.0]]),
            ..Default::default()
        };

        let mesh = create_polyline_mesh(&polyline, &Ellipsoid::WGS84, 0.0);
        assert!(mesh.is_none());
    }
}
