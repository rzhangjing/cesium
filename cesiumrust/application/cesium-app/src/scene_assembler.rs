//! Scene assembler: bootstraps a minimal but complete Cesium scene at startup.
//!
//! Creates the globe entity, camera, test imagery layer, and test entities
//! (points, polylines, polygons, billboards) to verify the rendering pipeline.
//!
//! Keyboard controls:
//!   R — reset camera to default view
//!   T — toggle terrain wireframe
//!   L — cycle through imagery layers
//!   F — fly to Grand Canyon preset
//!   H — print scene statistics to console

use bevy::prelude::*;
use cesium_bevy_render::{
    create_ellipsoid_mesh, CesiumCamera, CesiumGlobe, CesiumTerrainTile,
    FlyToRequest, GlobeConfig, TileLoadStats, METERS_PER_RENDER_UNIT,
};
use cesium_bevy_render::imagery::ImageryLayerManager;
use cesium_camera::Camera as DomainCamera;
use cesium_geospatial::cartographic::Cartographic;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_scene_mode::SceneMode;

use crate::orbit_camera::OrbitState;

// ── Resources ────────────────────────────────────────────────────────

#[derive(Resource, Default)]
struct WireframeMode(bool);

#[derive(Resource)]
struct ImageryCycleIndex(usize);

impl Default for ImageryCycleIndex {
    fn default() -> Self {
        Self(0)
    }
}

#[derive(Resource)]
struct SceneStatsTimer(Timer);

impl Default for SceneStatsTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(5.0, TimerMode::Repeating))
    }
}

// ── Plugin ────────────────────────────────────────────────────────────

pub struct SceneAssemblerPlugin;

impl Plugin for SceneAssemblerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WireframeMode>()
            .init_resource::<ImageryCycleIndex>()
            .init_resource::<SceneStatsTimer>()
            .add_systems(Startup, (setup_scene, setup_entities).chain())
            .add_systems(Startup, print_scene_health_check)
            .add_systems(
                Update,
                (
                    keyboard_controls,
                    print_scene_stats,
                ),
            );
    }
}

// ── Globe spawn ──────────────────────────────────────────────────────

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut globe_config: ResMut<GlobeConfig>,
    mut imagery_mgr: ResMut<ImageryLayerManager>,
) {
    let scale = (1.0 / METERS_PER_RENDER_UNIT) as f32;

    // ── Globe entity ──────────────────────────────────────────────
    let globe_mesh = create_ellipsoid_mesh(64, 128);
    let globe_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.08, 0.18, 0.35),
        perceptual_roughness: 0.85,
        ..default()
    });

    let globe_id = commands
        .spawn((
            CesiumGlobe,
            Mesh3d(meshes.add(globe_mesh)),
            MeshMaterial3d(globe_material),
            Transform::from_scale(Vec3::splat(scale)),
        ))
        .with_children(|parent| {
            parent.spawn((
                CesiumTerrainTile {
                    x: 0,
                    y: 0,
                    level: 0,
                },
            ));
        })
        .id();

    println!(
        "[SceneAssembler] Spawned globe entity {:?} with WGS84 ellipsoid mesh (64×128)",
        globe_id
    );

    // ── Configure globe ───────────────────────────────────────────
    globe_config.ellipsoid = Ellipsoid::WGS84;
    globe_config.imagery_providers.push(
        "https://tile.openstreetmap.org/{z}/{x}/{y}.png".into(),
    );

    // ── Camera ────────────────────────────────────────────────────
    // Position: looking at North America from space (~3x Earth radius away)
    let camera_position = ellipsoid_position(-95.0, 40.0, 20_000_000.0);
    let look_target = ellipsoid_position(-95.0, 40.0, 0.0);
    let direction = (look_target - camera_position).normalize();
    let up = camera_position.normalize();

    let domain_camera = DomainCamera::new(
        camera_position,
        direction,
        up,
    );

    commands.spawn((
        CesiumCamera {
            camera: domain_camera,
            scene_mode: SceneMode::Scene3D,
            enable_collision_detection: true,
            minimum_zoom_distance: 100.0,
            maximum_zoom_distance: 20_000_000.0,
        },
        Transform::from_translation(camera_position.as_vec3())
            .looking_at(look_target.as_vec3(), up.as_vec3()),
    ));

    println!(
        "[SceneAssembler] Spawned CesiumCamera at ({:.1}, {:.1}) altitude={:.0}m",
        -95.0, 40.0, 20_000_000.0
    );

    // ── Imagery layers ────────────────────────────────────────────
    imagery_mgr.add_layer(
        "https://tile.openstreetmap.org/{z}/{x}/{y}.png",
        1.0,
        0,
        18,
    );
    imagery_mgr.add_layer(
        "https://services.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}",
        1.0,
        0,
        18,
    );
    imagery_mgr.add_layer(
        "https://tiles.stadiamaps.com/tiles/stamen_toner/{z}/{x}/{y}.png",
        0.8,
        0,
        18,
    );

    println!(
        "[SceneAssembler] Configured {} imagery layers",
        imagery_mgr.layer_count()
    );

    println!("[SceneAssembler] Scene setup complete");
}

// ── Test entities ────────────────────────────────────────────────────

fn setup_entities(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let scale = (1.0 / METERS_PER_RENDER_UNIT) as f32;

    // Point at New York
    let ny_pos = ellipsoid_position(-74.006, 40.7128, 1000.0);
    let point_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.0, 0.0),
        emissive: LinearRgba::rgb(50.0, 0.0, 0.0),
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.002))),
        MeshMaterial3d(point_material),
        Transform {
            translation: ny_pos.as_vec3() * scale,
            scale: Vec3::splat(5.0),
            ..default()
        },
    ));
    println!(
        "[SceneAssembler] Spawned point at New York ({:.4}, {:.4})",
        -74.006, 40.7128
    );

    // Polyline from San Francisco to New York
    let sf_pos = ellipsoid_position(-122.4194, 37.7749, 1000.0);
    let line_mesh = create_line_mesh(&[sf_pos, ny_pos], scale);
    let line_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 0.3, 1.0),
        emissive: LinearRgba::rgb(5.0, 0.0, 15.0),
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(line_mesh)),
        MeshMaterial3d(line_material),
        Transform::default(),
    ));
    println!(
        "[SceneAssembler] Spawned polyline SF→NY ({}→{})",
        "37.8N 122.4W", "40.7N 74.0W"
    );

    // Polygon over Texas (approximate bounding rectangle)
    let texas_points = [
        (-106.5, 36.5),
        (-106.5, 31.5),
        (-95.5, 31.5),
        (-95.5, 36.5),
    ];
    let tx_surf: Vec<glam::DVec3> = texas_points
        .iter()
        .map(|&(lon, lat)| ellipsoid_position(lon, lat, 5000.0))
        .collect();
    let tx_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.0, 0.8, 0.2, 0.3),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(create_polygon_mesh(&tx_surf, scale))),
        MeshMaterial3d(tx_material),
        Transform::default(),
    ));
    println!("[SceneAssembler] Spawned polygon over Texas (4 vertices)");

    // Billboard at London
    let london_pos = ellipsoid_position(-0.1276, 51.5074, 50000.0);
    let billboard_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.84, 0.0),
        emissive: LinearRgba::rgb(30.0, 20.0, 0.0),
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.003))),
        MeshMaterial3d(billboard_material),
        Transform {
            translation: london_pos.as_vec3() * scale,
            scale: Vec3::splat(5.0),
            ..default()
        },
    ));
    println!("[SceneAssembler] Spawned billboard at London ({:.4}, {:.4})", -0.1276, 51.5074);
}

// ── Keyboard controls ────────────────────────────────────────────────

fn keyboard_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<OrbitState>,
    mut wireframe: ResMut<WireframeMode>,
    mut cycle_idx: ResMut<ImageryCycleIndex>,
    imagery_mgr: Res<ImageryLayerManager>,
    mut ev_fly: EventWriter<FlyToRequest>,
    globe_config: Res<GlobeConfig>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        *state = OrbitState::default();
        println!("[Ctrl] Camera reset to default view");
    }

    if keys.just_pressed(KeyCode::KeyT) {
        wireframe.0 = !wireframe.0;
        println!("[Ctrl] Terrain wireframe: {}", if wireframe.0 { "ON" } else { "OFF" });
    }

    if keys.just_pressed(KeyCode::KeyL) {
        let count = imagery_mgr.layer_count();
        if count > 0 {
            cycle_idx.0 = (cycle_idx.0 + 1) % count;
            println!(
                "[Ctrl] Active imagery layer: {} / {}",
                cycle_idx.0 + 1,
                count
            );
        }
    }

    if keys.just_pressed(KeyCode::KeyF) {
        let target = Cartographic::from_degrees(-112.1, 36.1, 5000.0);
        ev_fly.send(FlyToRequest {
            destination: target,
            duration_secs: 1.5,
        });
        println!("[Ctrl] Flying to Grand Canyon (36.1N 112.1W)");
    }

    if keys.just_pressed(KeyCode::KeyH) {
        print_concurrent_stats(&globe_config, &imagery_mgr, &state);
    }
}

// ── Scene stats ──────────────────────────────────────────────────────

fn print_scene_stats(
    time: Res<Time>,
    mut timer: ResMut<SceneStatsTimer>,
    state: Res<OrbitState>,
    globe_config: Res<GlobeConfig>,
    imagery_mgr: Res<ImageryLayerManager>,
    stats: Res<TileLoadStats>,
    globe_query: Query<(), With<CesiumGlobe>>,
    tile_query: Query<&CesiumTerrainTile>,
) {
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }

    let globe_count = globe_query.iter().count();
    let tile_count = tile_query.iter().count();
    let imagery_count = imagery_mgr.layer_count();
    let fps = 1.0 / time.delta_secs();

    // Compute approximate camera lat/lon
    let cam_pos = compute_cam_position(&state);
    let ellipsoid = &globe_config.ellipsoid;
    let carto = ellipsoid.cartesian_to_cartographic(cam_pos);

    println!(
        "[Stats] Globes={} Tiles={} ImageryLayers={} FPS={:.1} | Cam={}",
        globe_count,
        tile_count,
        imagery_count,
        fps,
        carto
            .map(|c| format!("{:.2}°N {:.2}°W H={:.0}m", c.latitude.to_degrees(), -c.longitude.to_degrees(), c.height))
            .unwrap_or_else(|| "unknown".into())
    );

    if stats.tiles_loaded > 0 || stats.tiles_failed > 0 {
        println!(
            "[Stats] Downloads: {} loaded, {} failed ({} MB)",
            stats.tiles_loaded,
            stats.tiles_failed,
            stats.bytes_downloaded / 1_000_000
        );
    }
}

fn print_concurrent_stats(
    globe_config: &GlobeConfig,
    imagery_mgr: &ImageryLayerManager,
    state: &OrbitState,
) {
    let ellipsoid = &globe_config.ellipsoid;
    let cam_pos = compute_cam_position(state);
    let carto = ellipsoid.cartesian_to_cartographic(cam_pos);

    println!("═══ CesiumRust Scene Statistics ═══");
    println!("  Ellipsoid: {:?}", ellipsoid);
    println!("  Imagery layers: {}", imagery_mgr.layer_count());
    for (i, layer) in imagery_mgr.layers.iter().enumerate() {
        println!(
            "    [{}] {} (levels {}-{}, opacity={:.2})",
            i + 1,
            &layer.url_template[..layer.url_template.len().min(60)],
            layer.min_level,
            layer.max_level,
            layer.opacity
        );
    }
    println!("  Imagery enabled: {}", imagery_mgr.enabled);
    println!(
        "  Camera: {:.4}°N {:.4}°W altitude={:.0}m dist={:.2}RU",
        carto.map(|c| c.latitude.to_degrees()).unwrap_or(0.0),
        carto.map(|c| -c.longitude.to_degrees()).unwrap_or(0.0),
        carto.map(|c| c.height).unwrap_or(0.0),
        state.distance
    );
    println!("══════════════════════════════════════");
}

// ── Health check ─────────────────────────────────────────────────────

fn print_scene_health_check(
    globe_query: Query<(), With<CesiumGlobe>>,
    camera_query: Query<(), With<CesiumCamera>>,
    imagery_mgr: Res<ImageryLayerManager>,
) {
    println!("═══ CesiumRust Health Check ═══");

    let globe_ok = !globe_query.is_empty();
    println!(
        "  Globe mesh:  {}",
        if globe_ok { "✓ SPAWNED" } else { "✗ MISSING" }
    );

    let camera_ok = !camera_query.is_empty();
    println!(
        "  Camera:      {}",
        if camera_ok {
            "✓ ACTIVE"
        } else {
            "✗ MISSING"
        }
    );

    let imagery_ok = imagery_mgr.layer_count() > 0;
    println!(
        "  Imagery:     {} ({} layers)",
        if imagery_ok { "✓ CONFIGURED" } else { "⚠  NONE" },
        imagery_mgr.layer_count()
    );

    let all_ok = globe_ok && camera_ok && imagery_ok;
    println!(
        "  Overall:     {}",
        if all_ok { "✓ ALL CHECKS PASSED" } else { "⚠  SOME CHECKS FAILED" }
    );
    println!("═════════════════════════════");

    if !all_ok {
        eprintln!("[HealthCheck] ⚠  WARNING: {}",
            vec![
                if !globe_ok { Some("globe mesh not spawned") } else { None },
                if !camera_ok { Some("camera not spawned") } else { None },
                if !imagery_ok { Some("no imagery layers configured") } else { None },
            ].into_iter().flatten().collect::<Vec<_>>().join(", ")
        );
    } else {
        println!("[HealthCheck] ✓ All systems nominal");
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

fn ellipsoid_position(lon_deg: f64, lat_deg: f64, height: f64) -> glam::DVec3 {
    let carto = Cartographic::from_degrees(lon_deg, lat_deg, height);
    Ellipsoid::WGS84.cartographic_to_cartesian(&carto)
}

fn compute_cam_position(state: &OrbitState) -> glam::DVec3 {
    let cos_pitch = state.pitch.cos() as f64;
    let sin_pitch = state.pitch.sin() as f64;
    let distance = state.distance as f64;
    let heading = state.heading as f64;
    glam::DVec3::new(
        distance * cos_pitch * heading.cos(),
        distance * cos_pitch * heading.sin(),
        distance * sin_pitch,
    )
}

fn create_line_mesh(points: &[glam::DVec3], scale: f32) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    for point in points {
        indices.push(positions.len() as u32);
        let p = point.as_vec3() * scale;
        positions.push(p.into());
    }

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::LineStrip,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
    mesh
}

fn create_polygon_mesh(points: &[glam::DVec3], scale: f32) -> Mesh {
    // Simple triangle fan from first vertex
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let center = points.iter().fold(glam::DVec3::ZERO, |a, b| a + *b) / points.len() as f64;

    for point in points {
        let p = point.as_vec3() * scale;
        positions.push(p.into());
    }
    let c = center.as_vec3() * scale;
    positions.push(c.into());

    let n = points.len() as u32;
    let center_idx = n;
    let mut indices: Vec<u32> = Vec::new();

    for i in 0..n {
        let next = (i + 1) % n;
        indices.push(center_idx);
        indices.push(i);
        indices.push(next);
    }

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
    mesh.compute_normals();
    mesh
}

// ── Integration tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_bevy_render::{
        components::{CesiumGlobe, CesiumTerrainTile},
        camera::components::CesiumCamera,
    };

    #[test]
    fn test_ellipsoid_position_origin() {
        let pos = ellipsoid_position(0.0, 0.0, 0.0);
        assert!((pos.x - METERS_PER_RENDER_UNIT).abs() < 10.0, "x={}", pos.x);
        assert!(pos.y.abs() < 1.0, "y={}", pos.y);
        assert!(pos.z.abs() < 1.0, "z={}", pos.z);
    }

    #[test]
    fn test_ellipsoid_position_north_pole() {
        let pos = ellipsoid_position(0.0, 90.0, 0.0);
        assert!(pos.x.abs() < 1.0, "x={}", pos.x);
        assert!(pos.y.abs() < 1.0, "y={}", pos.y);
        assert!(pos.z > 6_350_000.0, "z={}", pos.z);
    }

    #[test]
    fn test_ellipsoid_position_new_york() {
        let pos = ellipsoid_position(-74.006, 40.7128, 0.0);
        let dist = (pos.x * pos.x + pos.y * pos.y + pos.z * pos.z).sqrt();
        assert!(dist > 6_000_000.0 && dist < 6_500_000.0, "distance from center: {}", dist);
    }

    #[test]
    fn test_create_line_mesh_has_correct_topology() {
        let sf = ellipsoid_position(-122.4194, 37.7749, 1000.0);
        let ny = ellipsoid_position(-74.006, 40.7128, 1000.0);
        let scale = (1.0 / METERS_PER_RENDER_UNIT) as f32;
        let mesh = create_line_mesh(&[sf, ny], scale);

        let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        if let bevy::render::mesh::VertexAttributeValues::Float32x3(pos) = positions {
            assert_eq!(pos.len(), 2);
        } else {
            panic!("Expected Float32x3 positions");
        }
    }

    #[test]
    fn test_create_polygon_mesh_has_correct_vertices() {
        let points: Vec<glam::DVec3> = [
            (-106.5, 36.5),
            (-106.5, 31.5),
            (-95.5, 31.5),
            (-95.5, 36.5),
        ]
        .iter()
        .map(|&(lon, lat)| ellipsoid_position(lon, lat, 0.0))
        .collect();
        let scale = (1.0 / METERS_PER_RENDER_UNIT) as f32;
        let mesh = create_polygon_mesh(&points, scale);

        let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        if let bevy::render::mesh::VertexAttributeValues::Float32x3(pos) = positions {
            // 4 perimeter vertices + 1 center
            assert_eq!(pos.len(), 5);
        } else {
            panic!("Expected Float32x3 positions");
        }

        assert!(mesh.indices().is_some(), "polygon mesh should have indices");
    }

    #[test]
    fn test_compute_cam_position_default() {
        let state = OrbitState::default();
        let pos = compute_cam_position(&state);
        let expected_dir = glam::DVec3::new(
            3.0 * (0.4_f64).cos() * (0.0_f64).cos(),
            3.0 * (0.4_f64).cos() * (0.0_f64).sin(),
            3.0 * (0.4_f64).sin(),
        );
        let diff = (pos - expected_dir).length();
        assert!(diff < 0.01, "diff={}", diff);
    }

    #[test]
    fn test_imagery_cycle_has_expected_count() {
        let mut mgr = ImageryLayerManager::default();
        mgr.add_layer("https://a.tiles.example.com/{z}/{x}/{y}.png", 1.0, 0, 18);
        mgr.add_layer("https://b.tiles.example.com/{z}/{x}/{y}.png", 0.5, 0, 12);

        assert_eq!(mgr.layer_count(), 2);
        assert_eq!(mgr.visible_layers().count(), 2);
    }

    #[test]
    fn test_imagery_layer_cycle() {
        let mut mgr = ImageryLayerManager::default();
        mgr.add_layer("https://a.tiles.example.com/{z}/{x}/{y}.png", 1.0, 0, 18);

        let mut idx = ImageryCycleIndex::default();
        assert_eq!(idx.0, 0);

        let count = mgr.layer_count();
        idx.0 = (idx.0 + 1) % if count > 0 { count } else { 1 };
        assert_eq!(idx.0, 0, "should wrap around to 0 when only 1 layer");
    }

    #[test]
    fn test_scene_assembler_resources_are_send() {
        fn assert_send<T: Send>() {}
        assert_send::<WireframeMode>();
        assert_send::<ImageryCycleIndex>();
        assert_send::<SceneStatsTimer>();
    }
}
