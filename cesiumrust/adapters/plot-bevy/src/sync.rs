//! The per-frame view sync: project the [`cesium_plot`] document onto render
//! layer 3 so every element shows in BOTH the 2D flat map and the 3D globe
//! (plan §2 / §3). This is the concrete reconciliation between the framework-
//! free scene model and the Bevy ECS.
//!
//! [`sync_visuals`] runs every frame and, in order:
//!  1. picks the single **active** camera (by projection type matching the view
//!     mode) and gathers its metrics into a [`ViewMetrics`];
//!  2. evaluates visibility through the pure [`eval_visibility`] (all ten
//!     dimensions folded);
//!  3. reconciles entities against the visible set — despawning stale ones and,
//!     only when the document content or the projection mode changed, rebuilding
//!     the mesh / label entities;
//!  4. writes per-frame geometry: billboard `Transform`s (position, camera
//!     rotation, constant-pixel scale), polyline ribbon meshes (constant pixel
//!     width at every depth) and label screen positions.
//!
//! All overlay entities carry [`RenderLayers::layer(3)`], the one layer shared by
//! both cameras, and an unlit [`StandardMaterial`] with the element's effective
//! colour — so the overlay needs no lighting and is colour-exact.

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

/// Flat-map overlay elevation. Imagery tiles float at `TILE_Z_ELEV + level`
/// (≲ 20 world units) under a camera at `z = 100`; the overlay sits well above
/// every tile so it is never depth-hidden by the basemap.
const FLAT_OVERLAY_Z: f32 = 50.0;

/// Which layer the whole overlay renders into — shared by the 2D and 3D cameras.
pub const OVERLAY_LAYER: usize = 3;

/// Remembers the last synced revision / mode / selection so a rebuild fires only
/// on real content, projection or selection changes, not on every idle frame.
#[derive(Resource, Default)]
pub struct SyncState {
    last_revision: u64,
    last_mode: cesium_plot::model::ViewMode,
    last_selection: BTreeSet<ElementId>,
    /// Camera pose + viewport signature of the last frame that ran the update
    /// loop (perf A2: lets a fully static frame skip it entirely).
    last_view: Option<ViewSig>,
    /// Visible set of that same frame, so a stationary-camera frame can tell the
    /// ECS already matches the scene and skip the whole reconcile + rewrite pass.
    last_visible: BTreeSet<ElementId>,
}

/// A cheap equality key over everything that changes an element's on-screen
/// placement *without* a document change: view mode, camera pose, focal length
/// and viewport. Two frames sharing a signature lay every visible element out
/// identically, so re-writing their transforms / meshes is redundant.
#[derive(Clone, Copy, PartialEq)]
struct ViewSig {
    mode: cesium_plot::model::ViewMode,
    translation: [f32; 3],
    rotation: [f32; 4],
    focal_px: f64,
    screen: [f32; 2],
    zoom: f32,
}

/// Camera-independent tessellation cache (perf A1). The densified / sampled
/// vertex chains a line or a face outline draws from depend only on the
/// element's stored geometry and the view mode — never on the camera — yet they
/// were recomputed for every visible element on every frame. This map holds them
/// keyed by element, flushed whenever the document revision or the mode changes
/// (the same triggers as a rebuild), so a moving camera simply reuses them and
/// only re-projects / re-ribbons the (now cached) coordinates.
#[derive(Resource, Default)]
pub struct PlotShapeCache {
    revision: u64,
    mode: Option<cesium_plot::model::ViewMode>,
    strokes: HashMap<ElementId, Vec<GeoPoint>>,
    faces: HashMap<ElementId, (Vec<GeoPoint>, Vec<Vec<GeoPoint>>)>,
}

/// The highlight colour a selected element is drawn in (keeps its alpha).
const SELECTED_TINT: [f32; 3] = [1.0, 0.85, 0.0];

/// The main view-sync system (see module docs).
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
    // 1. Active camera + projection metrics. Prefer the `is_active` camera whose
    //    projection matches the mode (perspective globe / orthographic flat),
    //    falling back to any active camera.
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

    // 2. Pure visibility pass.
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

    // Did any content, mode or selection input change since the last draw?
    let cur_sel: BTreeSet<ElementId> = selection
        .as_ref()
        .map(|s| s.0.iter().copied().collect())
        .unwrap_or_default();
    let rebuild = plot_doc.dirty
        || state.last_revision != plot_doc.revision
        || state.last_mode != ctx.mode
        || cur_sel != state.last_selection;

    // Perf A2 — stationary-camera fast path. When the document, the mode, the
    // selection, the camera pose / projection and the resulting visible set are
    // ALL identical to the last frame that drew, the ECS already mirrors the
    // scene exactly, so skip the whole reconcile + rewrite pass. Idle (non-
    // panning) frames then cost ~0 regardless of element count; the moment the
    // camera moves (or anything changes) `rebuild` or the signature differs and
    // the normal pass runs (where perf A1 keeps the re-densification cheap).
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

    // Perf A1 — drop the camera-independent tessellation cache whenever the
    // content or the projection mode changed, so the loop below re-samples once.
    if shapes_cache.revision != plot_doc.revision || shapes_cache.mode != Some(ctx.mode) {
        shapes_cache.strokes.clear();
        shapes_cache.faces.clear();
        shapes_cache.revision = plot_doc.revision;
        shapes_cache.mode = Some(ctx.mode);
    }

    // 3a. Despawn elements that left the visible set.
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

    // 3b. Full rebuild only on content / mode / selection change: clear live
    //     entities so the loop below re-creates them with fresh geometry,
    //     material and highlight.
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

    // Lazily-created UI root shared by all labels (only spawned if a label shows).
    let mut root: Option<Entity> = roots.iter().next();
    let mut root_bound = root.and_then(|r| bound.get(r).ok().map(|t| t.0));

    // 4. Per-element draw + per-frame geometry update.
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
                // Perf A1: reuse the cached densified stroke, sampling it once per
                // content / mode change instead of once per frame.
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
            _ => {} // M4+ geometry kinds (polygon / rect / circle / …) draw later.
        }
    }

    plot_doc.dirty = false;
    state.last_revision = plot_doc.revision;
    state.last_mode = ctx.mode;
    state.last_selection = cur_sel;
    state.last_view = Some(vsig);
    state.last_visible = visible;
}

/// Billboard screen size in px: the point diameter for a point, the icon box for
/// an icon (defaulting to 32 px when no icon style is set).
fn billboard_size_px(style: &Style, geometry: &Geometry) -> f64 {
    match geometry {
        Geometry::Icon(_) => style.icon.map(|i| i.size_px as f64).unwrap_or(32.0),
        _ => style.point_size_px as f64,
    }
}

/// The overlay world position of a geographic point (a fixed z lift in the flat
/// map so it clears the basemap; the ellipsoid surface in the globe).
fn overlay_world(metrics: &ViewMetrics, geo: GeoPoint) -> Vec3 {
    let mut w = metrics.project(geo);
    if matches!(metrics.mode, cesium_plot::model::ViewMode::Flat) {
        w.z = FLAT_OVERLAY_Z;
    }
    w
}

/// An unlit, blend-material painted with the element's effective colour, or the
/// selection tint when the element is currently selected (alpha is preserved).
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

/// A unit quad in the XY plane (extent `[-0.5, 0.5]`), front face toward +Z, so
/// a camera-facing billboard scaled by [`billboard_scale`] measures `size_px`.
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

/// The shared unit-quad handle, built on first use.
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

/// Create-or-update a camera-facing billboard for one point / icon element and
/// write its transform (position, rotation, constant-pixel scale) this frame.
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

/// Create-or-update a polyline's ribbon mesh and rewrite its vertices this frame
/// so the stroke keeps a constant pixel width at every depth / zoom.
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

/// Overwrite a mesh's geometry with the given ribbon vertices + indices.
fn write_ribbon(mesh: &mut Mesh, positions: &[[f32; 3]], indices: &[u32]) {
    let n = positions.len();
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.to_vec());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; n]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; n]);
    mesh.insert_indices(Indices::U32(indices.to_vec()));
}

/// A face material: unlit, blended, double-sided (the winding of an ear-cut
/// triangle projected onto the sphere is not guaranteed CCW in world space).
fn face_material(color: Rgba) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgba(color[0], color[1], color[2], color[3]),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    }
}

/// The fill colour (style fill scaled by opacity), or `None` for an outline-only
/// face.
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

/// Paint `c` with the selection tint (alpha preserved) when `selected`.
fn tinted(c: Rgba, selected: bool) -> Rgba {
    if selected {
        [SELECTED_TINT[0], SELECTED_TINT[1], SELECTED_TINT[2], c[3]]
    } else {
        c
    }
}

/// Triangulate a face (outer ring + holes) and build its filled world-space mesh.
/// Connectivity is computed in lon/lat (a valid plane for a simple face), the
/// vertices are then projected through the active mode so the same mesh is
/// correct in 2D and 3D.
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

/// Build the closed-ring outline ribbon (outer + every hole) as one merged
/// triangle strip set, `width_px` wide on screen at each vertex's depth.
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
        world.push(world[0]); // close the ring
        let (p, i) = ribbon(&world, &|k| {
            line_half_width(metrics, world[k.min(world.len() - 1)], width_px)
        }, normal);
        let base = pos.len() as u32;
        pos.extend(p);
        idx.extend(i.iter().map(|v| v + base));
    }
    (pos, idx)
}

/// Create-or-update a filled face: a static triangulated fill (rebuilt only when
/// the element / mode / selection changes) plus a screen-constant-width outline
/// stroke rewritten every frame.
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

    // Fill — built once per rebuild (world vertices are fixed for the mode).
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

    // Outline — constant pixel width, rewritten each frame like a polyline.
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

/// Create-or-update a label text node and write its absolute screen position
/// from the projected anchor (glued to the active camera's viewport).
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

/// Despawn every entity of a visual entry (mesh, face fill / outline, label).
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

    /// A headless app with just the bridge resources, the sync system and one
    /// active perspective camera. Returns the app and the camera entity so a
    /// test can re-point its projection.
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

    /// Two points, one polyline and one label — the four M2 primitive kinds.
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

    /// Count live mesh / label entities through the authoritative registry.
    fn counts(app: &App) -> (usize, usize) {
        let v = app.world().resource::<PlotVisuals>();
        let meshes = v.entries.values().filter(|e| e.mesh.is_some()).count();
        let labels = v.entries.values().filter(|e| e.label.is_some()).count();
        (meshes, labels)
    }

    /// Count live face fill + outline entities (M4 polygonal faces).
    fn face_counts(app: &App) -> (usize, usize) {
        let v = app.world().resource::<PlotVisuals>();
        let fills = v.entries.values().filter(|e| e.fill.is_some()).count();
        let outlines = v.entries.values().filter(|e| e.outline.is_some()).count();
        (fills, outlines)
    }

    /// A polygon, a rectangle and a circle — the three M4 filled-face kinds.
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
        // 2 points + 1 polyline ribbon as meshes; 1 label as UI text.
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
        // Hide the layer → nothing visible → every entry despawns and is dropped.
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
        // Re-point the one camera to an orthographic top-down projection and
        // switch the mode: a mode change forces a full rebuild in flat space.
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
        // Three filled faces → three fills and three outline strokes.
        assert_eq!(face_counts(&app), (3, 3));
        // Faces carry no plain billboard mesh.
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

    /// Locks the two Phase-A fast paths: the camera-independent shape cache is
    /// filled once, survives camera moves, is flushed on content change, and the
    /// stationary-frame skip never drops or churns entities.
    #[test]
    fn shape_cache_is_reused_across_frames_and_flushed_on_edit() {
        let (mut app, cam) = globe_app();
        {
            let mut doc = app.world_mut().resource_mut::<PlotDocument>();
            seed(&mut doc.doc);
            seed_faces(&mut doc.doc);
            doc.mark_dirty();
        }

        // First pass samples each line / face exactly once into the cache.
        app.update();
        assert_eq!(counts(&app), (3, 1));
        assert_eq!(face_counts(&app), (3, 3));
        {
            let c = app.world().resource::<PlotShapeCache>();
            assert_eq!(c.strokes.len(), 1, "polyline densified once");
            assert_eq!(c.faces.len(), 3, "poly + rect + circle sampled once");
        }

        // Perf A2: a fully stationary repeat frame is skipped — entities and the
        // cache must both stay put (no churn, no re-sample).
        app.update();
        assert_eq!(counts(&app), (3, 1));
        assert_eq!(face_counts(&app), (3, 3));
        assert_eq!(app.world().resource::<PlotShapeCache>().strokes.len(), 1);

        // A camera move changes the view signature so the rewrite pass runs
        // again, yet the cache is camera-independent and must survive intact.
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

        // A content change advances the revision, which flushes the cache; the
        // next pass must transparently re-sample (same counts, same cache size).
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
