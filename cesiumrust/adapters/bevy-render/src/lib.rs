//! cesium-bevy-render: Bevy rendering adapter
//!
//! This adapter implements the GpuSink port, converting domain geometry (f64)
//! to Bevy meshes (f32) for GPU rendering.
//!
//! # Architecture
//! - `mesh_conversion`: GeometryData → Bevy Mesh (f64 → f32 precision boundary)
//! - `ellipsoid_mesh`: WGS84 ellipsoid mesh generation
//! - `plugin`: Bevy plugin for CesiumRust rendering

use bevy::prelude::*;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::geometry::{self, GeometryData, VertexFormat};

/// Convert f64 GeometryData to Bevy Mesh (f32 precision boundary)
pub fn geometry_to_mesh(geometry: &GeometryData) -> Mesh {
    // Convert positions: f64 → f32
    let positions: Vec<[f32; 3]> = geometry
        .positions
        .iter()
        .map(|p| [p[0] as f32, p[1] as f32, p[2] as f32])
        .collect();

    // Convert normals: f64 → f32 (if present)
    let normals: Vec<[f32; 3]> = geometry
        .normals
        .as_ref()
        .map(|n| n.iter().map(|v| [v[0] as f32, v[1] as f32, v[2] as f32]).collect())
        .unwrap_or_default();

    // Convert texture coordinates: f64 → f32 (if present)
    let uvs: Vec<[f32; 2]> = geometry
        .tex_coords
        .as_ref()
        .map(|t| t.iter().map(|v| [v[0] as f32, v[1] as f32]).collect())
        .unwrap_or_default();

    // Indices are already u32
    let indices = geometry.indices.clone();

    // Create mesh with triangle topology (default for our geometry)
    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::MAIN_WORLD,
    );

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    if !normals.is_empty() {
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    }
    if !uvs.is_empty() {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    }
    if !indices.is_empty() {
        mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
    }

    mesh
}

/// Generate a WGS84 ellipsoid mesh with the given number of subdivisions
///
/// # Arguments
/// * `stacks` - Number of latitude subdivisions (default: 64)
/// * `slices` - Number of longitude subdivisions (default: 128)
///
/// # Returns
/// A Bevy Mesh representing the WGS84 ellipsoid
pub fn create_ellipsoid_mesh(stacks: u32, slices: u32) -> Mesh {
    let radii = Ellipsoid::WGS84.radii();
    let geometry = geometry::ellipsoid_geometry(radii, stacks, slices, VertexFormat::ALL);
    geometry_to_mesh(&geometry)
}

/// Marker component for the globe entity
#[derive(Component)]
pub struct Globe;

/// Resource holding the ellipsoid configuration
#[derive(Resource)]
pub struct EllipsoidConfig {
    pub ellipsoid: Ellipsoid,
    pub stacks: u32,
    pub slices: u32,
}

impl Default for EllipsoidConfig {
    fn default() -> Self {
        Self {
            ellipsoid: Ellipsoid::WGS84,
            stacks: 64,
            slices: 128,
        }
    }
}

/// Plugin that sets up the CesiumRust rendering pipeline
pub struct CesiumRenderPlugin;

impl Plugin for CesiumRenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EllipsoidConfig>()
            .add_systems(Startup, setup_globe);
    }
}

/// System that spawns the globe entity
fn setup_globe(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<EllipsoidConfig>,
) {
    // Create the ellipsoid mesh
    let mesh = create_ellipsoid_mesh(config.stacks, config.slices);

    // Spawn the globe with a solid color material (Earth-like blue)
    commands.spawn((
        Globe,
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.4, 0.7), // Ocean blue
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    // Spawn a camera looking at the globe from a distance
    // WGS84 semi-major axis is ~6378137 meters, so we position camera at ~3x that distance
    let camera_distance = 6378137.0 * 3.0;
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, camera_distance as f32).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Add a directional light (sun)
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.4, 0.0)),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geometry_to_mesh() {
        let radii = Ellipsoid::WGS84.radii();
        let geometry = geometry::ellipsoid_geometry(radii, 8, 16, VertexFormat::ALL);
        let mesh = geometry_to_mesh(&geometry);

        // Verify mesh has position attribute
        assert!(mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
        assert!(mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some());
        assert!(mesh.attribute(Mesh::ATTRIBUTE_UV_0).is_some());
    }

    #[test]
    fn test_create_ellipsoid_mesh() {
        let mesh = create_ellipsoid_mesh(16, 32);
        assert!(mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
    }
}
