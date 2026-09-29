//! 每帧视图同步：将 [`cesium_plot`] 文档投影到渲染层 3，以便每个元素同时显示在
//! 2D 平面地图和 3D 球体中（计划 §2 / §3）。这是无框架场景模型与
//! Bevy ECS 之间的具体协调。
//!
//! [`sync_visuals`] 每帧运行并按顺序：
//!  1. 选择单个**激活**相机（通过投影类型匹配视图模式）
//!     并将其度量采集到 [`ViewMetrics`]；
//!  2. 通过纯函数 [`eval_visibility`] 评估可见性（折叠全部十维）；
//!  3. 对可见集协调实体——销毁过时的，且仅在文档内容或
//!     投影模式变化时重建网格 / 标签实体；
//!  4. 写入每帧几何：billboard `Transform`（位置、相机
//!     旋转、恒定像素缩放）、多段线 ribbon 网格（每深度恒定像素宽）
//!     和标签屏幕位置。
//!
//! 所有叠加层实体携带 [`RenderLayers::layer(3)`]（两个相机共享的层）
//! 和一个使用元素有效颜色的无光照 [`StandardMaterial`]——
//! 因此叠加层不需要光照且颜色精确。

use std::collections::{BTreeSet, HashMap};

use bevy::prelude::*;
use bevy::pbr::StandardMaterial;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::view::RenderLayers;
use bevy::ui::TargetCamera;

use cesium_plot::geo::GeoPoint;
use cesium_plot::geom::tessellate::triangulate_holes;
use cesium_plot::model::geometry::{Geometry, LabelGeometry};
use cesium_plot::model::ids::ElementId;
use cesium_plot::model::{Rgba, Style};
use cesium_plot::model::ViewContext;
use cesium_plot::visibility::eval_visibility;

use crate::labels::{self, PlotUiRoot};
use crate::reproject::{billboard_scale, line_half_width, ribbon, ViewMetrics};
use crate::resources::{
    PlotDocument, PlotFilters, PlotLabel, PlotSelection, PlotViewCtx, PlotVisual, PlotVisuals,
};
use crate::shapes;

/// 平面地图叠加层高度。影像瓦片悬浮在 `TILE_Z_ELEV + level`
/// （≲ 20 世界单位）位于 `z = 100` 的相机下方；叠加层远高于
/// 每个瓦片，因此从不被底图深度遮挡。
const FLAT_OVERLAY_Z: f32 = 50.0;

/// 整个叠加层渲染进入的层—— 2D 和 3D 相机共享。
pub const OVERLAY_LAYER: usize = 3;

/// 记住上次同步的 revision / 模式 / 选择，以便重建只在真正的
/// 内容、投影或选择变化时触发，而非每个空闲帧。
#[derive(Resource, Default)]
pub struct SyncState {
    last_revision: u64,
    last_mode: cesium_plot::model::ViewMode,
    last_selection: BTreeSet<ElementId>,
    /// 上次运行更新循环的相机位姿 + 视口签名（性能 A2：让完全静止帧
    /// 可以整体跳过它）。
    last_view: Option<ViewSig>,
    /// 同一帧的可见集，以便静止相机帧可以判断 ECS 已经与场景匹配并
    /// 跳过整个协调 + 重写流程。
    last_visible: BTreeSet<ElementId>,
}

/// 一个廉价的等价键，覆盖所有在不改变文档的情况下改变元素屏幕位置的
/// 内容：视图模式、相机位姿、焦距和视口。共享同一签名的两帧
/// 会将每个可见元素布局完全相同，因此重写它们的 transform / mesh 是多余的。
#[derive(Clone, Copy, PartialEq)]
struct ViewSig {
    mode: cesium_plot::model::ViewMode,
    translation: [f32; 3],
    rotation: [f32; 4],
    focal_px: f64,
    screen: [f32; 2],
    zoom: f32,
}

/// 相机无关的剖分缓存（性能 A1）。线或面轮廓绘制的加密 / 采样
/// 顶点链只取决于元素存储的几何和视图模式——从不取决于相机——然而它们
/// 曾经对每个可见元素每帧重新计算。此图以元素为键持有它们，
/// 在文档 revision 或模式变化时刷新（与 rebuild 相同的触发条件），
/// 因此移动的相机只复用它们并仅重新投影 / 重新 ribbon（现已缓存的）坐标。
#[derive(Resource, Default)]
pub struct PlotShapeCache {
    revision: u64,
    mode: Option<cesium_plot::model::ViewMode>,
    strokes: HashMap<ElementId, Vec<GeoPoint>>,
    faces: HashMap<ElementId, (Vec<GeoPoint>, Vec<Vec<GeoPoint>>)>,
}

/// 已选元素的高亮颜色（保持其 alpha）。
const SELECTED_TINT: [f32; 3] = [1.0, 0.85, 0.0];

/// 主视图同步系统（参见模块文档）。
#[allow(clippy::too_many_arguments)]
pub fn sync_visuals(
    mut commands: Commands,
    ctx: Res<PlotViewCtx>,
    mut plot_doc: ResMut<PlotDocument>,
    filters: Res<PlotFilters>,
    mut visuals: ResMut<PlotVisuals>,
    mut state: ResMut<SyncState>,
    mut shapes_cache: ResMut<PlotShapeCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cams: Query<(Entity, &Camera, &GlobalTransform, &Projection)>,
    mut transforms: Query<&mut Transform, With<PlotVisual>>,
    mut nodes: Query<&mut Node, With<PlotLabel>>,
    roots: Query<Entity, With<PlotUiRoot>>,
    bound: Query<&TargetCamera>,
    selection: Option<Res<PlotSelection>>,
) {
    // 1. 激活相机 + 投影度量。优先选择投影匹配模式的 `is_active` 相机
    //    （透视球体 / 正交平面），回退到任意激活相机。
    let mut exact: Option<Entity> = None;
    let mut fallback: Option<Entity> = None;
    for (e, c, _gt, p) in cams.iter() {
        if !c.is_active {
            continue;
        }
        let matches = match ctx.mode {
            cesium_plot::model::ViewMode::Globe => matches!(p, Projection::Perspective(_)),
            cesium_plot::model::ViewMode::Flat => matches!(p, Projection::Orthographic(_)),
        };
        if matches {
            exact = Some(e);
            break;
        }
        if fallback.is_none() {
            fallback = Some(e);
        }
    }
    let Some(active) = exact.or(fallback) else {
        return;
    };
    let Ok((_e, cam, ct, proj)) = cams.get(active) else {
        return;
    };
    let rot = ct.rotation();
    let focal_px = match proj {
        Projection::Perspective(pp) => {
            let half = (pp.fov * 0.5).tan() as f64;
            if half > 1e-9 {
                (ctx.screen_h as f64 * 0.5) / half
            } else {
                0.0
            }
        }
        Projection::Orthographic(_) => 0.0,
    };
    let metrics = ViewMetrics {
        mode: ctx.mode,
        pixels_per_world: ctx.flat_zoom as f64,
        focal_px,
        cam_pos: ct.translation(),
    };

    // 2. 纯可见性评估。
    let ppw_rep = match ctx.mode {
        cesium_plot::model::ViewMode::Flat => metrics.pixels_per_world,
        cesium_plot::model::ViewMode::Globe => metrics.pixels_per_world_at(Vec3::ZERO),
    };
    let view = ViewContext {
        mode: ctx.mode,
        pixels_per_world: ppw_rep,
        meters_per_pixel: metrics.meters_per_pixel(),
        screen_w: ctx.screen_w as f64,
        screen_h: ctx.screen_h as f64,
        time_s: 0.0,
    };
    let result = eval_visibility(&plot_doc.doc, &view, &filters.0);
    let visible: BTreeSet<ElementId> = result.visible;

    // 自上次绘制以来是否有内容、模式或选择输入变化？
    let cur_sel: BTreeSet<ElementId> = selection
        .as_ref()
        .map(|s| s.0.iter().copied().collect())
        .unwrap_or_default();
    let rebuild = plot_doc.dirty
        || state.last_revision != plot_doc.revision
        || state.last_mode != ctx.mode
        || cur_sel != state.last_selection;

    // 性能 A2 —— 静止相机快速路径。当文档、模式、选择、相机位姿 / 投影
    // 和结果可见集都与上次绘制的帧完全相同时，ECS 已经精确镜像场景，
    // 因此跳过整个协调 + 重写流程。空闲（非平移）帧代价接近 0
    // 无论元素数量；当相机移动（或有变化）时 `rebuild` 或签名不同
    // 则正常流程运行（其中性能 A1 保持重新加密廉价）。
    let vsig = ViewSig {
        mode: ctx.mode,
        translation: ct.translation().to_array(),
        rotation: ct.rotation().to_array(),
        focal_px,
        screen: [ctx.screen_w, ctx.screen_h],
        zoom: ctx.flat_zoom,
    };
    if !rebuild && state.last_view == Some(vsig) && visible == state.last_visible {
        return;
    }

    // 性能 A1 —— 每当内容或投影模式变化时丢弃相机无关的剖分缓存，
    // 以便下面的循环重新采样一次。
    if shapes_cache.revision != plot_doc.revision || shapes_cache.mode != Some(ctx.mode) {
        shapes_cache.strokes.clear();
        shapes_cache.faces.clear();
        shapes_cache.revision = plot_doc.revision;
        shapes_cache.mode = Some(ctx.mode);
    }

    // 3a. 销毁离开可见集的元素。
    let gone: Vec<(ElementId, crate::resources::VisualEntry)> = visuals
        .entries
        .iter()
        .filter(|(id, _)| !visible.contains(id))
        .map(|(id, e)| (*id, e.clone()))
        .collect();
    for (_, entry) in gone {
        despawn_entry(&mut commands, &entry);
    }
    for id in visuals
        .entries
        .keys()
        .filter(|id| !visible.contains(id))
        .copied()
        .collect::<Vec<_>>()
    {
        visuals.entries.remove(&id);
    }

    // 3b. 仅在内容 / 模式 / 选择变化时完全重建：清除活跃实体
    //     以便下面的循环用新几何、材质和高亮重新创建它们。
    if rebuild {
        let entries: Vec<crate::resources::VisualEntry> =
            visuals.entries.values().cloned().collect();
        for entry in entries {
            despawn_entry(&mut commands, &entry);
        }
        for entry in visuals.entries.values_mut() {
            *entry = Default::default();
        }
    }

    // 懒创建的 UI 根节点，所有标签共享（仅当有标签显示时才创建）。
    let mut root: Option<Entity> = roots.iter().next();
    let mut root_bound = root.and_then(|r| bound.get(r).ok().map(|t| t.0));

    // 4. 逐元素绘制 + 每帧几何更新。
    for id in &visible {
        let Some(element) = plot_doc.doc.element(*id) else {
            continue;
        };
        let style = &element.style;
        let selected = selection
            .as_ref()
            .map(|s| s.contains(*id))
            .unwrap_or(false);
        match &element.geometry {
            Geometry::Point(p) | Geometry::Icon(cesium_plot::model::IconGeometry { at: p, .. }) => {
                let size_px = billboard_size_px(style, &element.geometry);
                update_billboard(
                    &mut commands,
                    &mut visuals,
                    &mut meshes,
                    &mut materials,
                    &mut transforms,
                    *id,
                    *p,
                    size_px,
                    &metrics,
                    rot,
                    style,
                    selected,
                );
            }
            Geometry::Polyline(_) | Geometry::Arc(_) | Geometry::Path(_) => {
                // 性能 A1：复用缓存的加密描边，每次内容 / 模式变化时采样一次
                // 而非每帧一次。
                if !shapes_cache.strokes.contains_key(id) {
                    if let Some(pos) = shapes::stroke_positions(&element.geometry, ctx.mode) {
                        shapes_cache.strokes.insert(*id, pos);
                    }
                }
                if let Some(pos) = shapes_cache.strokes.get(id) {
                    update_polyline(
                        &mut commands,
                        &mut visuals,
                        &mut meshes,
                        &mut materials,
                        *id,
                        pos,
                        &metrics,
                        rot,
                        style,
                        selected,
                    );
                }
            }
            Geometry::Polygon(_) | Geometry::Rectangle(_) | Geometry::Circle(_)
            | Geometry::Ellipse(_) => {
                if !shapes_cache.faces.contains_key(id) {
                    if let Some(rings) = shapes::face_rings(&element.geometry, ctx.mode) {
                        shapes_cache.faces.insert(*id, rings);
                    }
                }
                if let Some((outer, holes)) = shapes_cache.faces.get(id) {
                    update_face(
                        &mut commands,
                        &mut visuals,
                        &mut meshes,
                        &mut materials,
                        *id,
                        outer,
                        holes,
                        &metrics,
                        rot,
                        style,
                        selected,
                    );
                }
            }
            Geometry::Label(lg) => {
                if root.is_none() {
                    root = Some(labels::spawn_ui_root(&mut commands));
                }
                if let Some(r) = root {
                    if root_bound != Some(active) {
                        commands.entity(r).insert(TargetCamera(active));
                        root_bound = Some(active);
                    }
                    update_label(
                        &mut commands,
                        &mut visuals,
                        &mut nodes,
                        *id,
                        lg,
                        style,
                        &metrics,
                        cam,
                        ct,
                        r,
                    );
                }
            }
            _ => {} // M4+ 几何类型（多边形 / 矩形 / 圆 / …）稍后绘制。
        }
    }

    plot_doc.dirty = false;
    state.last_revision = plot_doc.revision;
    state.last_mode = ctx.mode;
    state.last_selection = cur_sel;
    state.last_view = Some(vsig);
    state.last_visible = visible;
}

/// Billboard 屏幕尺寸（px）：点的直径用于点元素，图标框尺寸用于图标元素
/// （未设置图标样式时默认 32 px）。
fn billboard_size_px(style: &Style, geometry: &Geometry) -> f64 {
    match geometry {
        Geometry::Icon(_) => style.icon.map(|i| i.size_px as f64).unwrap_or(32.0),
        _ => style.point_size_px as f64,
    }
}

/// 地理坐标的叠加层世界位置（平面地图中固定 z 提升以超过底图；
/// 球体中为椭球表面）。
fn overlay_world(metrics: &ViewMetrics, geo: GeoPoint) -> Vec3 {
    let mut w = metrics.project(geo);
    if matches!(metrics.mode, cesium_plot::model::ViewMode::Flat) {
        w.z = FLAT_OVERLAY_Z;
    }
    w
}

/// 一个无光照、混合材质，使用元素的有效颜色绘制，或当元素被当前选中时使用选择高亮（alpha 保持不变）。
fn overlay_material(style: &Style, selected: bool) -> StandardMaterial {
    let c = style.effective_color();
    let rgb = if selected {
        SELECTED_TINT
    } else {
        [c[0], c[1], c[2]]
    };
    StandardMaterial {
        base_color: Color::srgba(rgb[0], rgb[1], rgb[2], c[3]),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    }
}

/// XY 平面中的单位 quad（范围 `[-0.5, 0.5]`），正面朝 +Z，因此由
/// [`billboard_scale`] 缩放的面向相机的 billboard 在屏幕上量为 `size_px`。
fn build_unit_quad() -> Mesh {
    let positions = [[-0.5, -0.5, 0.0], [0.5, -0.5, 0.0], [0.5, 0.5, 0.0], [-0.5, 0.5, 0.0]];
    let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let normals = [[0.0, 0.0, 1.0]; 4];
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.to_vec());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals.to_vec());
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs.to_vec());
    mesh.insert_indices(Indices::U16(vec![0, 1, 2, 0, 2, 3]));
    mesh
}

/// 共享的单位 quad 句柄，首次使用时创建。
fn ensure_quad(
    visuals: &mut PlotVisuals,
    meshes: &mut Assets<Mesh>,
) -> Handle<Mesh> {
    if let Some(h) = &visuals.quad {
        return h.clone();
    }
    let h = meshes.add(build_unit_quad());
    visuals.quad = Some(h.clone());
    h
}

/// 为单个点 / 图标元素创建或更新面向相机的 billboard 并写入其本帧
/// transform（位置、旋转、恒定像素缩放）。
#[allow(clippy::too_many_arguments)]
fn update_billboard(
    commands: &mut Commands,
    visuals: &mut PlotVisuals,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    transforms: &mut Query<&mut Transform, With<PlotVisual>>,
    id: ElementId,
    geo: GeoPoint,
    size_px: f64,
    metrics: &ViewMetrics,
    rot: Quat,
    style: &Style,
    selected: bool,
) {
    let quad = ensure_quad(visuals, meshes);
    let world = overlay_world(metrics, geo);
    let entry = visuals.entries.entry(id).or_default();
    if entry.mesh.is_none() {
        let mat = materials.add(overlay_material(style, selected));
        let e = commands
            .spawn((
                PlotVisual { element: id },
                Mesh3d(quad.clone()),
                MeshMaterial3d(mat.clone()),
                RenderLayers::layer(OVERLAY_LAYER),
                Visibility::Visible,
                Transform::from_translation(world),
            ))
            .id();
        entry.mesh = Some(e);
        entry.mesh_handle = Some(quad);
        entry.mat = Some(mat);
    }
    if let Some(e) = entry.mesh {
        if let Ok(mut tf) = transforms.get_mut(e) {
            tf.translation = world;
            tf.rotation = rot;
            tf.scale = billboard_scale(metrics, world, size_px);
        }
    }
}

/// 创建或更新多段线的 ribbon 网格并重写其本帧顶点，
/// 以便描边在每个深度 / 缩放下保持恒定像素宽度。
#[allow(clippy::too_many_arguments)]
fn update_polyline(
    commands: &mut Commands,
    visuals: &mut PlotVisuals,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    id: ElementId,
    positions: &[GeoPoint],
    metrics: &ViewMetrics,
    rot: Quat,
    style: &Style,
    selected: bool,
) {
    if positions.len() < 2 {
        return;
    }
    let world: Vec<Vec3> = positions.iter().map(|g| overlay_world(metrics, *g)).collect();
    let normal = rot * Vec3::Z;
    let width = style.width_px as f64;
    let (pos, idx) = ribbon(&world, &|i| line_half_width(metrics, world[i], width), normal);
    if pos.is_empty() {
        return;
    }

    let entry = visuals.entries.entry(id).or_default();
    if entry.mesh.is_none() {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
        write_ribbon(&mut mesh, &pos, &idx);
        let handle = meshes.add(mesh);
        let mat = materials.add(overlay_material(style, selected));
        let e = commands
            .spawn((
                PlotVisual { element: id },
                Mesh3d(handle.clone()),
                MeshMaterial3d(mat.clone()),
                RenderLayers::layer(OVERLAY_LAYER),
                Visibility::Visible,
                Transform::IDENTITY,
            ))
            .id();
        entry.mesh = Some(e);
        entry.mesh_handle = Some(handle);
        entry.mat = Some(mat);
    } else if let Some(h) = entry.mesh_handle.clone() {
        if let Some(m) = meshes.get_mut(&h) {
            write_ribbon(m, &pos, &idx);
        }
    }
}

/// 用给定的 ribbon 顶点 + 索引覆写网格的几何。
fn write_ribbon(mesh: &mut Mesh, positions: &[[f32; 3]], indices: &[u32]) {
    let n = positions.len();
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.to_vec());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; n]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; n]);
    mesh.insert_indices(Indices::U32(indices.to_vec()));
}

/// 面材质：无光照、混合、双面（耳切三角剖分投影到球面后的绕序
/// 在世界空间中不保证 CCW）。
fn face_material(color: Rgba) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgba(color[0], color[1], color[2], color[3]),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    }
}

/// 填充颜色（样式 fill 乘以不透明度），或为 `None` 表示纯轮廓面。
fn fill_color(style: &Style) -> Option<Rgba> {
    style.fill.map(|f| {
        let mut c = f;
        c[3] *= style.opacity;
        c
    })
}

/// The outline colour + screen width: an explicit outline wins, else the base
/// (effective) colour at the line width.
fn outline_style(style: &Style) -> (Rgba, f64) {
    match style.outline {
        Some(o) => {
            let mut c = o.color;
            c[3] *= style.opacity;
            (c, o.width_px as f64)
        }
        None => (style.effective_color(), style.width_px as f64),
    }
}

/// `selected` 时用选择高亮绘制 `c`（保持 alpha）。
fn tinted(c: Rgba, selected: bool) -> Rgba {
    if selected {
        [SELECTED_TINT[0], SELECTED_TINT[1], SELECTED_TINT[2], c[3]]
    } else {
        c
    }
}

/// 剖分一个面（外环 + 孔）并构建其填充世界空间网格。
/// 连接性在经纬度中计算（对于简单面是有效平面），
/// 顶点然后通过激活模式投影以便同一网格在 2D 和 3D 中都正确。
fn build_face_mesh(
    outer: &[GeoPoint],
    holes: &[Vec<GeoPoint>],
    metrics: &ViewMetrics,
) -> Mesh {
    let mut geos: Vec<GeoPoint> = outer.to_vec();
    for h in holes {
        geos.extend(h.iter().copied());
    }
    let planar: Vec<[f64; 2]> = geos.iter().map(|g| [g.lon_deg, g.lat_deg]).collect();
    let outer_planar = &planar[..outer.len()];
    let hole_planars: Vec<Vec<[f64; 2]>> = holes
        .iter()
        .map(|h| h.iter().map(|g| [g.lon_deg, g.lat_deg]).collect())
        .collect();
    let idx = triangulate_holes(outer_planar, &hole_planars);
    let positions: Vec<[f32; 3]> = geos
        .iter()
        .map(|g| overlay_world(metrics, *g).to_array())
        .collect();
    let n = positions.len();
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; n]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; n]);
    if !idx.is_empty() {
        mesh.insert_indices(Indices::U32(idx));
    }
    mesh
}

/// 构建闭合环轮廓 ribbon（外部 + 每个孔）作为一个合并的
/// 三角带集，在每个顶点深度处屏幕宽 `width_px`。
fn build_face_outline(
    outer: &[GeoPoint],
    holes: &[Vec<GeoPoint>],
    metrics: &ViewMetrics,
    rot: Quat,
    width_px: f64,
) -> (Vec<[f32; 3]>, Vec<u32>) {
    let normal = rot * Vec3::Z;
    let mut pos: Vec<[f32; 3]> = Vec::new();
    let mut idx: Vec<u32> = Vec::new();
    let rings = std::iter::once(outer)
        .chain(holes.iter().map(|h| h.as_slice()));
    for ring in rings {
        if ring.len() < 2 {
            continue;
        }
        let mut world: Vec<Vec3> = ring.iter().map(|g| overlay_world(metrics, *g)).collect();
        world.push(world[0]); // 闭合环
        let (p, i) = ribbon(&world, &|k| {
            line_half_width(metrics, world[k.min(world.len() - 1)], width_px)
        }, normal);
        let base = pos.len() as u32;
        pos.extend(p);
        idx.extend(i.iter().map(|v| v + base));
    }
    (pos, idx)
}

/// 创建或更新一个填充面：静态三角填充（仅在元素 / 模式 / 选择变化时重建）
/// 加上每帧重写的屏幕恒定宽轮廓描边。
#[allow(clippy::too_many_arguments)]
fn update_face(
    commands: &mut Commands,
    visuals: &mut PlotVisuals,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    id: ElementId,
    outer: &[GeoPoint],
    holes: &[Vec<GeoPoint>],
    metrics: &ViewMetrics,
    rot: Quat,
    style: &Style,
    selected: bool,
) {
    let entry = visuals.entries.entry(id).or_default();

    // 填充 —— 每次 rebuild 构建一次（世界顶点对于模式是固定的）。
    if entry.fill.is_none() {
        if let Some(fc) = fill_color(style) {
            let handle = meshes.add(build_face_mesh(outer, holes, metrics));
            let mat = materials.add(face_material(tinted(fc, selected)));
            let e = commands
                .spawn((
                    PlotVisual { element: id },
                    Mesh3d(handle.clone()),
                    MeshMaterial3d(mat.clone()),
                    RenderLayers::layer(OVERLAY_LAYER),
                    Visibility::Visible,
                    Transform::IDENTITY,
                ))
                .id();
            entry.fill = Some(e);
            entry.fill_handle = Some(handle);
            entry.fill_mat = Some(mat);
        }
    }

    // 轮廓 —— 恒定像素宽度，每帧像多段线一样重写。
    let (oc, ow) = outline_style(style);
    let (pos, idx) = build_face_outline(outer, holes, metrics, rot, ow);
    if pos.is_empty() {
        return;
    }
    let entry = visuals.entries.entry(id).or_default();
    if entry.outline.is_none() {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
        write_ribbon(&mut mesh, &pos, &idx);
        let handle = meshes.add(mesh);
        let mat = materials.add(face_material(tinted(oc, selected)));
        let e = commands
            .spawn((
                PlotVisual { element: id },
                Mesh3d(handle.clone()),
                MeshMaterial3d(mat.clone()),
                RenderLayers::layer(OVERLAY_LAYER),
                Visibility::Visible,
                Transform::IDENTITY,
            ))
            .id();
        entry.outline = Some(e);
        entry.outline_handle = Some(handle);
        entry.outline_mat = Some(mat);
    } else if let Some(h) = entry.outline_handle.clone() {
        if let Some(m) = meshes.get_mut(&h) {
            write_ribbon(m, &pos, &idx);
        }
    }
}

/// 创建或更新一个标签文本节点并从投影锚点写入其绝对屏幕位置
/// （粘附到激活相机的视口）。
#[allow(clippy::too_many_arguments)]
fn update_label(
    commands: &mut Commands,
    visuals: &mut PlotVisuals,
    nodes: &mut Query<&mut Node, With<PlotLabel>>,
    id: ElementId,
    lg: &LabelGeometry,
    style: &Style,
    metrics: &ViewMetrics,
    cam: &Camera,
    ct: &GlobalTransform,
    root: Entity,
) {
    let entry = visuals.entries.entry(id).or_default();
    if entry.label.is_none() {
        entry.label = Some(labels::spawn_label(commands, root, id, lg, style));
    }
    let world = overlay_world(metrics, lg.at);
    if let (Some(le), Some(sp)) = (entry.label, labels::world_to_screen(cam, ct, world)) {
        if let Ok(mut node) = nodes.get_mut(le) {
            let off = labels::anchor_offset(lg.anchor, lg.offset_px, Vec2::ZERO);
            node.left = Val::Px(sp.x + off.x);
            node.top = Val::Px(sp.y + off.y);
        }
    }
}

/// 销毁视觉条目的所有实体（网格、面填充 / 轮廓、标签）。
fn despawn_entry(commands: &mut Commands, entry: &crate::resources::VisualEntry) {
    if let Some(m) = entry.mesh {
        commands.entity(m).despawn();
    }
    if let Some(f) = entry.fill {
        commands.entity(f).despawn();
    }
    if let Some(o) = entry.outline {
        commands.entity(o).despawn();
    }
    if let Some(l) = entry.label {
        commands.entity(l).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::{PlotInputCapture, PlotViewMode};
    use bevy::app::App;
    use cesium_plot::model::geometry::{
        Circle, LabelGeometry, Polygon, Polyline, Rectangle,
    };
    use cesium_plot::model::ids::LayerId;
    use cesium_plot::model::Document;

    /// 一个 headless 应用，只包含桥接资源、同步系统和一个激活的透视相机。
    /// 返回应用和相机实体，以便测试可以重新指向其投影。
    fn globe_app() -> (App, Entity) {
        let mut app = App::new();
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<PlotViewCtx>()
            .init_resource::<PlotInputCapture>()
            .init_resource::<PlotDocument>()
            .init_resource::<PlotFilters>()
            .init_resource::<PlotVisuals>()
            .init_resource::<SyncState>()
            .init_resource::<PlotShapeCache>()
            .add_systems(Update, sync_visuals);
        let cam = app
            .world_mut()
            .spawn((
                Camera {
                    is_active: true,
                    ..default()
                },
                GlobalTransform::from_translation(Vec3::new(3.0, 0.0, 0.0)),
                Projection::Perspective(PerspectiveProjection {
                    fov: std::f32::consts::FRAC_PI_3,
                    ..default()
                }),
            ))
            .id();
        {
            let mut ctx = app.world_mut().resource_mut::<PlotViewCtx>();
            ctx.mode = PlotViewMode::Globe;
            ctx.screen_w = 800.0;
            ctx.screen_h = 600.0;
        }
        (app, cam)
    }

    /// 两个点、一条多段线和一个标签——四种 M2 图元类型。
    fn seed(doc: &mut Document) -> LayerId {
        let layer = doc.new_layer("L");
        let p1 = doc.make_element("a", Geometry::Point(GeoPoint::surface(0.0, 0.0)));
        let p2 = doc.make_element("b", Geometry::Point(GeoPoint::surface(10.0, 20.0)));
        doc.add_element_to_layer(layer, p1);
        doc.add_element_to_layer(layer, p2);
        let line = doc.make_element(
            "line",
            Geometry::Polyline(Polyline {
                positions: vec![
                    GeoPoint::surface(0.0, 0.0),
                    GeoPoint::surface(1.0, 1.0),
                    GeoPoint::surface(2.0, 0.0),
                ],
            }),
        );
        doc.add_element_to_layer(layer, line);
        let label = doc.make_element(
            "lbl",
            Geometry::Label(LabelGeometry {
                at: GeoPoint::surface(5.0, 5.0),
                text: "hello".into(),
                anchor: cesium_plot::model::LabelAnchor::Center,
                offset_px: [0.0, 0.0],
            }),
        );
        doc.add_element_to_layer(layer, label);
        layer
    }

    /// 通过权威注册表统计活跃网格 / 标签实体数量。
    fn counts(app: &App) -> (usize, usize) {
        let v = app.world().resource::<PlotVisuals>();
        let meshes = v.entries.values().filter(|e| e.mesh.is_some()).count();
        let labels = v.entries.values().filter(|e| e.label.is_some()).count();
        (meshes, labels)
    }

    /// 统计活跃面填充 + 轮廓实体数量（M4 多边形面）。
    fn face_counts(app: &App) -> (usize, usize) {
        let v = app.world().resource::<PlotVisuals>();
        let fills = v.entries.values().filter(|e| e.fill.is_some()).count();
        let outlines = v.entries.values().filter(|e| e.outline.is_some()).count();
        (fills, outlines)
    }

    /// 一个多边形、一个矩形和一个圆——三种 M4 填充面类型。
    fn seed_faces(doc: &mut Document) {
        let layer = doc.new_layer("F");
        let poly = doc.make_element(
            "poly",
            Geometry::Polygon(Polygon {
                outer: vec![
                    GeoPoint::surface(0.0, 0.0),
                    GeoPoint::surface(2.0, 0.0),
                    GeoPoint::surface(2.0, 2.0),
                    GeoPoint::surface(0.0, 2.0),
                ],
                holes: Vec::new(),
            }),
        );
        let rect = doc.make_element(
            "rect",
            Geometry::Rectangle(Rectangle {
                west: 5.0,
                south: 5.0,
                east: 8.0,
                north: 8.0,
            }),
        );
        let circle = doc.make_element(
            "circle",
            Geometry::Circle(Circle {
                center: GeoPoint::surface(20.0, 20.0),
                radius_m: 100_000.0,
            }),
        );
        doc.add_element_to_layer(layer, poly);
        doc.add_element_to_layer(layer, rect);
        doc.add_element_to_layer(layer, circle);
    }

    #[test]
    fn empty_document_draws_nothing() {
        let (mut app, _cam) = globe_app();
        app.update();
        assert_eq!(counts(&app), (0, 0));
    }

    #[test]
    fn points_line_and_label_are_reconciled() {
        let (mut app, _cam) = globe_app();
        {
            let mut doc = app.world_mut().resource_mut::<PlotDocument>();
            seed(&mut doc.doc);
            doc.mark_dirty();
        }
        app.update();
        // 2 点 + 1 多段线 ribbon 作为网格；1 标签作为 UI 文本。
        assert_eq!(counts(&app), (3, 1));
    }

    #[test]
    fn hidden_layer_despawns_its_element() {
        let (mut app, _cam) = globe_app();
        let layer = {
            let mut doc = app.world_mut().resource_mut::<PlotDocument>();
            let l = seed(&mut doc.doc);
            doc.mark_dirty();
            l
        };
        app.update();
        assert_eq!(counts(&app), (3, 1));
        // 隐藏层 → 无可见内容 → 每个条目销毁并被丢弃。
        {
            let mut doc = app.world_mut().resource_mut::<PlotDocument>();
            doc.doc.layer_mut(layer).unwrap().visible = false;
            doc.mark_dirty();
        }
        app.update();
        assert_eq!(counts(&app), (0, 0));
    }

    #[test]
    fn switching_to_flat_rebuilds_everything() {
        let (mut app, cam) = globe_app();
        {
            let mut doc = app.world_mut().resource_mut::<PlotDocument>();
            seed(&mut doc.doc);
            doc.mark_dirty();
        }
        app.update();
        assert_eq!(counts(&app), (3, 1));
        // 将唯一相机重新指向正交俯视投影并切换模式：
        // 模式变化强制在平面空间中完全重建。
        {
            let mut p = app.world_mut().get_mut::<Projection>(cam).unwrap();
            *p = Projection::Orthographic(OrthographicProjection {
                scale: 1.0 / 200.0,
                ..OrthographicProjection::default_3d()
            });
        }
        {
            let mut ct = app.world_mut().get_mut::<GlobalTransform>(cam).unwrap();
            *ct = GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 100.0));
        }
        {
            let mut ctx = app.world_mut().resource_mut::<PlotViewCtx>();
            ctx.mode = PlotViewMode::Flat;
            ctx.flat_zoom = 200.0;
        }
        app.update();
        assert_eq!(counts(&app), (3, 1), "same visible set, rebuilt flat");
    }

    #[test]
    fn faces_get_a_fill_and_an_outline() {
        let (mut app, _cam) = globe_app();
        {
            let mut doc = app.world_mut().resource_mut::<PlotDocument>();
            seed_faces(&mut doc.doc);
            doc.mark_dirty();
        }
        app.update();
        // 三个填充面 → 三个填充和三个轮廓描边。
        assert_eq!(face_counts(&app), (3, 3));
        // 面不携带普通 billboard 网格。
        assert_eq!(counts(&app), (0, 0));
    }

    #[test]
    fn hiding_a_face_layer_despawns_fill_and_outline() {
        let (mut app, _cam) = globe_app();
        let layer = {
            let mut doc = app.world_mut().resource_mut::<PlotDocument>();
            seed_faces(&mut doc.doc);
            let l = doc.doc.layers().first().unwrap().id;
            doc.mark_dirty();
            l
        };
        app.update();
        assert_eq!(face_counts(&app), (3, 3));
        {
            let mut doc = app.world_mut().resource_mut::<PlotDocument>();
            doc.doc.layer_mut(layer).unwrap().visible = false;
            doc.mark_dirty();
        }
        app.update();
        assert_eq!(face_counts(&app), (0, 0), "hide despawns every face entity");
    }

    /// 锁定两个 Phase-A 快速路径：相机无关的形状缓存只填充一次、
    /// 存活相机移动、在内容变化时刷新，且静止帧跳过从不丢弃或 churn 实体。
    #[test]
    fn shape_cache_is_reused_across_frames_and_flushed_on_edit() {
        let (mut app, cam) = globe_app();
        {
            let mut doc = app.world_mut().resource_mut::<PlotDocument>();
            seed(&mut doc.doc);
            seed_faces(&mut doc.doc);
            doc.mark_dirty();
        }

        // 第一次遍历将每条线 / 面恰好采样一次到缓存中。
        app.update();
        assert_eq!(counts(&app), (3, 1));
        assert_eq!(face_counts(&app), (3, 3));
        {
            let c = app.world().resource::<PlotShapeCache>();
            assert_eq!(c.strokes.len(), 1, "polyline densified once");
            assert_eq!(c.faces.len(), 3, "poly + rect + circle sampled once");
        }

        // 性能 A2：完全静止的重复帧被跳过——实体和缓存都必须保持不变（无 churn、无重采样）。
        app.update();
        assert_eq!(counts(&app), (3, 1));
        assert_eq!(face_counts(&app), (3, 3));
        assert_eq!(app.world().resource::<PlotShapeCache>().strokes.len(), 1);

        // 相机移动改变视图签名因此重写流程再次运行，
        // 但缓存是相机无关的必须完整存活。
        app.world_mut()
            .entity_mut(cam)
            .insert(GlobalTransform::from_translation(Vec3::new(4.0, 1.0, 2.0)));
        app.update();
        assert_eq!(counts(&app), (3, 1));
        assert_eq!(face_counts(&app), (3, 3));
        {
            let c = app.world().resource::<PlotShapeCache>();
            assert_eq!(c.strokes.len(), 1, "cache reused across camera move");
            assert_eq!(c.faces.len(), 3);
        }

        // 内容变化推进 revision，从而刷新缓存，
        // 下一遍历必须透明地重新采样（相同计数、相同缓存大小）。
        app.world_mut().resource_mut::<PlotDocument>().mark_dirty();
        app.update();
        assert_eq!(counts(&app), (3, 1));
        assert_eq!(face_counts(&app), (3, 3));
        {
            let c = app.world().resource::<PlotShapeCache>();
            assert_eq!(c.strokes.len(), 1, "cache repopulated after flush");
            assert_eq!(c.faces.len(), 3);
        }
    }
}
