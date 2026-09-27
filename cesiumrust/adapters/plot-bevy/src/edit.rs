//! The edit bridge (plan §8, M6): turns selection + keyboard / drag gestures
//! into reversible [`PlotCommand`]s, applies them to the [`PlotDocument`] and
//! records them on the [`PlotHistory`] stack so every edit is undoable.
//!
//! The command builders ([`delete_commands`], [`duplicate_commands`],
//! [`translate_commands`]) are *pure over the document* — no ECS, no window —
//! so the whole move / duplicate / delete / nudge contract is unit-testable
//! headless, exactly like the core ops. The two systems are thin shells:
//!  * [`edit_system`] drives them from the keyboard (Delete / Ctrl+D / Ctrl+Z /
//!    Ctrl+Y / arrow nudge);
//!  * [`drag_move_system`] folds a pointer drag over an already-selected element
//!    into a single translate command (one drag == one undo step, plan §8).
//!
//! Editing is inert while a draw tool is active (the draw FSM owns the keys) and
//! while headless (no camera / window), so the golden baseline never sees these.

use bevy::input::mouse::MouseButton;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use cesium_plot::geo::GeoPoint;
use cesium_plot::model::geometry::Geometry;
use cesium_plot::model::ids::ElementId;
use cesium_plot::model::{Document, Style, ViewMode};
use cesium_plot::ops::{transform, PlotCommand};

use crate::interaction::{screen_to_geo, PlotInteraction};
use crate::picking::PlotSelectionChanged;
use crate::resources::{
    PlotDocument, PlotHistory, PlotHover, PlotInputCapture, PlotSelection, PlotViewCtx,
};

/// Screen distance (logical pixels) the pointer must travel before a press turns
/// into a drag (so a plain click on a selected element does not nudge it).
const DRAG_THRESHOLD_PX: f32 = 4.0;
/// A single arrow-key nudge, in degrees of lon/lat (a coarse but view-independent
/// step; fine positioning uses the drag).
const NUDGE_DEG: f64 = 0.05;

/// Build a remove command for every selected id that still lives in a layer.
pub fn delete_commands(doc: &Document, ids: impl IntoIterator<Item = ElementId>) -> PlotCommand {
    let steps: Vec<PlotCommand> = ids
        .into_iter()
        .filter_map(|id| {
            let element = doc.element(id)?.clone();
            let (layer, _) = doc.element_context(id)?;
            Some(PlotCommand::RemoveElement {
                element: Box::new(element),
                layer,
            })
        })
        .collect();
    PlotCommand::Composite { steps }
}

/// Build an add command per selected element, cloning its geometry / style /
/// attributes into a freshly allocated id in the same layer. Names get a copy
/// suffix so duplicated elements are distinguishable in the panel (M7).
pub fn duplicate_commands(doc: &mut Document, ids: &[ElementId]) -> PlotCommand {
    let mut steps = Vec::new();
    for &id in ids {
        // Snapshot the source before allocating so `make_element`'s borrow ends.
        let src = match doc.element(id) {
            Some(e) => e.clone(),
            None => continue,
        };
        let layer = match doc.element_context(id) {
            Some(c) => c.0,
            None => continue,
        };
        let mut ne = doc.make_element(format!("{} 副本", src.name), src.geometry);
        ne.element.style = src.style;
        ne.element.attributes = src.attributes;
        ne.element.flags = src.flags;
        ne.element.scale_visibility = src.scale_visibility;
        ne.element.time_window = src.time_window;
        steps.push(PlotCommand::AddElement {
            layer,
            element: Box::new(ne.element),
        });
    }
    PlotCommand::Composite { steps }
}

/// Build a move command: translate every editable selected element by
/// `(dlon, dlat)` degrees, folding them into one composite (one undo step).
pub fn translate_commands(doc: &Document, ids: &[ElementId], dlon: f64, dlat: f64) -> PlotCommand {
    let steps: Vec<PlotCommand> = ids
        .iter()
        .filter_map(|&id| {
            let e = doc.element(id)?;
            if !e.flags.editable {
                return None;
            }
            let before = e.geometry.clone();
            let after = transform::translate(&before, dlon, dlat);
            Some(PlotCommand::UpdateGeometry {
                id,
                before: Box::new(before),
                after: Box::new(after),
            })
        })
        .collect();
    PlotCommand::Composite { steps }
}

/// A pointer-drag move of the current selection (plan §8 "拖拽选中集"). The
/// pressed element must already be selected + editable; the move is applied live
/// from the stored `before` geometries and folded into one command on release.
#[derive(Resource, Default)]
pub struct PlotDrag {
    /// A press grabbed a selection (not yet past the movement threshold).
    active: bool,
    /// The pointer has crossed [`DRAG_THRESHOLD_PX`] — the move is now live.
    armed: bool,
    /// Screen position at press (for the threshold test).
    start_px: Vec2,
    /// Geographic coordinate under the cursor when the move armed.
    start_geo: Option<GeoPoint>,
    /// The elements being moved.
    moving: Vec<ElementId>,
    /// Their geometry captured at arm time (the undo `before`).
    before: Vec<(ElementId, Geometry)>,
}

/// Apply a command to the document and record it (skipping empty composites).
fn commit(plot_doc: &mut PlotDocument, history: &mut PlotHistory, command: PlotCommand) {
    if matches!(&command, PlotCommand::Composite { steps } if steps.is_empty()) {
        return;
    }
    command.apply(&mut plot_doc.doc);
    history.0.record(command);
    plot_doc.mark_dirty();
}

/// Public entry point to the commit path so the M7 panels can route style and
/// geometry edits through the same undo-recording pipeline the keyboard / drag
/// systems use (one panel edit == one undo step).
pub fn apply_command(plot_doc: &mut PlotDocument, history: &mut PlotHistory, command: PlotCommand) {
    commit(plot_doc, history, command);
}

/// Build a style-edit command for every selected element whose style actually
/// changes under `mutate`, capturing before/after so the edit is reversible.
/// Elements already carrying the target style are skipped, so a no-op produces
/// an empty composite (which [`apply_command`] drops without recording).
pub fn style_command(
    doc: &Document,
    ids: &[ElementId],
    mutate: &dyn Fn(&Style) -> Style,
) -> PlotCommand {
    let steps: Vec<PlotCommand> = ids
        .iter()
        .filter_map(|id| {
            let element = doc.element(*id)?;
            let before = element.style.clone();
            let after = mutate(&before);
            if before == after {
                return None;
            }
            Some(PlotCommand::SetStyle {
                id: *id,
                before: Box::new(before),
                after: Box::new(after),
            })
        })
        .collect();
    PlotCommand::Composite { steps }
}

/// Drop selection ids that no longer exist (after an undo of an add).
fn prune_selection(
    selection: &mut PlotSelection,
    doc: &Document,
    events: &mut EventWriter<PlotSelectionChanged>,
) {
    let before = selection.0.len();
    selection.0.retain(|id| doc.element(*id).is_some());
    if selection.0.len() != before {
        events.send(PlotSelectionChanged {
            selected: selection.0.len() as u32,
        });
    }
}

/// The keyboard edit system (see module docs).
pub fn edit_system(
    mut plot_doc: ResMut<PlotDocument>,
    mut history: ResMut<PlotHistory>,
    mut selection: ResMut<PlotSelection>,
    interaction: Res<PlotInteraction>,
    keys: Res<ButtonInput<KeyCode>>,
    mut selection_events: EventWriter<PlotSelectionChanged>,
) {
    // Editing hotkeys belong to the draw FSM while a draw is active.
    if interaction.is_drawing() {
        return;
    }
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    // Undo / redo first (they act even with an empty selection).
    if ctrl && keys.just_pressed(KeyCode::KeyZ) {
        let moved = if shift {
            history.0.redo(&mut plot_doc.doc)
        } else {
            history.0.undo(&mut plot_doc.doc)
        };
        if moved.is_some() {
            plot_doc.mark_dirty();
            prune_selection(&mut selection, &plot_doc.doc, &mut selection_events);
        }
        return;
    }
    if ctrl && keys.just_pressed(KeyCode::KeyY) {
        if history.0.redo(&mut plot_doc.doc).is_some() {
            plot_doc.mark_dirty();
            prune_selection(&mut selection, &plot_doc.doc, &mut selection_events);
        }
        return;
    }

    let ids: Vec<ElementId> = selection.0.iter().copied().collect();
    if ids.is_empty() {
        return;
    }

    // Delete.
    if keys.just_pressed(KeyCode::Delete) {
        let command = delete_commands(&plot_doc.doc, ids);
        commit(&mut plot_doc, &mut history, command);
        if selection.clear() {
            selection_events.send(PlotSelectionChanged { selected: 0 });
        }
        return;
    }

    // Duplicate (Ctrl+D).
    if ctrl && keys.just_pressed(KeyCode::KeyD) {
        let command = duplicate_commands(&mut plot_doc.doc, &ids);
        let new_ids = command.targets();
        commit(&mut plot_doc, &mut history, command);
        selection.0 = new_ids.into_iter().collect();
        selection_events.send(PlotSelectionChanged {
            selected: selection.0.len() as u32,
        });
        return;
    }

    // Nudge (arrows, one step per tap so holding does not flood the history).
    let mut dlon = 0.0;
    let mut dlat = 0.0;
    if keys.just_pressed(KeyCode::ArrowLeft) {
        dlon -= NUDGE_DEG;
    }
    if keys.just_pressed(KeyCode::ArrowRight) {
        dlon += NUDGE_DEG;
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        dlat += NUDGE_DEG;
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        dlat -= NUDGE_DEG;
    }
    if dlon != 0.0 || dlat != 0.0 {
        let command = translate_commands(&plot_doc.doc, &ids, dlon, dlat);
        commit(&mut plot_doc, &mut history, command);
    }
}

/// The pointer-drag move system (see module docs).
#[allow(clippy::too_many_arguments)]
pub fn drag_move_system(
    mut drag: ResMut<PlotDrag>,
    mut capture: ResMut<PlotInputCapture>,
    mut plot_doc: ResMut<PlotDocument>,
    mut history: ResMut<PlotHistory>,
    selection: Res<PlotSelection>,
    interaction: Res<PlotInteraction>,
    hover: Res<PlotHover>,
    ctx: Res<PlotViewCtx>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cams: Query<(&Camera, &GlobalTransform, &Projection)>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    // While drawing the FSM owns the pointer; Ctrl+drag is box-select (M7).
    if interaction.is_drawing() {
        return;
    }
    let cursor = windows.get_single().ok().and_then(|w| w.cursor_position());
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);

    if drag.active {
        if mouse.just_released(MouseButton::Left) {
            end_drag(&mut plot_doc, &mut history, &mut drag);
            capture.0 = false;
            *drag = PlotDrag::default();
            return;
        }
        let Some(cur) = cursor else { return };
        if !drag.armed {
            if (cur - drag.start_px).length() < DRAG_THRESHOLD_PX {
                return;
            }
            let Some((cam, ct)) = active_cam(&cams, ctx.mode) else {
                return;
            };
            let Some(start) = screen_to_geo(cam, ct, ctx.mode, cur) else {
                return;
            };
            drag.start_geo = Some(start);
            drag.before = drag
                .moving
                .iter()
                .filter_map(|id| plot_doc.doc.element(*id).map(|e| (*id, e.geometry.clone())))
                .collect();
            drag.armed = true;
        }
        let Some((cam, ct)) = active_cam(&cams, ctx.mode) else {
            return;
        };
        let Some(now) = screen_to_geo(cam, ct, ctx.mode, cur) else {
            return;
        };
        let Some(start) = drag.start_geo else {
            return;
        };
        let (dlon, dlat) = (now.lon_deg - start.lon_deg, now.lat_deg - start.lat_deg);
        for (id, base) in &drag.before {
            if let Some(e) = plot_doc.doc.element_mut(*id) {
                e.set_geometry(transform::translate(base, dlon, dlat));
            }
        }
        plot_doc.mark_dirty();
        return;
    }

    // Grab a drag on a press over an already-selected, editable element; a plain
    // click on a new element is left to the picker.
    if mouse.just_pressed(MouseButton::Left) && !ctrl {
        if let Some(hit) = hover.0 {
            if selection.contains(hit.element) {
                drag.active = true;
                drag.armed = false;
                drag.start_px = cursor.unwrap_or(Vec2::ZERO);
                drag.moving = selection.0.iter().copied().collect();
                capture.0 = true;
            }
        }
    }
}

/// Fold a finished drag into one recorded move command (no-op if nothing moved).
fn end_drag(plot_doc: &mut PlotDocument, history: &mut PlotHistory, drag: &mut PlotDrag) {
    if !drag.armed {
        return;
    }
    let mut steps = Vec::new();
    for (id, base) in &drag.before {
        let Some(e) = plot_doc.doc.element(*id) else {
            continue;
        };
        if &e.geometry != base {
            steps.push(PlotCommand::UpdateGeometry {
                id: *id,
                before: Box::new(base.clone()),
                after: Box::new(e.geometry.clone()),
            });
        }
    }
    if !steps.is_empty() {
        let command = PlotCommand::Composite { steps };
        command.apply(&mut plot_doc.doc);
        history.0.record(command);
        plot_doc.mark_dirty();
    }
}

/// Choose the active camera for the view mode (projection match first, else any
/// active) — the same rule the pick / sync systems use.
fn active_cam<'q>(
    cams: &'q Query<(&Camera, &GlobalTransform, &Projection)>,
    mode: ViewMode,
) -> Option<(&'q Camera, &'q GlobalTransform)> {
    let want_persp = matches!(mode, ViewMode::Globe);
    let mut fallback = None;
    for (c, ct, p) in cams.iter() {
        if !c.is_active {
            continue;
        }
        let is_persp = matches!(p, Projection::Perspective(_));
        if is_persp == want_persp {
            return Some((c, ct));
        }
        if fallback.is_none() {
            fallback = Some((c, ct));
        }
    }
    fallback
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_plot::geo::GeoPoint;
    use cesium_plot::model::geometry::Polyline;
    use cesium_plot::model::{Document, Element};

    fn p(lon: f64, lat: f64) -> GeoPoint {
        GeoPoint::surface(lon, lat)
    }

    fn line_doc() -> (Document, ElementId) {
        let mut doc = Document::with_default_layer();
        let layer = doc.active_layer().unwrap();
        let ne = doc.make_element(
            "l",
            Geometry::Polyline(Polyline {
                positions: vec![p(0.0, 0.0), p(1.0, 1.0)],
            }),
        );
        let id = ne.id;
        doc.add_element_to_layer(layer, ne);
        (doc, id)
    }

    #[test]
    fn delete_command_removes_then_undo_restores() {
        let (mut doc, id) = line_doc();
        let cmd = delete_commands(&doc, [id]);
        assert_eq!(cmd.targets(), vec![id]);
        cmd.apply(&mut doc);
        assert!(doc.element(id).is_none());
        cmd.inverse().apply(&mut doc);
        assert!(doc.element(id).is_some(), "undo re-adds the element");
    }

    #[test]
    fn duplicate_command_adds_a_new_id() {
        let (mut doc, id) = line_doc();
        let cmd = duplicate_commands(&mut doc, &[id]);
        let new_ids = cmd.targets();
        assert_eq!(new_ids.len(), 1);
        assert_ne!(new_ids[0], id, "the copy has a fresh id");
        cmd.apply(&mut doc);
        assert_eq!(doc.element_count(), 2);
        assert_eq!(
            doc.element(new_ids[0]).unwrap().geometry,
            doc.element(id).unwrap().geometry
        );
        cmd.inverse().apply(&mut doc);
        assert_eq!(doc.element_count(), 1);
    }

    #[test]
    fn translate_command_moves_and_undoes() {
        let (mut doc, id) = line_doc();
        let cmd = translate_commands(&doc, &[id], 2.0, 1.0);
        cmd.apply(&mut doc);
        let moved = doc.element(id).unwrap().geometry.vertices();
        assert_eq!((moved[0].lon_deg, moved[0].lat_deg), (2.0, 1.0));
        cmd.inverse().apply(&mut doc);
        let back = doc.element(id).unwrap().geometry.vertices();
        assert_eq!((back[0].lon_deg, back[0].lat_deg), (0.0, 0.0));
    }

    #[test]
    fn translate_skips_ineditable_elements() {
        let (mut doc, id) = line_doc();
        doc.element_mut(id).unwrap().flags.editable = false;
        let cmd = translate_commands(&doc, &[id], 5.0, 5.0);
        assert!(
            matches!(&cmd, PlotCommand::Composite { steps } if steps.is_empty()),
            "an uneditable selection yields no steps"
        );
    }

    #[test]
    fn history_roundtrips_a_duplicate() {
        use cesium_plot::ops::HistoryStack;
        let (mut doc, id) = line_doc();
        let mut h = HistoryStack::new();
        let dup = duplicate_commands(&mut doc, &[id]);
        dup.apply(&mut doc);
        h.record(dup);
        assert_eq!(doc.element_count(), 2);
        h.undo(&mut doc);
        assert_eq!(doc.element_count(), 1);
    }

    #[test]
    fn end_drag_records_a_single_move_when_armed() {
        let (doc, id) = line_doc();
        let mut plot_doc = PlotDocument {
            doc,
            revision: 0,
            dirty: false,
        };
        let mut history = PlotHistory::default();
        // Move the element in the document, then fold it as a completed drag.
        let base = Geometry::Polyline(Polyline {
            positions: vec![p(0.0, 0.0), p(1.0, 1.0)],
        });
        let mut drag = PlotDrag {
            active: true,
            armed: true,
            start_px: Vec2::ZERO,
            start_geo: Some(p(0.0, 0.0)),
            moving: vec![id],
            before: vec![(id, base)],
        };
        plot_doc.doc.element_mut(id).unwrap().set_geometry(Geometry::Polyline(Polyline {
            positions: vec![p(3.0, 3.0), p(4.0, 4.0)],
        }));
        end_drag(&mut plot_doc, &mut history, &mut drag);
        assert!(history.0.can_undo());
        history.0.undo(&mut plot_doc.doc);
        let g = plot_doc.doc.element(id).unwrap().geometry.vertices();
        assert_eq!((g[0].lon_deg, g[0].lat_deg), (0.0, 0.0));
    }

    #[test]
    fn end_drag_is_a_noop_when_not_armed() {
        let (doc, _id) = line_doc();
        let mut plot_doc = PlotDocument {
            doc,
            revision: 0,
            dirty: false,
        };
        let mut history = PlotHistory::default();
        // Pressed (active) but never crossed the movement threshold (not armed).
        let mut drag = PlotDrag {
            active: true,
            ..PlotDrag::default()
        };
        end_drag(&mut plot_doc, &mut history, &mut drag);
        assert!(!history.0.can_undo());
    }

    #[test]
    fn a_moved_snapshot_is_independent() {
        let (mut doc, id) = line_doc();
        let snapshot = doc.element(id).cloned().unwrap();
        doc.element_mut(id).unwrap().name = "changed".into();
        assert_eq!(snapshot.name, "l");
        let _ = Element::new(ElementId(123), "unused", Geometry::Point(p(0.0, 0.0)));
    }
}
