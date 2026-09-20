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

pub mod camera;
pub mod components;
pub mod datasource;
pub mod entity;
pub mod entity_render;
pub mod fabric_material;
pub mod imagery;
pub mod material_system;
pub mod resources;
pub mod scene_pipeline;
pub mod shader_registry;
pub mod terrain;
pub mod tileset;
pub mod atmosphere;
pub mod effects;
pub mod voxel;
pub mod vector;
pub mod shadow;
pub mod widgets;
pub mod pipeline;
pub mod headless;

pub use camera::{
    camera_control_port_system, CameraControlImpl, CameraControlPort, CameraState, CesiumCamera,
    CesiumCameraPlugin, FlyToRequest,
};
pub use components::{
    CesiumGlobe, CesiumImageryLayer, CesiumTerrainTile, CesiumTileNode, CesiumTilesetRoot,
    TileContent, TileContentState, TilesetLoadingState,
};
pub use datasource::{
    CesiumDataSourcePlugin,
    czml_loader::{CzmlLoadPlugin, CzmlLoadQueue, load_czml_file},
    geojson_loader::{GeoJsonLoadPlugin, GeoJsonLoadQueue, load_geojson_file},
    gpx_loader::{GpxLoadPlugin, GpxLoadQueue, load_gpx_file},
    kml_loader::{KmlLoadPlugin, KmlLoadQueue, load_kml_file},
};
pub use entity::{
    CesiumEntityPlugin,
    components::{
        BillboardGraphicsComponent, BillboardTag, CesiumEntity, EntityWrapper, GlobeEllipsoid,
        ModelGraphicsComponent, NeedsVisualUpdate, PointGraphicsComponent,
        PolygonGraphicsComponent, PolylineGraphicsComponent, TimeDynamicProperties,
        VisualizationBuilt,
    },
    time_system::{AnimationClock, entity_visibility_system, time_dynamic_update_system},
    visualizer::{
        billboard_face_camera_system, entity_visualizer_system,
    },
};
pub use fabric_material::{FabricKind, FabricMaterial, FabricMaterialPlugin, FabricParams};
pub use imagery::CesiumImageryPlugin;
pub use material_system::{
    CesiumMaterialPlugin, MaterialAnimationTime, MaterialRef, MaterialSystemResource,
};
pub use resources::{GlobeConfig, RenderScale, TileLoadStats, METERS_PER_RENDER_UNIT};
pub use terrain::CesiumTerrainPlugin;
pub use tileset::CesiumTilesetPlugin;
pub use tileset::debug_plugin::DebugPlugin;
pub use tileset::debug_system::DebugConfig;
pub use tileset::picking::{TilePickEvent, TilePickingPlugin};

pub use atmosphere::CesiumAtmospherePlugin;
pub use effects::{CesiumEffectsPlugin, PostProcessConfig, CesiumParticlePlugin};
// pub use effects::oit::{OITPlugin, OitConfig, SplitConfig}; // FIX-P0-OIT: isolated.
pub use voxel::{CesiumVoxelPlugin, VoxelConfig, VoxelPrimitiveComponent, VoxelPrimitiveType};
pub use vector::{CesiumVectorTilePlugin, CesiumWktPlugin, VectorTileConfig, WktLoadQueue};
pub use shadow::{CesiumShadowPlugin, ShadowConfig, ShadowState, ShadowCaster};
pub use widgets::CesiumWidgetPlugin;
pub use pipeline::CesiumPipelinePlugin;

use bevy::pbr::DirectionalLightShadowMap;

use bevy::prelude::*;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::geometry::{self, GeometryData, PrimitiveType, VertexFormat};
use cesium_imagery::blending::PixelColor;
use cesium_terrain::terrain_mesh::TerrainMesh;

/// Convert f64 GeometryData to Bevy Mesh (f32 precision boundary)
///
/// # RTC (Relative-To-Center) precision bridge
/// `rtc_center`, when `Some(c)`, shifts every vertex position by `-c` **in f64**
/// before truncating to f32, producing a tile-local mesh whose coordinates stay
/// small enough to retain f32 precision even for Earth-scale ECEF inputs. This
/// is the standard CesiumJS/3D Tiles "RTC center" trick, applied at the single
/// f64→f32 precision boundary of this adapter.
///
/// When `None`, behaviour is byte-for-byte identical to the previous signature:
/// positions are cast straight from f64 to f32 with no offset applied.
///
/// DEVIATION: 公共签名新增 rtc_center: Option<DVec3>（非 additive）；
/// see docs/deviations.md#dev-013
pub fn geometry_to_mesh(geometry: &GeometryData, rtc_center: Option<glam::DVec3>) -> Mesh {
    // Convert positions: f64 → f32, optionally recentered in f64 first (see doc above)
    let positions: Vec<[f32; 3]> = geometry
        .positions
        .iter()
        .map(|p| match rtc_center {
            Some(c) => [(p[0] - c.x) as f32, (p[1] - c.y) as f32, (p[2] - c.z) as f32],
            None => [p[0] as f32, p[1] as f32, p[2] as f32],
        })
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
    // Full-ellipsoid mesh is already centered at the origin; no RTC offset needed.
    geometry_to_mesh(&geometry, None)
}

/// Convert a domain TerrainMesh (f64) to a Bevy Mesh (f32)
///
/// # RTC (Relative-To-Center) precision bridge
/// `rtc_center`, when `Some(c)`, shifts every vertex position by `-c` **in f64**
/// before truncating to f32, producing a tile-local mesh (same rationale and
/// semantics as [`geometry_to_mesh`]). When `None`, behaviour is byte-for-byte
/// identical to the previous signature.
///
/// DEVIATION: 公共签名新增 rtc_center: Option<DVec3>（非 additive）；
/// see docs/deviations.md#dev-013
pub fn terrain_mesh_to_bevy(terrain: &TerrainMesh, rtc_center: Option<glam::DVec3>) -> Mesh {
    let positions: Vec<[f32; 3]> = terrain
        .positions
        .iter()
        .map(|p| match rtc_center {
            Some(c) => [(p[0] - c.x) as f32, (p[1] - c.y) as f32, (p[2] - c.z) as f32],
            None => [p[0] as f32, p[1] as f32, p[2] as f32],
        })
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

/// Lighting mode selector — determines which lighting rig `setup_lighting`
/// spawns. Inserted as a Bevy `Resource` by the application layer (main.rs)
/// before `CesiumCorePlugin` builds, so the Startup system can read it.
///
/// * `FullAmbient` — uniform ambient only (CesiumJS `enableLighting=false`
///   look). This is the **default** and produces pixel-identical output to
///   the pre-M4.1 baseline (PSNR=∞).
/// * `DayNight` — ambient + directional sun light with shadow maps,
///   driven by `celestial_system` (compute_sun_direction_eci).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LightingMode {
    /// Uniform ambient illumination only (v0 baseline, zero-diff).
    #[default]
    FullAmbient,
    /// Directional sun + ambient; activates shadow + atmosphere plugins.
    DayNight,
}

/// Plugin that initializes CesiumRust core Bevy resources (GlobeConfig,
/// RenderScale, TileLoadStats, AnimationClock) and sets up scene lighting.
pub struct CesiumCorePlugin;

impl Plugin for CesiumCorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GlobeConfig>()
            .init_resource::<RenderScale>()
            .init_resource::<TileLoadStats>()
            .init_resource::<LightingMode>()
            // M4.1: promote AnimationClock to core so celestial_system can
            // read it without requiring CesiumEntityPlugin.
            .init_resource::<AnimationClock>()
            .add_systems(Startup, setup_lighting);
    }
}

/// System that spawns scene lighting, branching on [`LightingMode`].
///
/// ## FullAmbient (default — v0 zero-diff)
///
/// Uniform ambient illumination only, no directional sun: CesiumJS's default
/// globe runs with `enableLighting = false`, i.e. the whole planet renders as
/// full daylight with no day/night terminator, and we match that look here.
///
/// Brightness calibration: the PBR ambient pipeline applies an empirical
/// ~3.18e-4 physical-units scale to `brightness` (measured: 10 000 blew out
/// to a ~3x overexposed wash, 2.2 rendered black), so `1 / 3.18e-4 ~= 3300`
/// reproduces the imagery albedo ~1:1 — CesiumJS displays tiles as-is.
///
/// Color temperature: the reference CesiumJS whole-globe look is cool-toned
/// (sage land, medium steel-blue ocean) while raw Bing albedo renders warm
/// and near-black in the ocean; a cool tint (red cut, blue boost) plus a
/// slight brightness lift shifts the white balance toward the reference.
///
/// ## DayNight (M4.1)
///
/// Ambient (reduced) + a `DirectionalLight` with illuminance 10 000 lx and
/// Bevy's built-in `DirectionalLightShadowMap` (4 cascades, 2048 px).
/// The light direction is driven per-frame by `celestial_system`
/// (compute_sun_direction_eci → look_to). Shadow cascade computation lives
/// in `shadow_update_system` which queries the DirectionalLight transform.
///
/// Near/far in render units: 0.01 / 100.0 (× METERS_PER_RENDER_UNIT =
/// 63 781 m … 637 813 700 m) — covers LEO to cislunar without z-fighting
/// at the globe surface.
fn setup_lighting(mut commands: Commands, mode: Res<LightingMode>) {
    match *mode {
        LightingMode::FullAmbient => {
            // v0 baseline: ambient only, pixel-identical to pre-M4.1.
            commands.insert_resource(AmbientLight {
                color: Color::srgb(0.79, 0.94, 1.17),
                brightness: 3800.0,
            });
        }
        LightingMode::DayNight => {
            // Reduced ambient so the directional sun dominates day-side.
            commands.insert_resource(AmbientLight {
                color: Color::srgb(0.79, 0.94, 1.17),
                brightness: 1200.0,
            });

            // Directional sun light with shadow maps.
            commands.spawn((
                DirectionalLight {
                    illuminance: 10_000.0,
                    shadows_enabled: true,
                    ..default()
                },
                // Default transform; celestial_system will orient it per-frame.
                Transform::IDENTITY,
            ));

            // Bevy built-in shadow map config (4 cascades, 2048 resolution).
            commands.insert_resource(DirectionalLightShadowMap {
                size: 2048,
            });
        }
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
        let mesh = geometry_to_mesh(&geometry, None);

        // Verify mesh has position attribute
        assert!(mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
        assert!(mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some());
        assert!(mesh.attribute(Mesh::ATTRIBUTE_UV_0).is_some());
    }

    #[test]
    fn test_geometry_to_mesh_rtc_center_none_matches_legacy() {
        let geometry = GeometryData {
            positions: vec![[1.0, 2.0, 3.0], [-4.0, 5.5, -6.25]],
            normals: None,
            tex_coords: None,
            tangents: None,
            bitangents: None,
            indices: vec![0, 1, 0],
            primitive_type: PrimitiveType::Triangles,
            bounding_sphere: BoundingSphere::new(glam::DVec3::ZERO, 10.0),
        };

        let mesh = geometry_to_mesh(&geometry, None);
        let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        if let bevy::render::mesh::VertexAttributeValues::Float32x3(pos) = positions {
            assert_eq!(pos[0], [1.0_f32, 2.0_f32, 3.0_f32]);
            assert_eq!(pos[1], [-4.0_f32, 5.5_f32, -6.25_f32]);
        } else {
            panic!("Expected Float32x3 positions");
        }
    }

    #[test]
    fn test_geometry_to_mesh_rtc_center_some_recenters_in_f64() {
        let geometry = GeometryData {
            positions: vec![[1.0, 2.0, 3.0], [-4.0, 5.5, -6.25]],
            normals: None,
            tex_coords: None,
            tangents: None,
            bitangents: None,
            indices: vec![0, 1, 0],
            primitive_type: PrimitiveType::Triangles,
            bounding_sphere: BoundingSphere::new(glam::DVec3::ZERO, 10.0),
        };

        let center = glam::DVec3::new(1.0, 2.0, 3.0);
        let mesh = geometry_to_mesh(&geometry, Some(center));
        let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        if let bevy::render::mesh::VertexAttributeValues::Float32x3(pos) = positions {
            assert_eq!(pos[0], [0.0_f32, 0.0_f32, 0.0_f32]);
            assert_eq!(pos[1], [-5.0_f32, 3.5_f32, -9.25_f32]);
        } else {
            panic!("Expected Float32x3 positions");
        }
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

        let mesh = terrain_mesh_to_bevy(&terrain, None);
        assert!(mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
        assert!(mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some());
        assert!(mesh.attribute(Mesh::ATTRIBUTE_UV_0).is_some());
    }

    #[test]
    fn test_terrain_mesh_to_bevy_rtc_center_some_recenters_in_f64() {
        let terrain = TerrainMesh {
            positions: vec![[10.0, 20.0, 30.0], [11.0, 20.0, 30.0], [10.0, 21.0, 30.0]],
            normals: None,
            tex_coords: None,
            indices: vec![0, 1, 2],
            minimum_height: 0.0,
            maximum_height: 0.0,
            bounding_sphere: BoundingSphere::new(glam::DVec3::ZERO, 1.0),
        };

        let center = glam::DVec3::new(10.0, 20.0, 30.0);
        let mesh = terrain_mesh_to_bevy(&terrain, Some(center));
        let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        if let bevy::render::mesh::VertexAttributeValues::Float32x3(pos) = positions {
            assert_eq!(pos[0], [0.0_f32, 0.0_f32, 0.0_f32]);
            assert_eq!(pos[1], [1.0_f32, 0.0_f32, 0.0_f32]);
            assert_eq!(pos[2], [0.0_f32, 1.0_f32, 0.0_f32]);
        } else {
            panic!("Expected Float32x3 positions");
        }
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
