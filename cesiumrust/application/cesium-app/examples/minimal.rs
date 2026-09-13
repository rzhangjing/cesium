//! Minimal CesiumRust example: globe + camera + test entities.
//!
//! This demonstrates the simplest possible CesiumRust app using
//! the hexagonal-architecture plugin stack.

use bevy::prelude::*;
use cesium_bevy_render::{CesiumCorePlugin, CesiumEntityPlugin, CesiumImageryPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "CesiumRust — Minimal Example".into(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(CesiumCorePlugin)
        .add_plugins(CesiumEntityPlugin)
        .add_plugins(CesiumImageryPlugin)
        .add_systems(Startup, spawn_scene)
        .run();
}

fn spawn_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut imagery_mgr: ResMut<cesium_bevy_render::imagery::ImageryLayerManager>,
    mut globe_config: ResMut<cesium_bevy_render::GlobeConfig>,
) {
    use cesium_bevy_render::{
        create_ellipsoid_mesh, CesiumGlobe, CesiumTerrainTile, METERS_PER_RENDER_UNIT,
    };
    use cesium_camera::Camera as DomainCamera;
    use cesium_geospatial::cartographic::Cartographic;
    use cesium_geospatial::ellipsoid::Ellipsoid;
    use cesium_scene_mode::SceneMode;
    use glam::{DVec3, Vec3};

    let scale = (1.0 / METERS_PER_RENDER_UNIT) as f32;

    // ── Globe ──────────────────────────────────────────────────
    globe_config.ellipsoid = Ellipsoid::WGS84;

    let globe_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.08, 0.18, 0.35),
        perceptual_roughness: 0.85,
        ..default()
    });
    commands.spawn((
        CesiumGlobe,
        Mesh3d(meshes.add(create_ellipsoid_mesh(32, 64))),
        MeshMaterial3d(globe_material),
        Transform::from_scale(Vec3::splat(scale)),
    ));
    commands.spawn(CesiumTerrainTile { x: 0, y: 0, level: 0 });

    // ── Camera ─────────────────────────────────────────────────
    let cam_pos = {
        let c = Cartographic::from_degrees(-95.0, 40.0, 20_000_000.0);
        Ellipsoid::WGS84.cartographic_to_cartesian(&c)
    };
    let target = {
        let c = Cartographic::from_degrees(-95.0, 40.0, 0.0);
        Ellipsoid::WGS84.cartographic_to_cartesian(&c)
    };
    let dir = (target - cam_pos).normalize();
    let up = cam_pos.normalize();
    let camera = DomainCamera::new(cam_pos, dir, up);

    commands.spawn((
        cesium_bevy_render::CesiumCamera {
            camera,
            scene_mode: SceneMode::Scene3D,
            enable_collision_detection: true,
            minimum_zoom_distance: 100.0,
            maximum_zoom_distance: 20_000_000.0,
        },
        Camera3d::default(),
        Transform::from_translation(cam_pos.as_vec3() * scale)
            .looking_at(target.as_vec3() * scale, up.as_vec3()),
    ));

    // ── Imagery ────────────────────────────────────────────────
    imagery_mgr.add_layer(
        "https://tile.openstreetmap.org/{z}/{x}/{y}.png",
        1.0,
        0,
        18,
    );

    // ── Test entities ──────────────────────────────────────────
    let ny = ellipsoid_point(-74.006, 40.7128, 1000.0);
    let point_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.0, 0.0),
        emissive: LinearRgba::rgb(50.0, 0.0, 0.0),
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.0015))),
        MeshMaterial3d(point_mat),
        Transform {
            translation: ny * scale,
            scale: Vec3::splat(3.0),
            ..default()
        },
    ));

    println!("[Minimal] Globe + camera + OSM layer + NY point spawned");
}

fn ellipsoid_point(lon_deg: f64, lat_deg: f64, height: f64) -> Vec3 {
    use cesium_geospatial::cartographic::Cartographic;
    use cesium_geospatial::ellipsoid::Ellipsoid;
    let carto = Cartographic::from_degrees(lon_deg, lat_deg, height);
    let ecef = Ellipsoid::WGS84.cartographic_to_cartesian(&carto);
    Vec3::new(ecef.x as f32, ecef.y as f32, ecef.z as f32)
}
