//! Geometry showcase - displays all geometry types in a grid.

use bevy::prelude::*;
use cesium_bevy_render::geometry_to_mesh;
use cesium_geospatial::cartographic::Cartographic;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::frustum::PerspectiveFrustum;
use cesium_geospatial::geometry::{
    self, box_geometry, box_outline_geometry, circle_geometry,
    coplanar_polygon_geometry, corridor_geometry, cylinder_geometry, cylinder_outline_geometry,
    ellipse_geometry, ellipse_outline_geometry, ellipsoid_geometry, ellipsoid_outline_geometry,
    frustum_geometry, ground_polyline_geometry, plane_geometry, plane_outline_geometry,
    polyline_geometry, polyline_volume_geometry, rectangle_geometry,
    sphere_geometry, wall_geometry, CornerType, CorridorOptions, CoplanarPolygonOptions,
    EllipseOptions, FrustumDef, GroundPolylineOptions, PolylineOptions, PolylineVolumeOptions,
    VertexFormat, WallOptions,
};
use cesium_geospatial::rectangle::Rectangle;
use glam::{DQuat, DVec3};

/// Plugin that spawns a geometry showcase scene.
pub struct GeometryShowcasePlugin;

impl Plugin for GeometryShowcasePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_geometry_showcase);
    }
}

fn setup_geometry_showcase(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // NOTE: Camera and Light are provided by CesiumRenderPlugin;
    // do NOT spawn duplicates here to avoid order-ambiguity warnings.

    let ell = Ellipsoid::WGS84;
    let vf = VertexFormat::ALL;

    // Grid layout: 5 columns x 4 rows, spacing 3 units.
    let spacing = 3.0;
    let cols = 5;
    let start_x = -(cols as f32 - 1.0) * spacing / 2.0;
    let start_z = -3.0 * spacing / 2.0;

    let mut idx = 0;
    let spawn_geo = |commands: &mut Commands,
                         meshes: &mut Assets<Mesh>,
                         materials: &mut Assets<StandardMaterial>,
                         geo: geometry::GeometryData,
                         color: Color,
                         idx: &mut usize| {
        let mesh = geometry_to_mesh(&geo);
        let row = *idx / cols;
        let col = *idx % cols;
        let x = start_x + col as f32 * spacing;
        let z = start_z + row as f32 * spacing;
        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                ..default()
            })),
            Transform::from_xyz(x, 0.0, z).with_scale(Vec3::splat(0.8)),
        ));
        *idx += 1;
    };

    // Row 1: Basic primitives.
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        box_geometry(DVec3::new(-0.5, -0.5, -0.5), DVec3::new(0.5, 0.5, 0.5), vf),
        Color::srgb(0.8, 0.2, 0.2),
        &mut idx,
    );
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        sphere_geometry(0.5, 16, 32, vf),
        Color::srgb(0.2, 0.8, 0.2),
        &mut idx,
    );
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        cylinder_geometry(1.0, 0.5, 0.5, 32, vf),
        Color::srgb(0.2, 0.2, 0.8),
        &mut idx,
    );
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        plane_geometry(vf),
        Color::srgb(0.8, 0.8, 0.2),
        &mut idx,
    );
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        ellipsoid_geometry(DVec3::new(0.5, 0.3, 0.4), 16, 32, vf),
        Color::srgb(0.8, 0.2, 0.8),
        &mut idx,
    );

    // Row 2: Geodetic geometries (scaled down).
    let center = ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0));
    let scale = 1e-6; // Scale down from meters to scene units.

    let ellipse_opts = EllipseOptions {
        center,
        semi_major_axis: 100_000.0,
        semi_minor_axis: 50_000.0,
        height: 0.0,
        rotation: 0.0,
        st_rotation: 0.0,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: ell,
    };
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        scale_geometry(&ellipse_geometry(&ellipse_opts, vf), scale),
        Color::srgb(0.2, 0.8, 0.8),
        &mut idx,
    );

    let corridor_opts = CorridorOptions {
        positions: vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(-2.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 2.0, 0.0)),
        ],
        width: 50_000.0,
        height: 0.0,
        granularity: std::f64::consts::PI / 180.0,
        corner_type: CornerType::Rounded,
        ellipsoid: ell,
    };
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        scale_geometry(&corridor_geometry(&corridor_opts, vf), scale),
        Color::srgb(0.8, 0.5, 0.2),
        &mut idx,
    );

    let wall_opts = WallOptions::from_constant_heights(
        vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(-1.0, -1.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, -1.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 1.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(-1.0, 1.0, 0.0)),
        ],
        Some(0.0),
        Some(100_000.0),
        ell,
    );
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        scale_geometry(&wall_geometry(&wall_opts, vf), scale),
        Color::srgb(0.5, 0.2, 0.8),
        &mut idx,
    );

    let polyline_opts = PolylineOptions {
        positions: vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(-2.0, -1.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(2.0, -1.0, 0.0)),
        ],
        width: 20_000.0,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: ell,
    };
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        scale_geometry(&polyline_geometry(&polyline_opts, vf), scale),
        Color::srgb(0.2, 0.5, 0.8),
        &mut idx,
    );

    let coplanar_opts = CoplanarPolygonOptions {
        positions: vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(-1.0, -1.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, -1.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 1.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(-1.0, 1.0, 0.0)),
        ],
        st_rotation: 0.0,
        ellipsoid: ell,
    };
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        scale_geometry(&coplanar_polygon_geometry(&coplanar_opts, vf), scale),
        Color::srgb(0.8, 0.8, 0.5),
        &mut idx,
    );

    // Row 3: More geodetic + outlines.
    let rect = Rectangle::from_degrees(-1.0, -1.0, 1.0, 1.0);
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        scale_geometry(&rectangle_geometry(&rect, &ell, std::f64::consts::PI / 180.0, 0.0, vf), scale),
        Color::srgb(0.5, 0.8, 0.5),
        &mut idx,
    );

    let circle_center = ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0));
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        scale_geometry(&circle_geometry(circle_center, 100_000.0, &ell, 64, vf), scale),
        Color::srgb(0.5, 0.5, 0.8),
        &mut idx,
    );

    let polyvol_opts = PolylineVolumeOptions {
        positions: vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(-1.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 0.0, 0.0)),
        ],
        shape: vec![[-10000.0, -10000.0], [10000.0, -10000.0], [10000.0, 10000.0], [-10000.0, 10000.0]],
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: ell,
    };
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        scale_geometry(&polyline_volume_geometry(&polyvol_opts, vf), scale),
        Color::srgb(0.8, 0.5, 0.5),
        &mut idx,
    );

    let ground_opts = GroundPolylineOptions {
        positions: vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(-1.0, -1.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, -1.0, 0.0)),
        ],
        width: 20_000.0,
        granularity: std::f64::consts::PI / 180.0,
        closed: false,
        ellipsoid: ell,
    };
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        scale_geometry(&ground_polyline_geometry(&ground_opts, vf), scale),
        Color::srgb(0.5, 0.8, 0.8),
        &mut idx,
    );

    let frustum_def = FrustumDef::Perspective(PerspectiveFrustum::new(
        std::f64::consts::PI / 3.0,
        1.0,
        0.1,
        1.0,
    ));
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        frustum_geometry(&frustum_def, DVec3::ZERO, DQuat::IDENTITY, vf),
        Color::srgb(0.8, 0.8, 0.8),
        &mut idx,
    );

    // Row 4: Outlines (rendered as lines).
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        box_outline_geometry(DVec3::new(-0.5, -0.5, -0.5), DVec3::new(0.5, 0.5, 0.5)),
        Color::srgb(1.0, 0.3, 0.3),
        &mut idx,
    );
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        ellipsoid_outline_geometry(DVec3::new(0.5, 0.3, 0.4), 16, 32),
        Color::srgb(0.3, 1.0, 0.3),
        &mut idx,
    );
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        cylinder_outline_geometry(1.0, 0.5, 0.5, 32),
        Color::srgb(0.3, 0.3, 1.0),
        &mut idx,
    );
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        plane_outline_geometry(),
        Color::srgb(1.0, 1.0, 0.3),
        &mut idx,
    );
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        scale_geometry(&ellipse_outline_geometry(&ellipse_opts), scale),
        Color::srgb(0.3, 1.0, 1.0),
        &mut idx,
    );
}

/// Scales a geometry's positions by a factor.
fn scale_geometry(geo: &geometry::GeometryData, scale: f64) -> geometry::GeometryData {
    let mut scaled = geo.clone();
    for p in &mut scaled.positions {
        p[0] *= scale;
        p[1] *= scale;
        p[2] *= scale;
    }
    scaled.bounding_sphere.center *= scale;
    scaled.bounding_sphere.radius *= scale;
    scaled
}
