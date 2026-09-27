//! The interaction state machine (plan §8): the drawing tool. M5 lands the
//! finite-state logic (tool selection → click-to-add-vertex → live draft →
//! commit / cancel), the input-capture gate that hands the pointer to the plot
//! overlay while a draw is active, and a rubber-band preview of the in-progress
//! draft. Editing / move / history land on top of the same command layer at M6.
//!
//! The state transitions are split from the ECS so they are deterministic and
//! unit-testable with no window or camera: [`PlotInteraction`] holds the current
//! [`PlotTool`] and draft vertices and folds clicks through the pure
//! [`commit_draft`]. The [`interaction_system`] is the thin shell that gathers
//! raw input, resolves the cursor to a geographic point through the *engine's*
//! inverse projection (mirroring the pick path, so a drawn vertex lands exactly
//! where the cursor is), and applies the resulting [`PlotCommand`] to the
//! document.

use bevy::input::mouse::MouseButton;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::view::RenderLayers;
use bevy::window::PrimaryWindow;

use cesium_plot::geo::{flat_to_geo, GeoPoint};
use cesium_plot::model::geometry::Geometry;
use cesium_plot::model::ids::{ElementId, LayerId};
use cesium_plot::model::ViewMode;
use cesium_plot::ops::{commit_draft, snap, DrawKind, PlotCommand};

use crate::reproject::{line_half_width, ribbon, ViewMetrics};
use crate::resources::{
    PlotDocument, PlotHistory, PlotInputCapture, PlotSelection, PlotSnap, PlotViewCtx,
};
use crate::surface;
use crate::sync::OVERLAY_LAYER;

/// Which tool the overlay is in. `Idle` lets the camera own the pointer; a
/// `Draw` kind captures it to build a new element.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PlotTool {
    /// No active tool (default): the pointer drives the camera / selection.
    #[default]
    Idle,
    /// Drawing `kind`: clicks append draft vertices.
    Draw(DrawKind),
}

/// Request a tool change (the toolbar / hotkeys send these; the FSM applies them
/// at the start of the next frame).
#[derive(Event, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlotSetTool(pub PlotTool);

/// Emitted when a draw is committed (or abandoned), so the UI can react.
#[derive(Event, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlotDrawFinished {
    /// The id of the committed element, or `None` when the draw was cancelled.
    pub committed: Option<ElementId>,
}

/// The interaction FSM state: the current tool and the in-progress draft.
#[derive(Resource, Default)]
pub struct PlotInteraction {
    /// Active tool.
    pub tool: PlotTool,
    /// Draft vertices accumulated for the current draw.
    pub draft: Vec<GeoPoint>,
}

impl PlotInteraction {
    /// Start drawing `kind` from an empty draft.
    pub fn begin(&mut self, kind: DrawKind) {
        self.tool = PlotTool::Draw(kind);
        self.draft.clear();
    }

    /// Drop back to idle, discarding the draft.
    pub fn stop(&mut self) {
        self.tool = PlotTool::Idle;
        self.draft.clear();
    }

    /// Whether a draw tool is active.
    #[inline]
    pub fn is_drawing(&self) -> bool {
        matches!(self.tool, PlotTool::Draw(_))
    }

    /// Record a click. Returns a committed geometry when a fixed-point kind
    /// (point / rectangle / circle) reaches its required count — the draft then
    /// resets to idle. Open kinds (polyline / polygon) keep drawing until
    /// [`finish`](Self::finish).
    pub fn add_point(&mut self, geo: GeoPoint) -> Option<Geometry> {
        let PlotTool::Draw(kind) = self.tool else {
            return None;
        };
        self.draft.push(geo);
        if let Some(n) = kind.fixed_points() {
            if self.draft.len() >= n {
                let g = commit_draft(kind, &self.draft);
                self.stop();
                return g;
            }
        }
        None
    }

    /// Finish an open-ended draw: fold the draft if valid (else stay drawing so
    /// the user can keep adding points).
    pub fn finish(&mut self) -> Option<Geometry> {
        let PlotTool::Draw(kind) = self.tool else {
            return None;
        };
        let g = commit_draft(kind, &self.draft);
        if g.is_some() {
            self.stop();
        }
        g
    }

    /// Cancel the current draw.
    pub fn cancel(&mut self) {
        self.stop();
    }

    /// Delete the last draft vertex (Backspace).
    pub fn backspace(&mut self) {
        if self.is_drawing() {
            self.draft.pop();
        }
    }
}

/// Resolve a logical-pixel cursor position to a geographic point through the
/// active camera's inverse projection — the pick-path counterpart used for
/// placing draft vertices (plan §3, "落点 = screen_to_geo").
pub fn screen_to_geo(
    cam: &Camera,
    ct: &GlobalTransform,
    mode: ViewMode,
    cursor: Vec2,
) -> Option<GeoPoint> {
    match mode {
        ViewMode::Flat => {
            let xy = cam.viewport_to_world_2d(ct, cursor).ok()?;
            Some(flat_to_geo(xy.as_dvec2()))
        }
        ViewMode::Globe => {
            let ray = cam.viewport_to_world(ct, cursor).ok()?;
            surface::ray_to_globe(ray.origin, ray.direction.into())
        }
    }
}

/// Commit a finished geometry into the document's active layer (creating one if
/// the document is empty), select it, record the add on the history stack and
/// bump the revision. Returns the new id.
pub fn commit_geometry(
    plot_doc: &mut PlotDocument,
    history: &mut PlotHistory,
    selection: &mut PlotSelection,
    kind: DrawKind,
    geometry: Geometry,
) -> ElementId {
    let layer: LayerId = match plot_doc.doc.active_layer() {
        Some(l) => l,
        None => {
            let l = plot_doc.doc.new_layer("默认层");
            plot_doc.doc.set_active_layer(Some(l));
            l
        }
    };
    let ne = plot_doc.doc.make_element(format!("{kind:?}"), geometry);
    let id = ne.id;
    let element = ne.element;
    let cmd = PlotCommand::AddElement {
        layer,
        element: Box::new(element),
    };
    cmd.apply(&mut plot_doc.doc);
    history.0.record(cmd);
    plot_doc.mark_dirty();
    selection.select_one(id);
    id
}

/// Pick the active camera for `mode` (projection-matching first, else any
/// active) and build its [`ViewMetrics`] + rotation — the same selection rule
/// the render / pick systems use so input, projection and drawing agree.
fn active_view<'q>(
    cams: &'q Query<(Entity, &Camera, &GlobalTransform, &Projection)>,
    ctx: &PlotViewCtx,
) -> Option<(Entity, &'q Camera, &'q GlobalTransform, Quat, ViewMetrics)> {
    let want_persp = matches!(ctx.mode, ViewMode::Globe);
    let mut exact: Option<(Entity, &Camera, &GlobalTransform, &Projection)> = None;
    let mut fallback: Option<(Entity, &Camera, &GlobalTransform, &Projection)> = None;
    for (e, c, ct, p) in cams.iter() {
        if !c.is_active {
            continue;
        }
        let is_persp = matches!(p, Projection::Perspective(_));
        if is_persp == want_persp {
            exact = Some((e, c, ct, p));
            break;
        }
        if fallback.is_none() {
            fallback = Some((e, c, ct, p));
        }
    }
    let (e, cam, ct, proj) = exact.or(fallback)?;
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
    Some((e, cam, ct, ct.rotation(), metrics))
}

/// The draw FSM system (see module docs).
#[allow(clippy::too_many_arguments)]
pub fn interaction_system(
    ctx: Res<PlotViewCtx>,
    mut plot_doc: ResMut<PlotDocument>,
    mut history: ResMut<PlotHistory>,
    mut interaction: ResMut<PlotInteraction>,
    mut capture: ResMut<PlotInputCapture>,
    mut selection: ResMut<PlotSelection>,
    snap_res: Res<PlotSnap>,
    mut tool_events: EventReader<PlotSetTool>,
    mut done_events: EventWriter<PlotDrawFinished>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cams: Query<(Entity, &Camera, &GlobalTransform, &Projection)>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    // 1. Apply queued tool switches.
    for ev in tool_events.read() {
        match ev.0 {
            PlotTool::Idle => interaction.stop(),
            PlotTool::Draw(k) => interaction.begin(k),
        }
    }

    // 2. Cancel / undo-vertex hotkeys.
    if keys.just_pressed(KeyCode::Escape) && interaction.is_drawing() {
        interaction.cancel();
        done_events.send(PlotDrawFinished { committed: None });
    }
    if keys.just_pressed(KeyCode::Backspace) {
        interaction.backspace();
    }

    // 3. Capture the pointer for the whole draw so the camera stands down.
    capture.0 = interaction.is_drawing();
    if !interaction.is_drawing() {
        return;
    }

    let Some((_, cam, ct, _rot, _metrics)) = active_view(&cams, &ctx) else {
        return;
    };
    let cursor = windows.get_single().ok().and_then(|w| w.cursor_position());

    // We only reach here while drawing, so the tool is a `Draw(kind)`.
    let kind = match interaction.tool {
        PlotTool::Draw(k) => k,
        PlotTool::Idle => return,
    };

    // 4. Enter finishes an open-ended draw.
    if keys.just_pressed(KeyCode::Enter) {
        if let Some(g) = interaction.finish() {
            let id = commit_geometry(&mut plot_doc, &mut history, &mut selection, kind, g);
            done_events.send(PlotDrawFinished {
                committed: Some(id),
            });
        }
        return;
    }

    // 5. A left click drops / completes a vertex. The raw cursor coordinate is
    //    first folded through the (by-default-disabled) snap config so a vertex
    //    can latch onto a nearby grid / vertex / edge before it lands.
    if mouse.just_pressed(MouseButton::Left) {
        if let Some(pos) = cursor {
            if let Some(geo) = screen_to_geo(cam, ct, ctx.mode, pos) {
                let geo = snap(&plot_doc.doc, geo, &snap_res, None).point();
                if let Some(g) = interaction.add_point(geo) {
                    // A fixed-point kind auto-completed → commit it.
                    let id = commit_geometry(&mut plot_doc, &mut history, &mut selection, kind, g);
                    done_events.send(PlotDrawFinished {
                        committed: Some(id),
                    });
                }
            }
        }
    }
}

/// A preview of the in-progress draft: a rubber-band stroke through the placed
/// vertices plus a live segment to the cursor (plan §8 "实时预览"). Transient
/// entities carry [`PlotPreviewEntity`] and are rebuilt fresh every frame.
#[allow(clippy::too_many_arguments)]
pub fn draw_preview_system(
    mut commands: Commands,
    ctx: Res<PlotViewCtx>,
    interaction: Res<PlotInteraction>,
    old: Query<Entity, With<PlotPreviewEntity>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<bevy::pbr::StandardMaterial>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cams: Query<(Entity, &Camera, &GlobalTransform, &Projection)>,
) {
    // Drop the previous frame's preview.
    for e in old.iter() {
        commands.entity(e).despawn();
    }

    if !interaction.is_drawing() || interaction.draft.is_empty() {
        return;
    }
    let Some((_, cam, ct, rot, metrics)) = active_view(&cams, &ctx) else {
        return;
    };

    // Draft vertices + the rubber-band tail to the live cursor.
    let mut geos: Vec<GeoPoint> = interaction.draft.clone();
    if let Some(pos) = windows.get_single().ok().and_then(|w| w.cursor_position()) {
        if let Some(g) = screen_to_geo(cam, ct, ctx.mode, pos) {
            geos.push(g);
        }
    }

    // A preview stroke through the chain (rubber band).
    if geos.len() >= 2 {
        let world: Vec<Vec3> = geos.iter().map(|g| metrics.project(*g)).collect();
        let normal = rot * Vec3::Z;
        let (pos, idx) = ribbon(&world, &|i| line_half_width(&metrics, world[i], 2.0), normal);
        if pos.is_empty() {
            return;
        }
        let n = pos.len();
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; n]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; n]);
        mesh.insert_indices(Indices::U32(idx));
        let mat = materials.add(preview_material());
        commands.spawn((
            PlotPreviewEntity,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(mat),
            RenderLayers::layer(OVERLAY_LAYER),
            Visibility::Visible,
            Transform::IDENTITY,
        ));
    }
}

/// Marker for the transient preview entities (rebuilt every frame).
#[derive(Component)]
pub struct PlotPreviewEntity;

/// The preview stroke colour (semi-opaque white, double-sided, unlit).
fn preview_material() -> bevy::pbr::StandardMaterial {
    bevy::pbr::StandardMaterial {
        base_color: Color::srgba(1.0, 1.0, 1.0, 0.8),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_plot::model::Document;
    use cesium_plot::ops::SnapConfig;

    fn p(lon: f64, lat: f64) -> GeoPoint {
        GeoPoint::surface(lon, lat)
    }

    #[test]
    fn snap_is_off_by_default_so_draws_are_unaffected() {
        // Baseline guard: the bridge's snap resource defaults to disabled, and a
        // disabled config returns the raw cursor coordinate untouched (M9).
        let snap_cfg = PlotSnap::default();
        assert!(!snap_cfg.enabled, "PlotSnap must default to off");
        let doc = Document::default();
        let raw = p(1.234, 5.678);
        let got = snap(&doc, raw, &snap_cfg, None);
        assert_eq!(got.point(), raw, "disabled snap never moves the vertex");
    }

    #[test]
    fn snapped_vertex_lands_identically_in_2d_and_3d() {
        // M9 both-mode proof: snapping is a geographic fold, so a snapped vertex
        // coincides with its target in *both* projections. Snap a raw cursor near
        // an existing vertex, then check it projects to the same world position as
        // the target under the Flat (2D) and Globe (3D) metrics independently.
        let mut doc = Document::default();
        let layer = doc.new_layer("L");
        let target = p(10.0, 20.0);
        let anchor = doc.make_element("a", Geometry::Point(target));
        doc.add_element_to_layer(layer, anchor);

        let cfg = SnapConfig {
            enabled: true,
            vertex_threshold_m: 200_000.0, // generous so the ~6 km offset latches
            ..Default::default()
        };
        let raw = p(10.05, 20.05);
        let snapped = snap(&doc, raw, &cfg, None).point();
        assert_eq!(snapped, target, "snapped exactly onto the existing vertex");

        let flat = ViewMetrics {
            mode: ViewMode::Flat,
            pixels_per_world: 100.0,
            focal_px: 0.0,
            cam_pos: Vec3::ZERO,
        };
        let globe = ViewMetrics {
            mode: ViewMode::Globe,
            pixels_per_world: 0.0,
            focal_px: 600.0,
            cam_pos: Vec3::new(3.0, 0.0, 0.0),
        };
        // Same geographic truth ⇒ identical render position, in either mode.
        assert!(flat.project(snapped).distance(flat.project(target)) < 1e-9);
        assert!(globe.project(snapped).distance(globe.project(target)) < 1e-9);
    }

    #[test]
    fn point_auto_completes_on_one_click() {
        let mut fs = PlotInteraction::default();
        fs.begin(DrawKind::Point);
        let g = fs.add_point(p(1.0, 2.0)).expect("commits");
        assert!(matches!(g, Geometry::Point(_)));
        assert!(!fs.is_drawing());
    }

    #[test]
    fn rectangle_completes_on_two_clicks_and_orders_them() {
        let mut fs = PlotInteraction::default();
        fs.begin(DrawKind::Rectangle);
        assert!(fs.add_point(p(10.0, 20.0)).is_none(), "one click still drawing");
        assert!(fs.is_drawing());
        let g = fs.add_point(p(-5.0, 3.0)).expect("second click commits");
        match g {
            Geometry::Rectangle(r) => assert_eq!((r.west, r.south, r.east, r.north), (-5.0, 3.0, 10.0, 20.0)),
            other => panic!("{other:?}"),
        }
        assert!(!fs.is_drawing());
    }

    #[test]
    fn polyline_finishes_on_enter_not_clicks() {
        let mut fs = PlotInteraction::default();
        fs.begin(DrawKind::Polyline);
        fs.add_point(p(0.0, 0.0));
        fs.add_point(p(1.0, 1.0));
        fs.add_point(p(2.0, 0.0));
        assert!(fs.is_drawing(), "polyline stays open");
        let g = fs.finish().expect("enter commits");
        match g {
            Geometry::Polyline(pl) => assert_eq!(pl.positions.len(), 3),
            other => panic!("{other:?}"),
        }
        assert!(!fs.is_drawing());
    }

    #[test]
    fn finish_needs_the_minimum_so_short_polyline_stays_open() {
        let mut fs = PlotInteraction::default();
        fs.begin(DrawKind::Polyline);
        fs.add_point(p(0.0, 0.0));
        assert!(fs.finish().is_none(), "one point is not a polyline");
        assert!(fs.is_drawing(), "still drawing");
    }

    #[test]
    fn backspace_drops_the_last_vertex() {
        let mut fs = PlotInteraction::default();
        fs.begin(DrawKind::Polygon);
        fs.add_point(p(0.0, 0.0));
        fs.add_point(p(1.0, 0.0));
        fs.backspace();
        assert_eq!(fs.draft.len(), 1);
    }

    #[test]
    fn cancel_clears_draft_and_tool() {
        let mut fs = PlotInteraction::default();
        fs.begin(DrawKind::Polygon);
        fs.add_point(p(0.0, 0.0));
        fs.cancel();
        assert!(!fs.is_drawing());
        assert!(fs.draft.is_empty());
    }

    #[test]
    fn commit_adds_to_document_and_selects() {
        let mut plot_doc = PlotDocument {
            doc: Document::default(),
            revision: 0,
            dirty: true,
        };
        let mut history = PlotHistory::default();
        let mut selection = PlotSelection::default();
        let g = commit_draft(DrawKind::Polyline, &[p(0.0, 0.0), p(1.0, 1.0)]).unwrap();
        let id = commit_geometry(&mut plot_doc, &mut history, &mut selection, DrawKind::Polyline, g);
        assert_eq!(plot_doc.doc.element_count(), 1);
        assert!(plot_doc.doc.active_layer().is_some(), "layer auto-created");
        assert!(selection.contains(id), "committed element is selected");
        assert!(plot_doc.dirty);
        // The add was recorded so it can be undone (M6).
        assert!(history.0.can_undo());
        history.0.undo(&mut plot_doc.doc);
        assert_eq!(plot_doc.doc.element_count(), 0);
    }
}
