//! cesium-bevy-render: Bevy rendering adapter
//!
//! This adapter implements the GpuSink port, converting domain geometry (f64)
//! to Bevy meshes (f32) for GPU rendering.
//!
//! # Architecture
//! - `mesh_conversion`: GeometryData → Bevy Mesh (f64 → f32 precision boundary)
//! - `ellipsoid_mesh`: WGS84 ellipsoid mesh generation
//! - `terrain_render`: TerrainMesh → Bevy Mesh with imagery textures
//! - `scene_pipeline`: SceneGraph → Culling → DrawCommands → Bevy entities
//! - `plugin`: Bevy plugin for CesiumRust rendering

pub mod scene_pipeline;
pub mod entity_render;
pub mod fabric_material;

use bevy::prelude::*;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::geometry::{self, GeometryData, PrimitiveType, VertexFormat};
use cesium_imagery::blending::PixelColor;
use cesium_terrain::terrain_mesh::TerrainMesh;

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

    // Determine topology based on primitive type.
    let topology = match geometry.primitive_type {
        PrimitiveType::Triangles => bevy::render::mesh::PrimitiveTopology::TriangleList,
        PrimitiveType::Lines => bevy::render::mesh::PrimitiveTopology::LineList,
    };

    let mut mesh = Mesh::new(
        topology,
        bevy::render::render_asset::RenderAssetUsages::default(),
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

/// Convert a domain TerrainMesh (f64) to a Bevy Mesh (f32)
pub fn terrain_mesh_to_bevy(terrain: &TerrainMesh) -> Mesh {
    let positions: Vec<[f32; 3]> = terrain
        .positions
        .iter()
        .map(|p| [p[0] as f32, p[1] as f32, p[2] as f32])
        .collect();

    let normals: Vec<[f32; 3]> = terrain
        .normals
        .as_ref()
        .map(|n| n.iter().map(|v| [v[0] as f32, v[1] as f32, v[2] as f32]).collect())
        .unwrap_or_default();

    let uvs: Vec<[f32; 2]> = terrain
        .tex_coords
        .as_ref()
        .map(|t| t.iter().map(|v| [v[0] as f32, v[1] as f32]).collect())
        .unwrap_or_default();

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    if !normals.is_empty() {
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    }
    if !uvs.is_empty() {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    }
    if !terrain.indices.is_empty() {
        mesh.insert_indices(bevy::render::mesh::Indices::U32(terrain.indices.clone()));
    }

    mesh
}

/// Creates a Bevy Image from raw RGBA pixel data for use as an imagery texture.
///
/// # Arguments
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
/// * `rgba_data` - Raw RGBA pixel data (4 bytes per pixel)
///
/// # Returns
/// A Bevy Image asset
pub fn create_imagery_texture(width: u32, height: u32, rgba_data: Vec<u8>) -> Image {
    Image::new(
        bevy::render::render_resource::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        rgba_data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::render::render_asset::RenderAssetUsages::default(),
    )
}

/// Creates a solid color texture from a PixelColor (useful for testing/fallback).
pub fn create_solid_color_texture(color: PixelColor, size: u32) -> Image {
    let r = (color.r.clamp(0.0, 1.0) * 255.0) as u8;
    let g = (color.g.clamp(0.0, 1.0) * 255.0) as u8;
    let b = (color.b.clamp(0.0, 1.0) * 255.0) as u8;
    let a = (color.a.clamp(0.0, 1.0) * 255.0) as u8;

    let pixel = [r, g, b, a];
    let data: Vec<u8> = pixel.iter().cycle().take((size * size * 4) as usize).copied().collect();

    create_imagery_texture(size, size, data)
}

/// Component marking a terrain tile entity with its imagery texture applied.
#[derive(Component)]
pub struct TerrainTile {
    /// Tile X coordinate
    pub x: u32,
    /// Tile Y coordinate
    pub y: u32,
    /// Tile level
    pub level: u32,
}

/// Component for a 3D Tiles tile entity.
#[derive(Component)]
pub struct Tile3D {
    /// Path to the tile in the tileset tree.
    pub path: Vec<usize>,
    /// The content URI (if loaded).
    pub content_uri: Option<String>,
    /// Screen space error at selection time.
    pub screen_space_error: f64,
}

/// Creates a wireframe box mesh for visualizing bounding volumes.
///
/// # Arguments
/// * `center` - Center of the box in ECEF coordinates
/// * `half_x` - Half-axis vector in X direction
/// * `half_y` - Half-axis vector in Y direction
/// * `half_z` - Half-axis vector in Z direction
///
/// # Returns
/// A Bevy Mesh with line topology representing the box edges
pub fn create_bounding_box_wireframe(
    center: glam::DVec3,
    half_x: glam::DVec3,
    half_y: glam::DVec3,
    half_z: glam::DVec3,
) -> Mesh {
    // 8 corners of the box
    let corners: Vec<[f32; 3]> = vec![
        (center - half_x - half_y - half_z).as_vec3().into(),
        (center + half_x - half_y - half_z).as_vec3().into(),
        (center + half_x + half_y - half_z).as_vec3().into(),
        (center - half_x + half_y - half_z).as_vec3().into(),
        (center - half_x - half_y + half_z).as_vec3().into(),
        (center + half_x - half_y + half_z).as_vec3().into(),
        (center + half_x + half_y + half_z).as_vec3().into(),
        (center - half_x + half_y + half_z).as_vec3().into(),
    ];

    // 12 edges (24 indices for line list)
    let indices: Vec<u32> = vec![
        // Bottom face
        0, 1, 1, 2, 2, 3, 3, 0,
        // Top face
        4, 5, 5, 6, 6, 7, 7, 4,
        // Vertical edges
        0, 4, 1, 5, 2, 6, 3, 7,
    ];

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::LineList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, corners);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
    mesh
}

/// Creates a wireframe sphere mesh for visualizing bounding spheres.
///
/// # Arguments
/// * `center` - Center of the sphere in ECEF coordinates
/// * `radius` - Radius of the sphere
/// * `segments` - Number of segments per circle
///
/// # Returns
/// A Bevy Mesh with line topology representing the sphere wireframe
pub fn create_bounding_sphere_wireframe(
    center: glam::DVec3,
    radius: f64,
    segments: u32,
) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let center_f32 = center.as_vec3();
    let radius_f32 = radius as f32;

    // Create 3 circles (XY, XZ, YZ planes)
    for plane in 0..3 {
        let base_index = positions.len() as u32;
        for i in 0..segments {
            let angle = 2.0 * std::f32::consts::PI * (i as f32) / (segments as f32);
            let (sin, cos) = angle.sin_cos();

            let pos = match plane {
                0 => glam::Vec3::new(cos * radius_f32, sin * radius_f32, 0.0), // XY
                1 => glam::Vec3::new(cos * radius_f32, 0.0, sin * radius_f32), // XZ
                _ => glam::Vec3::new(0.0, cos * radius_f32, sin * radius_f32), // YZ
            };

            positions.push((center_f32 + pos).into());

            // Line to next vertex (wrap around)
            let next = if i == segments - 1 { base_index } else { base_index + i + 1 };
            indices.push(base_index + i);
            indices.push(next);
        }
    }

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::LineList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
    mesh
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

/// Render-scale factor: domain works in meters (f64), GPU renders in f32.
/// Earth-scale coordinates (~6.4e6 m) exceed f32 depth/frustum precision,
/// so the adapter scales the world down to a unit sphere for rendering.
/// 1 render unit = 6378137 meters (WGS84 semi-major axis).
pub const METERS_PER_RENDER_UNIT: f64 = 6378137.0;

/// System that spawns the globe entity with a procedural Earth texture.
fn setup_globe(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    config: Res<EllipsoidConfig>,
) {
    // Create the ellipsoid mesh (positions in meters, f32)
    let mesh = create_ellipsoid_mesh(config.stacks, config.slices);

    // Scale factor: meters -> render units (globe renders as a unit sphere)
    let scale = (1.0 / METERS_PER_RENDER_UNIT) as f32;

    // Generate procedural Earth texture
    let earth_texture = generate_earth_texture(512, 256);
    let texture_handle = images.add(earth_texture);

    // Spawn the globe with textured material
    commands.spawn((
        Globe,
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(texture_handle),
            perceptual_roughness: 0.9,
            ..default()
        })),
        Transform::from_scale(Vec3::splat(scale)),
    ));

    // Sun directional light (fixed angle simulating sunlight)
    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::AMBIENT_DAYLIGHT,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.4, 0.6, 0.0)),
    ));
}

/// Generates a procedural Earth-like texture (equirectangular projection).
/// Blue oceans, green/brown landmasses, white polar ice caps.
fn generate_earth_texture(width: u32, height: u32) -> Image {
    let mut data = Vec::with_capacity((width * height * 4) as usize);

    for y in 0..height {
        for x in 0..width {
            let lon = (x as f64 / width as f64) * 360.0 - 180.0; // -180..180
            let lat = 90.0 - (y as f64 / height as f64) * 180.0; // 90..-90

            let (r, g, b) = earth_color_at(lat, lon);
            data.push((r * 255.0) as u8);
            data.push((g * 255.0) as u8);
            data.push((b * 255.0) as u8);
            data.push(255u8);
        }
    }

    create_imagery_texture(width, height, data)
}

/// Simple value noise for continent generation.
fn value_noise(x: f64, y: f64, seed: f64) -> f64 {
    let n = (x * 12.9898 + y * 78.233 + seed * 43758.5453).sin();
    (n * 43758.5453).fract().abs()
}

/// Fractal noise (3 octaves) for more natural coastlines.
fn fractal_noise(lon: f64, lat: f64) -> f64 {
    let mut val = 0.0;
    let mut amp = 1.0;
    let mut freq = 1.0;
    for i in 0..4 {
        let nx = lon * freq / 60.0;
        let ny = lat * freq / 60.0;
        val += value_noise(nx, ny, i as f64 * 7.3) * amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    val / 1.875 // normalize to ~[0, 1]
}

/// Returns (r, g, b) in [0,1] for a given lat/lon.
fn earth_color_at(lat: f64, lon: f64) -> (f64, f64, f64) {
    // Polar ice caps
    if lat.abs() > 70.0 {
        let ice_factor = ((lat.abs() - 70.0) / 20.0).clamp(0.0, 1.0);
        return (
            0.85 + 0.1 * ice_factor,
            0.88 + 0.08 * ice_factor,
            0.92 + 0.05 * ice_factor,
        );
    }

    // Continent mask via fractal noise
    let n = fractal_noise(lon, lat);
    // Bias: more ocean than land (~30% land)
    let is_land = n > 0.52;

    if is_land {
        // Land: green lowlands → brown highlands
        let elevation = (n - 0.52) / 0.48; // 0..1
        if lat.abs() > 55.0 {
            // Tundra/boreal
            (0.35 + 0.1 * elevation, 0.4 + 0.05 * elevation, 0.25)
        } else if lat.abs() < 25.0 {
            // Tropical/desert mix
            if n > 0.68 {
                // Desert
                (0.76, 0.65, 0.42)
            } else {
                // Tropical green
                (0.15 + 0.1 * elevation, 0.5 + 0.15 * elevation, 0.12)
            }
        } else {
            // Temperate
            (0.2 + 0.25 * elevation, 0.45 + 0.1 * elevation, 0.15 + 0.05 * elevation)
        }
    } else {
        // Ocean: deep blue with slight depth variation
        let depth = n / 0.52; // 0..1 (closer to 1 = shallower)
        (
            0.02 + 0.05 * depth,
            0.08 + 0.12 * depth,
            0.35 + 0.2 * depth,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_geospatial::bounding::BoundingSphere;

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

    #[test]
    fn test_terrain_mesh_to_bevy() {
        let terrain = TerrainMesh {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            normals: Some(vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]]),
            tex_coords: Some(vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]),
            indices: vec![0, 1, 2],
            minimum_height: 0.0,
            maximum_height: 0.0,
            bounding_sphere: BoundingSphere::new(glam::DVec3::ZERO, 1.0),
        };

        let mesh = terrain_mesh_to_bevy(&terrain);
        assert!(mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
        assert!(mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some());
        assert!(mesh.attribute(Mesh::ATTRIBUTE_UV_0).is_some());
    }

    #[test]
    fn test_create_imagery_texture() {
        let data = vec![255u8; 4 * 4 * 4]; // 4x4 RGBA
        let image = create_imagery_texture(4, 4, data);
        assert_eq!(image.width(), 4);
        assert_eq!(image.height(), 4);
    }

    #[test]
    fn test_create_solid_color_texture() {
        let color = PixelColor::opaque(1.0, 0.0, 0.0);
        let image = create_solid_color_texture(color, 2);
        assert_eq!(image.width(), 2);
        assert_eq!(image.height(), 2);
        // First pixel should be red (255, 0, 0, 255)
        assert_eq!(image.data[0], 255);
        assert_eq!(image.data[1], 0);
        assert_eq!(image.data[2], 0);
        assert_eq!(image.data[3], 255);
    }

    #[test]
    fn test_create_bounding_box_wireframe() {
        let center = glam::DVec3::ZERO;
        let half_x = glam::DVec3::new(1.0, 0.0, 0.0);
        let half_y = glam::DVec3::new(0.0, 1.0, 0.0);
        let half_z = glam::DVec3::new(0.0, 0.0, 1.0);

        let mesh = create_bounding_box_wireframe(center, half_x, half_y, half_z);

        // Should have 8 vertices (corners)
        let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        if let bevy::render::mesh::VertexAttributeValues::Float32x3(pos) = positions {
            assert_eq!(pos.len(), 8);
        } else {
            panic!("Expected Float32x3 positions");
        }
    }

    #[test]
    fn test_create_bounding_sphere_wireframe() {
        let center = glam::DVec3::ZERO;
        let radius = 100.0;
        let segments = 32;

        let mesh = create_bounding_sphere_wireframe(center, radius, segments);

        // Should have 3 circles * segments vertices
        let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        if let bevy::render::mesh::VertexAttributeValues::Float32x3(pos) = positions {
            assert_eq!(pos.len(), (3 * segments) as usize);
        } else {
            panic!("Expected Float32x3 positions");
        }
    }
}
