//! The picking subsystem (plan §7): turn a screen cursor into the single best
//! [`PickHit`] under it, drive hover feedback and click-to-select, and emit the
//! hover / selection events the interaction FSM and UI consume.
//!
//! The logic splits in two so it is both correct and testable:
//!  * [`pick_at`] is a **pure query** — given the document, the current pickable
//!    id set and *any* geographic → screen projector, it projects each candidate
//!    geometry, runs the [`cesium_plot::geom::hit`] primitives and folds the
//!    results through [`pick_best`]. It has no Bevy dependency, so the whole hit
//!    / rank / select contract is unit-testable headless (plan §15).
//!  * [`pick_system`] is the thin ECS shell: it gathers the active camera's
//!    metrics, builds the projector out of [`crate::labels::world_to_screen`] —
//!    the *same* call the renderer uses to place meshes and labels, so "what you
//!    pick" is by construction "what you see" in 2D and 3D — and then applies the
//!    hover / click state changes.

use std::collections::BTreeSet;

use bevy::input::mouse::MouseButton;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use cesium_plot::geo::{GeoBounds, GeoPoint};
use cesium_plot::geom::hit;
use cesium_plot::model::geometry::Geometry;
use cesium_plot::model::ids::ElementId;
use cesium_plot::model::{
    pick_best, Part, PickHit, Style, ViewContext, ViewMode, Document, RANK_POLY_BODY,
    RANK_POLY_EDGE,
};
use cesium_plot::visibility::eval_visibility;

use crate::labels::world_to_screen;
use crate::reproject::ViewMetrics;
use crate::resources::{PlotDocument, PlotFilters, PlotHover, PlotSelection, PlotViewCtx};
use crate::shapes;

/// Weight of a layer's `order` in the composite draw / pick priority (a layer
/// beats any intra-layer `z_order` differences below it).
const LAYER_WEIGHT: i32 = 100_000;

/// A cheap equality key capturing every input that determines [`pick_at`]'s
/// result for a frame. Two consecutive frames sharing a key produce an identical
/// hover, so the expensive per-element projection / tessellation is skipped and
/// the cached [`PickHit`] is reused. It deliberately includes the resolved
/// `pickable` set (the only way a `Filters` edit surfaces, since `Filters` has no
/// revision counter) and the camera pose / projection the projector closes over.
#[derive(Clone, PartialEq)]
pub struct PickKey {
    cursor: [f64; 2],
    revision: u64,
    globe: bool,
    flat_zoom: f32,
    screen: [f32; 2],
    cam_t: [f32; 3],
    cam_r: [f32; 4],
    focal_px: f64,
    persp: bool,
    pickable: Vec<ElementId>,
}

/// Cache for [`pick_system`]: the last hover result and the [`PickKey`] it was
/// computed under. Windowed-only (lives with the bridge plugin), so the headless
/// golden baseline is untouched.
#[derive(Resource, Default)]
pub struct PlotPickCache {
    key: Option<PickKey>,
    best: Option<PickHit>,
}

/// Emitted whenever the hovered element changes (plan §14).
#[derive(Event, Clone, Copy, Debug, PartialEq)]
pub struct PlotHoverChanged {
    /// The new hit under the cursor, if any.
    pub hit: Option<PickHit>,
}

/// Emitted whenever the selection set changes (plan §14).
#[derive(Event, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlotSelectionChanged {
    /// Size of the new selection.
    pub selected: u32,
}

/// Emitted on a right-click (plan §16 M9 "右键菜单"): the UI / app owns the
/// actual menu, this only reports *where* it was requested and *what* (if
/// anything) sat under the cursor, so a menu can offer context actions on the
/// picked element. Nothing else changes — the bridge just raises the intent.
#[derive(Event, Clone, Copy, Debug, PartialEq)]
pub struct PlotContextMenu {
    /// The element under the cursor, if the right-click hit one.
    pub target: Option<ElementId>,
    /// Screen position (logical pixels) to anchor the menu at.
    pub screen: Vec2,
}

/// Project a geographic point into screen pixels through the active camera, or
/// `None` when it fails to project (behind the camera, off-viewport, …).
type Projector<'a> = dyn Fn(GeoPoint) -> Option<[f64; 2]> + 'a;

/// Hit one geometry against `cursor`, returning `(part, screen_dist, rank)`.
/// Only the M2/M3 drawable kinds are tested; faces / conics gain a filled
/// interior hit at M4 once they are sampled.
fn hit_geometry(
    geo: &Geometry,
    style: &Style,
    mode: ViewMode,
    project: &Projector<'_>,
    cursor: [f64; 2],
) -> Option<(Part, f64, u8)> {
    match geo {
        Geometry::Point(p) => {
            let sp = project(*p)?;
            let r = (style.point_size_px as f64 * 0.5).max(hit::DEFAULT_TOL_PX);
            let (part, d) = hit::hit_point(cursor, sp, r)?;
            Some((part, d, cesium_plot::model::RANK_MARKER))
        }
        Geometry::Icon(i) => {
            let sp = project(i.at)?;
            let size = style.icon.map(|x| x.size_px as f64).unwrap_or(32.0);
            let r = (size * 0.5).max(hit::DEFAULT_TOL_PX);
            let (part, d) = hit::hit_point(cursor, sp, r)?;
            Some((part, d, cesium_plot::model::RANK_MARKER))
        }
        Geometry::Label(l) => {
            let sp = project(l.at)?;
            // A generous text-box proxy: a fixed radius around the anchor.
            let r = hit::DEFAULT_TOL_PX.max(10.0);
            let (part, d) = hit::hit_point(cursor, sp, r)?;
            Some((part, d, cesium_plot::model::RANK_MARKER))
        }
        Geometry::Polyline(_) | Geometry::Arc(_) | Geometry::Path(_) => {
            // Sampled / great-circle-densified stroke, identical to what renders
            // (via [`crate::shapes`]). Require every vertex to project so part
            // indices stay aligned with the geometry's real vertices.
            let pos = shapes::stroke_positions(geo, mode)?;
            let mut pts: Vec<[f64; 2]> = Vec::with_capacity(pos.len());
            for g in &pos {
                pts.push(project(*g)?);
            }
            let tol = style.width_px as f64 * 0.5 + hit::DEFAULT_TOL_PX;
            let (part, d) = hit::hit_polyline(cursor, &pts, tol)?;
            Some((part, d, cesium_plot::model::RANK_LINE))
        }
        Geometry::Polygon(_) | Geometry::Rectangle(_) | Geometry::Circle(_)
        | Geometry::Ellipse(_) => {
            let (outer, holes) = shapes::face_rings(geo, mode)?;
            let mut o: Vec<[f64; 2]> = Vec::with_capacity(outer.len());
            for g in &outer {
                o.push(project(*g)?);
            }
            let mut hs: Vec<Vec<[f64; 2]>> = Vec::with_capacity(holes.len());
            for h in &holes {
                let mut ring = Vec::with_capacity(h.len());
                for g in h {
                    ring.push(project(*g)?);
                }
                hs.push(ring);
            }
            let tol = style.width_px as f64 * 0.5 + hit::DEFAULT_TOL_PX;
            let (part, d) = hit::hit_polygon_multi(cursor, &o, &hs, tol)?;
            let rank = if part == Part::Body {
                RANK_POLY_BODY
            } else {
                RANK_POLY_EDGE
            };
            Some((part, d, rank))
        }
        _ => None, // point/icon/label handled above; composite reserved for M9
    }
}

/// Whether a geometry's narrow-phase hit is expensive enough (it projects a
/// sampled / densified vertex chain) to be worth a broad-phase reject first.
/// Point-like kinds project a single point and skip the broad phase.
fn is_multi_vertex(geo: &Geometry) -> bool {
    matches!(
        geo,
        Geometry::Polyline(_)
            | Geometry::Polygon(_)
            | Geometry::Rectangle(_)
            | Geometry::Circle(_)
            | Geometry::Ellipse(_)
            | Geometry::Arc(_)
            | Geometry::Path(_)
    )
}

/// Broad-phase screen test. Projects the element's conservative geographic
/// `bounds` on a 4×4 grid into a screen AABB and returns `false` only when the
/// cursor is provably outside it (padded by `tol` px). It is never a false
/// reject: an empty box, or any grid sample that fails to project (geometry off
/// or behind the camera), is conservatively reported as a possible hit. The
/// bounds come from [`cesium_plot::model::Geometry::bounds`], which is the
/// globe-densified extent, so it also covers the great-circle bulge.
fn bounds_might_hit(
    bounds: &GeoBounds,
    project: &Projector<'_>,
    cursor: [f64; 2],
    tol: f64,
) -> bool {
    if bounds.is_empty() {
        return true;
    }
    const FRACS: [f64; 4] = [0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0];
    let mut min = [f64::INFINITY; 2];
    let mut max = [f64::NEG_INFINITY; 2];
    for &fx in &FRACS {
        let lon = bounds.west_deg + (bounds.east_deg - bounds.west_deg) * fx;
        for &fy in &FRACS {
            let lat = bounds.south_deg + (bounds.north_deg - bounds.south_deg) * fy;
            match project(GeoPoint::surface(lon, lat)) {
                Some(sp) => {
                    min[0] = min[0].min(sp[0]);
                    min[1] = min[1].min(sp[1]);
                    max[0] = max[0].max(sp[0]);
                    max[1] = max[1].max(sp[1]);
                }
                // Cannot prove the element is off-screen → keep it.
                None => return true,
            }
        }
    }
    cursor[0] >= min[0] - tol
        && cursor[0] <= max[0] + tol
        && cursor[1] >= min[1] - tol
        && cursor[1] <= max[1] + tol
}

/// The pure pick query (plan §14). Returns the winning [`PickHit`] among the
/// `pickable` elements under `cursor`, or `None`. `project` maps a geographic
/// coordinate to screen pixels; the bridge supplies the camera-based one.
pub fn pick_at(
    doc: &Document,
    pickable: &BTreeSet<ElementId>,
    cursor: [f64; 2],
    mode: ViewMode,
    project: &Projector<'_>,
) -> Option<PickHit> {
    let mut candidates = Vec::new();
    for &id in pickable {
        let Some(element) = doc.element(id) else {
            continue;
        };
        let Some((layer_id, _)) = doc.element_context(id) else {
            continue;
        };
        let order = doc
            .layer(layer_id)
            .map(|l| l.order)
            .unwrap_or(0)
            .saturating_mul(LAYER_WEIGHT)
            + element.style.z_order;
        // Broad phase: cheap screen-AABB reject before the expensive per-vertex
        // projection / tessellation, and only for multi-vertex kinds (a point
        // already projects one coordinate, so culling it would cost more than it
        // saves). The conservative bounds guarantee this never drops a real hit.
        if is_multi_vertex(&element.geometry) {
            let tol = element.style.width_px as f64 * 0.5 + hit::DEFAULT_TOL_PX + 4.0;
            if !bounds_might_hit(&element.bounds, project, cursor, tol) {
                continue;
            }
        }
        if let Some((part, screen_dist, rank)) =
            hit_geometry(&element.geometry, &element.style, mode, project, cursor)
        {
            candidates.push(PickHit {
                element: id,
                part,
                layer: layer_id,
                z_order: order,
                rank,
                screen_dist,
            });
        }
    }
    pick_best(&candidates)
}

/// The per-frame pick system (see module docs).
#[allow(clippy::too_many_arguments)]
pub fn pick_system(
    ctx: Res<PlotViewCtx>,
    plot_doc: Res<PlotDocument>,
    filters: Res<PlotFilters>,
    mut cache: ResMut<PlotPickCache>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cams: Query<(&Camera, &GlobalTransform, &Projection)>,
    mouse: Res<ButtonInput<MouseButton>>,
    interaction: Res<crate::interaction::PlotInteraction>,
    mut hover: ResMut<PlotHover>,
    mut selection: ResMut<PlotSelection>,
    mut hover_events: EventWriter<PlotHoverChanged>,
    mut selection_events: EventWriter<PlotSelectionChanged>,
    mut menu_events: EventWriter<PlotContextMenu>,
) {
    // Cursor must be inside the window; a leaving cursor clears the hover.
    let cursor = match windows.get_single() {
        Ok(w) => w.cursor_position().map(|c| [c.x as f64, c.y as f64]),
        Err(_) => None,
    };
    let Some(cursor) = cursor else {
        if hover.0.is_some() {
            hover.0 = None;
            hover_events.send(PlotHoverChanged { hit: None });
        }
        return;
    };

    // Active camera (projection matching the view mode, else any active) — the
    // same selection rule [`crate::sync::sync_visuals`] uses, so pick and render
    // always agree on which camera defines the screen.
    let want_perspective = matches!(ctx.mode, cesium_plot::model::ViewMode::Globe);
    let mut exact: Option<(&Camera, &GlobalTransform, &Projection)> = None;
    let mut fallback: Option<(&Camera, &GlobalTransform, &Projection)> = None;
    for (c, ct, p) in cams.iter() {
        if !c.is_active {
            continue;
        }
        let is_persp = matches!(p, Projection::Perspective(_));
        let matches = if want_perspective { is_persp } else { !is_persp };
        if matches {
            exact = Some((c, ct, p));
            break;
        }
        if fallback.is_none() {
            fallback = Some((c, ct, p));
        }
    }
    let Some((cam, ct, proj)) = exact.or(fallback) else {
        return;
    };
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

    // Pickable set = visible ∧ selectable under the current filters (plan §7).
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
    let pickable = eval_visibility(&plot_doc.doc, &view, &filters.0).pickable;

    // Reuse the last hover whenever every input that feeds `pick_at` is
    // unchanged (cursor, document revision, camera pose / projection, viewport,
    // and the resolved pickable set). This is the big constant-factor win for
    // large scenes: a stationary cursor over a moving camera still re-picks, but
    // an idle frame skips the whole per-element projection / tessellation.
    let key = PickKey {
        cursor,
        revision: plot_doc.revision,
        globe: matches!(ctx.mode, cesium_plot::model::ViewMode::Globe),
        flat_zoom: ctx.flat_zoom,
        screen: [ctx.screen_w, ctx.screen_h],
        cam_t: ct.translation().to_array(),
        cam_r: ct.rotation().to_array(),
        focal_px,
        persp: matches!(proj, Projection::Perspective(_)),
        pickable: pickable.iter().copied().collect(),
    };
    let best = if cache.key.as_ref() == Some(&key) {
        cache.best
    } else {
        let project = |g: GeoPoint| -> Option<[f64; 2]> {
            world_to_screen(cam, ct, metrics.project(g)).map(|v| [v.x as f64, v.y as f64])
        };
        let b = pick_at(&plot_doc.doc, &pickable, cursor, ctx.mode, &project);
        cache.key = Some(key);
        cache.best = b;
        b
    };

    if best != hover.0 {
        hover.0 = best;
        hover_events.send(PlotHoverChanged { hit: best });
    }

    // A plain left click replaces the selection with the hit element, or clears
    // it when clicking empty space (ctrl / box multi-select lands at M5). While
    // a draw tool is active the click belongs to the FSM (add a vertex), so the
    // picker stands down entirely to avoid double-handling the same press.
    if interaction.is_drawing() {
        return;
    }
    if mouse.just_pressed(MouseButton::Left) {
        let changed = match best {
            Some(hit) => selection.select_one(hit.element),
            None => selection.clear(),
        };
        if changed {
            selection_events.send(PlotSelectionChanged {
                selected: selection.0.len() as u32,
            });
        }
    }

    // A right-click raises the context-menu intent (M9): report the hit (if any)
    // and the anchor position; the app decides what menu, if any, to show.
    if mouse.just_pressed(MouseButton::Right) {
        menu_events.send(PlotContextMenu {
            target: best.map(|h| h.element),
            screen: Vec2::new(cursor[0] as f32, cursor[1] as f32),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_plot::geo::GeoPoint;
    use cesium_plot::model::geometry::{LabelGeometry, Polyline, Rectangle};
    use cesium_plot::model::ids::ElementId;
    use cesium_plot::model::{Document, Geometry, LabelAnchor};

    /// A projector that maps lon/lat degrees straight to pixels (× 4) so tests
    /// can reason about screen positions without any camera.
    fn quad_projector(g: GeoPoint) -> Option<[f64; 2]> {
        Some([g.lon_deg * 4.0, g.lat_deg * 4.0])
    }

    fn point_doc() -> (Document, ElementId, ElementId) {
        let mut doc = Document::default();
        let layer = doc.new_layer("L");
        let a = doc.make_element("a", Geometry::Point(GeoPoint::surface(0.0, 0.0)));
        let b = doc.make_element("b", Geometry::Point(GeoPoint::surface(50.0, 50.0)));
        let ea = a.id;
        let eb = b.id;
        doc.add_element_to_layer(layer, a);
        doc.add_element_to_layer(layer, b);
        (doc, ea, eb)
    }

    #[test]
    fn pick_at_misses_when_cursor_is_off_every_element() {
        let (doc, _, _) = point_doc();
        let pickable: BTreeSet<ElementId> = doc.elements().map(|e| e.id).collect();
        // Far from both projected points.
        assert!(pick_at(&doc, &pickable, [500.0, 500.0], ViewMode::Flat, &quad_projector).is_none());
    }

    #[test]
    fn globe_pick_follows_the_great_circle_not_the_flat_chord() {
        // M9 both-mode proof for picking: a long east-west line at 60°N has a
        // great circle that bulges poleward. A point on that bulge is pickable in
        // Globe mode (the stroke is densified along the sphere) yet missed in Flat
        // mode (the straight 2D chord passes well south of it). This exercises the
        // mode-aware sampling shared with the renderer.
        let mut doc = Document::default();
        let layer = doc.new_layer("L");
        let line = doc.make_element(
            "route",
            Geometry::Polyline(Polyline {
                positions: vec![GeoPoint::surface(0.0, 60.0), GeoPoint::surface(60.0, 60.0)],
            }),
        );
        let id = line.id;
        doc.add_element_to_layer(layer, line);
        let pickable: BTreeSet<ElementId> = [id].into_iter().collect();

        // The most-poleward densified Globe sample (the great-circle apex).
        let globe_pts =
            shapes::stroke_positions(&doc.element(id).unwrap().geometry, ViewMode::Globe).unwrap();
        let bulge = globe_pts
            .iter()
            .fold(globe_pts[0], |a, &b| if b.lat_deg > a.lat_deg { b } else { a });
        assert!(bulge.lat_deg > 61.5, "expected a clear poleward bulge, got {bulge:?}");
        let cursor = [bulge.lon_deg * 4.0, bulge.lat_deg * 4.0];

        assert!(
            pick_at(&doc, &pickable, cursor, ViewMode::Globe, &quad_projector).is_some(),
            "Globe pick must hit the great-circle sample"
        );
        assert!(
            pick_at(&doc, &pickable, cursor, ViewMode::Flat, &quad_projector).is_none(),
            "Flat chord must miss the bulge"
        );
    }

    #[test]
    fn pick_at_hits_the_nearest_point() {
        let (doc, ea, _eb) = point_doc();
        let pickable: BTreeSet<ElementId> = doc.elements().map(|e| e.id).collect();
        // Cursor right on point a's projected pixel (0, 0).
        let hit = pick_at(&doc, &pickable, [1.0, -1.0], ViewMode::Flat, &quad_projector).unwrap();
        assert_eq!(hit.element, ea);
        assert_eq!(hit.rank, cesium_plot::model::RANK_MARKER);
    }

    #[test]
    fn marker_outranks_a_line_crossing_the_same_pixel() {
        // A marker at (0,0) and a line through (0,0) → the marker (rank 0) wins
        // over the line stroke (rank 1) even though the cursor is on both.
        let mut doc = Document::default();
        let layer = doc.new_layer("L");
        let marker = doc.make_element("m", Geometry::Point(GeoPoint::surface(0.0, 0.0)));
        let line = doc.make_element(
            "l",
            Geometry::Polyline(Polyline {
                positions: vec![
                    GeoPoint::surface(-100.0, 0.0),
                    GeoPoint::surface(100.0, 0.0),
                ],
            }),
        );
        let mid = marker.id;
        doc.add_element_to_layer(layer, marker);
        doc.add_element_to_layer(layer, line);
        let pickable: BTreeSet<ElementId> = doc.elements().map(|e| e.id).collect();
        // Cursor at the shared (0,0) screen pixel.
        let hit = pick_at(&doc, &pickable, [0.0, 0.0], ViewMode::Flat, &quad_projector).unwrap();
        assert_eq!(hit.element, mid, "marker beats line");
        assert_eq!(hit.rank, cesium_plot::model::RANK_MARKER);
    }

    #[test]
    fn higher_z_order_wins_within_the_same_rank() {
        let mut doc = Document::default();
        let layer = doc.new_layer("L");
        let mut low = doc.make_element("low", Geometry::Point(GeoPoint::surface(0.0, 0.0)));
        low.element.style.z_order = 1;
        let mut high = doc.make_element("high", Geometry::Point(GeoPoint::surface(0.0, 0.0)));
        high.element.style.z_order = 5;
        let low_id = low.id;
        let high_id = high.id;
        doc.add_element_to_layer(layer, low);
        doc.add_element_to_layer(layer, high);
        let _ = low_id;
        let pickable: BTreeSet<ElementId> = doc.elements().map(|e| e.id).collect();
        let hit = pick_at(&doc, &pickable, [0.0, 0.0], ViewMode::Flat, &quad_projector).unwrap();
        assert_eq!(hit.element, high_id, "z_order 5 beats 1");
    }

    #[test]
    fn only_the_pickable_set_is_considered() {
        let (doc, _ea, eb) = point_doc();
        // Only `eb` in the pickable set → a cursor on `ea` yields nothing.
        let pickable: BTreeSet<ElementId> = [eb].into_iter().collect();
        assert!(pick_at(&doc, &pickable, [0.0, 0.0], ViewMode::Flat, &quad_projector).is_none());
        let hit = pick_at(&doc, &pickable, [200.0, 200.0], ViewMode::Flat, &quad_projector).unwrap();
        assert_eq!(hit.element, eb);
    }

    #[test]
    fn polyline_reports_vertex_then_edge() {
        let mut doc = Document::default();
        let layer = doc.new_layer("L");
        let line = doc.make_element(
            "l",
            Geometry::Polyline(Polyline {
                positions: vec![
                    GeoPoint::surface(0.0, 0.0),
                    GeoPoint::surface(10.0, 0.0),
                    GeoPoint::surface(20.0, 0.0),
                ],
            }),
        );
        doc.add_element_to_layer(layer, line);
        let pickable: BTreeSet<ElementId> = doc.elements().map(|e| e.id).collect();
        // Near vertex 1 (screen 40,0) → a Vertex part with index 1.
        let hit = pick_at(&doc, &pickable, [40.0, 0.0], ViewMode::Flat, &quad_projector).unwrap();
        assert_eq!(hit.part, Part::Vertex(1));
        // Along edge 0, away from vertices (screen 20,1) → Edge(0).
        let hit = pick_at(&doc, &pickable, [20.0, 1.0], ViewMode::Flat, &quad_projector).unwrap();
        assert_eq!(hit.part, Part::Edge(0));
    }

    #[test]
    fn label_anchor_is_pickable() {
        let mut doc = Document::default();
        let layer = doc.new_layer("L");
        let lbl = doc.make_element(
            "lbl",
            Geometry::Label(LabelGeometry {
                at: GeoPoint::surface(5.0, 5.0),
                text: "hi".into(),
                anchor: LabelAnchor::Center,
                offset_px: [0.0, 0.0],
            }),
        );
        doc.add_element_to_layer(layer, lbl);
        let pickable: BTreeSet<ElementId> = doc.elements().map(|e| e.id).collect();
        // (5,5) → screen (20,20).
        let hit = pick_at(&doc, &pickable, [20.0, 20.0], ViewMode::Flat, &quad_projector).unwrap();
        assert_eq!(hit.rank, cesium_plot::model::RANK_MARKER);
    }

    #[test]
    fn polygon_interior_beats_nothing_and_edge_is_a_ring() {
        // A rectangle (0,0)-(10,10) → screen (0,0)-(40,40). Cursor in the middle
        // hits the *body*; cursor on the top edge hits an *edge* (boundary).
        let mut doc = Document::default();
        let layer = doc.new_layer("L");
        let rect = doc.make_element(
            "r",
            Geometry::Rectangle(Rectangle {
                west: 0.0,
                south: 0.0,
                east: 10.0,
                north: 10.0,
            }),
        );
        doc.add_element_to_layer(layer, rect);
        let pickable: BTreeSet<ElementId> = doc.elements().map(|e| e.id).collect();
        let body = pick_at(&doc, &pickable, [20.0, 20.0], ViewMode::Flat, &quad_projector).unwrap();
        assert_eq!(body.part, Part::Body);
        assert_eq!(body.rank, RANK_POLY_BODY);
        let edge = pick_at(&doc, &pickable, [20.0, 40.0], ViewMode::Flat, &quad_projector).unwrap();
        assert!(matches!(edge.part, Part::Edge(_)), "{:#?}", edge.part);
        assert_eq!(edge.rank, RANK_POLY_EDGE);
    }

    #[test]
    fn bounds_broad_phase_includes_overlaps_and_unknowns() {
        let b = GeoBounds::from_points(&[
            GeoPoint::surface(0.0, 0.0),
            GeoPoint::surface(10.0, 10.0),
        ]);
        // quad_projector maps the box to screen (0,0)-(40,40).
        assert!(bounds_might_hit(&b, &quad_projector, [20.0, 20.0], 6.0)); // interior
        assert!(bounds_might_hit(&b, &quad_projector, [46.0, 46.0], 6.0)); // within slack
        assert!(!bounds_might_hit(&b, &quad_projector, [70.0, 20.0], 6.0)); // clearly outside
        // An empty box never culls.
        assert!(bounds_might_hit(&GeoBounds::empty(), &quad_projector, [9999.0, 9999.0], 0.0));
        // A projector that cannot project some sample keeps the element (no false reject).
        let flaky = |g: GeoPoint| -> Option<[f64; 2]> {
            if g.lat_deg > 3.0 {
                None
            } else {
                Some([g.lon_deg * 4.0, g.lat_deg * 4.0])
            }
        };
        assert!(bounds_might_hit(&b, &flaky, [9999.0, 9999.0], 0.0));
    }

    #[test]
    fn broad_phase_does_not_drop_an_interior_face_hit() {
        // A rectangle spanning the cursor: the body hit sits far from every
        // control corner, yet the grid-sampled projected box still contains it.
        let mut doc = Document::default();
        let layer = doc.new_layer("L");
        let rect = doc.make_element(
            "r",
            Geometry::Rectangle(Rectangle {
                west: -50.0,
                south: -50.0,
                east: 50.0,
                north: 50.0,
            }),
        );
        doc.add_element_to_layer(layer, rect);
        let pickable: BTreeSet<ElementId> = doc.elements().map(|e| e.id).collect();
        let hit = pick_at(&doc, &pickable, [0.0, 0.0], ViewMode::Flat, &quad_projector).unwrap();
        assert_eq!(hit.part, Part::Body);
    }
}
