//! 几何展示 - 以网格形式展示所有几何类型。

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

/// 生成一个几何展示场景的插件。
pub struct GeometryShowcasePlugin;

impl Plugin for GeometryShowcasePlugin {
    /// 插件装配入口：在启动阶段挂载几何展示搭建系统。
    ///
    /// # 参数
    /// - `app`：Bevy 应用。
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_geometry_showcase);
    }
}

/// 启动系统：以网格布局生成各类几何体的展示实体。
///
/// # 参数
/// - `commands`：实体命令生成器。
/// - `meshes`：网格资产库。
/// - `materials`：标准材质资产库。
fn setup_geometry_showcase(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // 注意：Camera 与 Light 由 CesiumRenderPlugin 提供；
    // 此处不要重复 spawn，以避免顺序歧义警告。

    // 统一用 WGS84 椭球与全量顶点格式构建大地测量几何。
    let ell = Ellipsoid::WGS84;
    let vf = VertexFormat::ALL;

    // 网格布局：5 列 x 4 行，间距 3 个单位。
    // 以居中方式推算首格的世界坐标起点（x 按列数对称，z 按行数对称）。
    let spacing = 3.0;
    let cols = 5;
    let start_x = -(cols as f32 - 1.0) * spacing / 2.0;
    let start_z = -3.0 * spacing / 2.0;

    // idx 为当前格位计数，闭包每次调用后自增。
    let mut idx = 0;
    // 将单个几何转为网格并按格位坐标摆放，同时分配颜色与统一缩放。
    let spawn_geo = |commands: &mut Commands,
                         meshes: &mut Assets<Mesh>,
                         materials: &mut Assets<StandardMaterial>,
                         geo: geometry::GeometryData,
                         color: Color,
                         idx: &mut usize| {
        // 将几何数据转为可渲染网格。
        let mesh = geometry_to_mesh(&geo, None);
        // 由递增索引算出所属行/列，再换算为世界坐标 x/z。
        let row = *idx / cols;
        let col = *idx % cols;
        let x = start_x + col as f32 * spacing;
        let z = start_z + row as f32 * spacing;
        // 生成实体：网格 + 纯色材质 + 位置（附 0.8 缩放）。
        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            // 统一用传入颜色作为 base_color。
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                ..default()
            })),
            Transform::from_xyz(x, 0.0, z).with_scale(Vec3::splat(0.8)),
        ));
        *idx += 1;
    };

    // 第 1 行：基本 primitive。
    // 轴对齐立方体：由最小/最大角点定义。
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        box_geometry(DVec3::new(-0.5, -0.5, -0.5), DVec3::new(0.5, 0.5, 0.5), vf),
        // 立方体用红色高亮。
        Color::srgb(0.8, 0.2, 0.2),
        &mut idx,
    );
    // 球体：半径 0.5，16 环 x 32 段。
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        sphere_geometry(0.5, 16, 32, vf),
        // 球体用绿色。
        Color::srgb(0.2, 0.8, 0.2),
        &mut idx,
    );
    // 圆柱：长轴 1.0，两端半径 0.5，32 段。
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        cylinder_geometry(1.0, 0.5, 0.5, 32, vf),
        // 圆柱用蓝色。
        Color::srgb(0.2, 0.2, 0.8),
        &mut idx,
    );
    // 平面：默认单位 XY 平面。
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        plane_geometry(vf),
        // 平面用黄色。
        Color::srgb(0.8, 0.8, 0.2),
        &mut idx,
    );
    // 椭球：三轴半径 (0.5,0.3,0.4)，16 环 x 32 段。
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        ellipsoid_geometry(DVec3::new(0.5, 0.3, 0.4), 16, 32, vf),
        // 椭球用紫色。
        Color::srgb(0.8, 0.2, 0.8),
        &mut idx,
    );

    // 第 2 行：大地测量几何（缩小）。
    // 以赤道原点为中心；因真实尺度为米级，需缩小到场景单位。
    let center = ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0));
    let scale = 1e-6; // 从米缩小到场景单位。

    // 椭圆选项：长轴 100km、短轴 50km，粒度 1 度。
    let ellipse_opts = EllipseOptions {
        center,
        // 长半轴 100km / 短半轴 50km。
        semi_major_axis: 100_000.0,
        semi_minor_axis: 50_000.0,
        // 离地高度与旋转均为 0；粒度 1 度。
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
        // 渲染椭圆面（缩放到场景单位）。
        scale_geometry(&ellipse_geometry(&ellipse_opts, vf), scale),
        Color::srgb(0.2, 0.8, 0.8),
        &mut idx,
    );

    // 走廊选项：三段折线、宽 50km、圆角。
    let corridor_opts = CorridorOptions {
        // 中心线由西→中→北三段。
        positions: vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(-2.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 2.0, 0.0)),
        ],
        // 宽 50km、高度 0、圆角。
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
        // 渲染走廊面。
        scale_geometry(&corridor_geometry(&corridor_opts, vf), scale),
        Color::srgb(0.8, 0.5, 0.2),
        &mut idx,
    );

    // 墙体选项：四角围合矩形，高度 0→100km。
    let wall_opts = WallOptions::from_constant_heights(
        // 四个经纬度角点围成一圈。
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
        // 渲染墙体面。
        scale_geometry(&wall_geometry(&wall_opts, vf), scale),
        Color::srgb(0.5, 0.2, 0.8),
        &mut idx,
    );

    // 折线选项：三点折线、宽 20km。
    let polyline_opts = PolylineOptions {
        // 三点 V 形路径。
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
        // 渲染折线面。
        scale_geometry(&polyline_geometry(&polyline_opts, vf), scale),
        Color::srgb(0.2, 0.5, 0.8),
        &mut idx,
    );

    // 共面多边形选项：四角正方形。
    let coplanar_opts = CoplanarPolygonOptions {
        // 四角围合正方形（共面）。
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
        // 渲染共面多边形面。
        scale_geometry(&coplanar_polygon_geometry(&coplanar_opts, vf), scale),
        Color::srgb(0.8, 0.8, 0.5),
        &mut idx,
    );

    // 第 3 行：更多大地测量几何 + 轮廓线。
    // 矩形：经纬度围合的 ±1 度区域。
    let rect = Rectangle::from_degrees(-1.0, -1.0, 1.0, 1.0);
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        // 渲染矩形面。
        scale_geometry(&rectangle_geometry(&rect, &ell, std::f64::consts::PI / 180.0, 0.0, vf), scale),
        Color::srgb(0.5, 0.8, 0.5),
        &mut idx,
    );

    // 圆：赤道原点为中心，半径 100km，64 段。
    let circle_center = ell.cartographic_to_cartesian(&Cartographic::from_degrees(0.0, 0.0, 0.0));
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        // 渲染圆面。
        scale_geometry(&circle_geometry(circle_center, 100_000.0, &ell, 64, vf), scale),
        Color::srgb(0.5, 0.5, 0.8),
        &mut idx,
    );

    // 折线体：沿两点路径扫掠一个正方形截面。
    let polyvol_opts = PolylineVolumeOptions {
        // 扫掠路径为沿赤道两点。
        positions: vec![
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(-1.0, 0.0, 0.0)),
            ell.cartographic_to_cartesian(&Cartographic::from_degrees(1.0, 0.0, 0.0)),
        ],
        // 正方形截面（局部坐标 ±10km）。
        shape: vec![[-10000.0, -10000.0], [10000.0, -10000.0], [10000.0, 10000.0], [-10000.0, 10000.0]],
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: ell,
    };
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        // 渲染折线体面。
        scale_geometry(&polyline_volume_geometry(&polyvol_opts, vf), scale),
        Color::srgb(0.8, 0.5, 0.5),
        &mut idx,
    );

    // 贴地折线：三点路径、宽 20km、不闭合。
    let ground_opts = GroundPolylineOptions {
        // 三点 V 形贴地路径。
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
        // 渲染贴地折线面。
        scale_geometry(&ground_polyline_geometry(&ground_opts, vf), scale),
        Color::srgb(0.5, 0.8, 0.8),
        &mut idx,
    );

    // 视锥体：60 度垂直视角、1:1 宽高比、近/远平面 0.1/1.0。
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
        // 渲染视锥体面（不缩放，本身已是场景尺度）。
        frustum_geometry(&frustum_def, DVec3::ZERO, DQuat::IDENTITY, vf),
        Color::srgb(0.8, 0.8, 0.8),
        &mut idx,
    );

    // 第 4 行：轮廓线（以线段渲染）。
    // 以下均使用对应的 *_outline_geometry 变体，只构建边框网格。
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        // 立方体轮廓线。
        box_outline_geometry(DVec3::new(-0.5, -0.5, -0.5), DVec3::new(0.5, 0.5, 0.5)),
        Color::srgb(1.0, 0.3, 0.3),
        &mut idx,
    );
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        // 椭球轮廓线。
        ellipsoid_outline_geometry(DVec3::new(0.5, 0.3, 0.4), 16, 32),
        Color::srgb(0.3, 1.0, 0.3),
        &mut idx,
    );
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        // 圆柱轮廓线。
        cylinder_outline_geometry(1.0, 0.5, 0.5, 32),
        Color::srgb(0.3, 0.3, 1.0),
        &mut idx,
    );
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        // 平面轮廓线。
        plane_outline_geometry(),
        Color::srgb(1.0, 1.0, 0.3),
        &mut idx,
    );
    spawn_geo(
        &mut commands,
        &mut meshes,
        &mut materials,
        // 椭圆轮廓线（缩放到场景单位）。
        scale_geometry(&ellipse_outline_geometry(&ellipse_opts), scale),
        Color::srgb(0.3, 1.0, 1.0),
        &mut idx,
    );
}

/// 按一个因子缩放几何的位置。
/// 逐项乘以 scale，并同步缩放包围球中心与半径。
fn scale_geometry(geo: &geometry::GeometryData, scale: f64) -> geometry::GeometryData {
    // 先克隆以免变更传入几何。
    let mut scaled = geo.clone();
    // 将每个顶点坐标按因子缩放（大地测量几何为米级，需缩回场景单位）。
    for p in &mut scaled.positions {
        p[0] *= scale;
        p[1] *= scale;
        p[2] *= scale;
    }
    scaled.bounding_sphere.center *= scale;
    // 包围球中心与半径同步缩放，保持剔除/拾取正确。
    scaled.bounding_sphere.radius *= scale;
    scaled
}
