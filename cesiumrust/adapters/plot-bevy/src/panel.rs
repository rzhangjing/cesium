//! The layer / property / visibility panels (plan §9, §10, M7).
//!
//! Everything user-facing folds into one reversible action enum
//! ([`PanelAction`]) applied by the free function [`apply_panel_action`]. That
//! function touches only plain resources — the [`PlotDocument`], [`PlotHistory`],
//! [`PlotFilters`] and [`PlotSelection`] — so the *entire* panel contract (layer
//! tree order / visibility / lock / focus / opacity, the ten-dimension
//! visibility switches, and the selected-element style editor) is unit-testable
//! headless, exactly like the M6 edit bridge. The Bevy UI is a thin shell:
//!  * [`panel_startup`] spawns the docked root;
//!  * [`bind_panel_camera`] keeps its `TargetCamera` pointed at whichever camera
//!    is active every frame (the multi-camera UI invariant — see the labels
//!    module note), so the panel is visible in both the 2D and the 3D view;
//!  * [`panel_click_system`] maps a pressed button to its [`PanelAction`];
//!  * [`panel_sync_system`] rebuilds the button tree when a signature derived
//!    from the document / filters / selection changes.
//!
//! The panel lives only on the windowed branch (the whole bridge plugin does),
//! so the headless offscreen baseline never instantiates it.

use bevy::prelude::*;

use cesium_plot::model::geometry::GeometryKind;
use cesium_plot::model::ids::LayerId;
use cesium_plot::model::{ElementId, ViewMode};
use cesium_plot::model::Rgba;

use crate::edit::{apply_command, delete_commands, duplicate_commands, style_command};
use crate::resources::{PlotDocument, PlotFilters, PlotHistory, PlotSelection};

/// One discrete thing a panel control can do. `Copy` so it can ride along on a
/// button component and be replayed directly in tests.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PanelAction {
    // §10.1 / §10.8 master + focus switches.
    ToggleOverlay,
    ToggleOnlySelected,
    /// §10.5 type dimension: flip one geometry kind.
    ToggleType(GeometryKind),
    // §9 / §10.2 layer-tree controls.
    ToggleLayerVisible(LayerId),
    ToggleLayerLock(LayerId),
    ToggleLayerSelectable(LayerId),
    FocusLayer(LayerId),
    NudgeLayer(LayerId, i32),
    CycleLayerOpacity(LayerId),
    AddLayer,
    // Selected-element style editor (undoable).
    SetSelectionColor(Rgba),
    AdjustWidth(f32),
    ToggleFill,
    ToggleDepthTest,
    ToggleShowFlat,
    ToggleShowGlobe,
    // Selected-element batch ops (undoable).
    DeleteSelection,
    DuplicateSelection,
    ToggleSelectionVisible,
}

/// Opaque UI markers: a button carrying the action it fires.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct PanelButton(PanelAction);

/// Marker on the docked panel root so the camera-bind + rebuild systems find it.
#[derive(Component)]
pub(crate) struct PanelRoot;

/// Resource remembering the spawned panel root (spawned once in [`panel_startup`]).
#[derive(Resource, Default)]
pub struct PanelRootEntity {
    pub root: Option<Entity>,
}

/// Cycle of layer opacity presets (the panel has a single cycle button rather
/// than a slider — enough to exercise the §9 opacity multiplier).
const OPACITY_STEPS: [f32; 4] = [1.0, 0.75, 0.5, 0.25];

/// Swatch colours offered by the style editor.
const COLOR_PRESETS: [Rgba; 5] = [
    [0.90, 0.20, 0.20, 1.0], // red
    [0.20, 0.75, 0.30, 1.0], // green
    [0.20, 0.45, 0.90, 1.0], // blue
    [0.95, 0.75, 0.15, 1.0], // amber
    [0.95, 0.95, 0.95, 1.0], // white
];

/// The next element-id-ordered snapshot of the selection (deterministic).
fn selected_ids(selection: &PlotSelection) -> Vec<ElementId> {
    let mut ids: Vec<ElementId> = selection.0.iter().copied().collect();
    ids.sort_by_key(|id| id.raw());
    ids
}

/// The layer opacity preset following `cur` in [`OPACITY_STEPS`].
fn next_opacity(cur: f32) -> f32 {
    for i in 0..OPACITY_STEPS.len() {
        if (cur - OPACITY_STEPS[i]).abs() < 1e-3 {
            return OPACITY_STEPS[(i + 1) % OPACITY_STEPS.len()];
        }
    }
    OPACITY_STEPS[0]
}

/// A button label for a geometry kind (the type-filter row).
pub fn kind_label(kind: GeometryKind) -> &'static str {
    match kind {
        GeometryKind::Point => "点",
        GeometryKind::Icon => "图标",
        GeometryKind::Label => "文字",
        GeometryKind::Line => "线",
        GeometryKind::Polygon => "面",
        GeometryKind::Rectangle => "矩形",
        GeometryKind::Circle => "圆",
        GeometryKind::Ellipse => "椭圆",
        GeometryKind::Arc => "弧",
        GeometryKind::Path => "路径",
        GeometryKind::Composite => "组合",
    }
}

/// Apply one panel action. This is the single funnel every control routes
/// through and the heart of the M7 unit tests — no ECS, no window.
pub fn apply_panel_action(
    action: PanelAction,
    plot_doc: &mut PlotDocument,
    history: &mut PlotHistory,
    filters: &mut PlotFilters,
    selection: &mut PlotSelection,
) {
    match action {
        PanelAction::ToggleOverlay => {
            filters.toggle_overlay();
            plot_doc.mark_dirty();
        }
        PanelAction::ToggleOnlySelected => {
            // Seed the focus set from the live selection so turning it on keeps
            // the current picks visible (the evaluator reads `filters.selected`).
            filters.selected = selected_ids(selection).into_iter().collect();
            filters.toggle_only_selected();
            plot_doc.mark_dirty();
        }
        PanelAction::ToggleType(kind) => {
            filters.toggle_type(kind);
            plot_doc.mark_dirty();
        }
        PanelAction::ToggleLayerVisible(id) => {
            let cur = plot_doc.doc.layer(id).map(|l| l.visible).unwrap_or(true);
            plot_doc.doc.set_layer_visible(id, !cur);
            plot_doc.mark_dirty();
        }
        PanelAction::ToggleLayerLock(id) => {
            let cur = plot_doc.doc.layer(id).map(|l| l.editable).unwrap_or(true);
            plot_doc.doc.set_layer_editable(id, !cur);
            plot_doc.mark_dirty();
        }
        PanelAction::ToggleLayerSelectable(id) => {
            let cur = plot_doc.doc.layer(id).map(|l| l.selectable).unwrap_or(true);
            plot_doc.doc.set_layer_selectable(id, !cur);
            plot_doc.mark_dirty();
        }
        PanelAction::FocusLayer(id) => {
            plot_doc.doc.focus_layer(id);
            plot_doc.mark_dirty();
        }
        PanelAction::NudgeLayer(id, delta) => {
            plot_doc.doc.nudge_layer_order(id, delta);
            plot_doc.mark_dirty();
        }
        PanelAction::CycleLayerOpacity(id) => {
            let cur = plot_doc.doc.layer(id).map(|l| l.opacity).unwrap_or(1.0);
            let next = next_opacity(cur);
            plot_doc.doc.set_layer_opacity(id, next);
            plot_doc.mark_dirty();
        }
        PanelAction::AddLayer => {
            let n = plot_doc.doc.layers().len() + 1;
            let id = plot_doc.doc.new_layer(format!("图层 {n}"));
            plot_doc.doc.focus_layer(id);
            plot_doc.mark_dirty();
        }
        PanelAction::DeleteSelection => {
            let ids = selected_ids(selection);
            let command = delete_commands(&plot_doc.doc, ids.iter().copied());
            apply_command(plot_doc, history, command);
            selection.clear();
        }
        PanelAction::DuplicateSelection => {
            let ids = selected_ids(selection);
            let command = duplicate_commands(&mut plot_doc.doc, &ids);
            // duplicate_commands mints fresh ids and stages AddElement steps; the
            // new ids are recorded on the (already applied after apply_command)
            // document. Select them by diffing before / after count is overkill —
            // just clear and let the user re-pick; keep the edit undoable.
            apply_command(plot_doc, history, command);
        }
        PanelAction::ToggleSelectionVisible => {
            let ids = selected_ids(selection);
            let steps: Vec<cesium_plot::ops::PlotCommand> = ids
                .iter()
                .filter_map(|id| {
                    let e = plot_doc.doc.element(*id)?;
                    let before = e.flags.visible_manual;
                    Some(cesium_plot::ops::PlotCommand::SetVisibilityFlag {
                        id: *id,
                        before,
                        after: !before,
                    })
                })
                .collect();
            apply_command(
                plot_doc,
                history,
                cesium_plot::ops::PlotCommand::Composite { steps },
            );
        }
        PanelAction::SetSelectionColor(c) => route_style(plot_doc, history, selection, move |s| {
            s.clone().with_color(c)
        }),
        PanelAction::AdjustWidth(d) => route_style(plot_doc, history, selection, move |s| {
            let mut n = s.clone();
            n.width_px = (n.width_px + d).max(0.5);
            n
        }),
        PanelAction::ToggleFill => route_style(plot_doc, history, selection, |s| {
            let mut n = s.clone();
            n.fill = match n.fill {
                Some(_) => None,
                None => Some([n.color[0], n.color[1], n.color[2], 0.25]),
            };
            n
        }),
        PanelAction::ToggleDepthTest => route_style(plot_doc, history, selection, |s| {
            let mut n = s.clone();
            n.depth_test = !n.depth_test;
            n
        }),
        PanelAction::ToggleShowFlat => route_style(plot_doc, history, selection, |s| {
            let mut n = s.clone();
            n.show_in_flat = !n.show_in_flat;
            n
        }),
        PanelAction::ToggleShowGlobe => route_style(plot_doc, history, selection, |s| {
            let mut n = s.clone();
            n.show_in_globe = !n.show_in_globe;
            n
        }),
    }
}

/// Build + apply an undoable style command over the current selection.
fn route_style(
    plot_doc: &mut PlotDocument,
    history: &mut PlotHistory,
    selection: &mut PlotSelection,
    mutate: impl Fn(&cesium_plot::model::Style) -> cesium_plot::model::Style,
) {
    let ids = selected_ids(selection);
    let command = style_command(&plot_doc.doc, &ids, &mutate);
    apply_command(plot_doc, history, command);
}

/// Fold the panel-relevant state into a small signature so the tree is only
/// rebuilt when something actually changes (idle frames do not churn entities).
fn panel_signature(doc: &PlotDocument, filters: &PlotFilters, selection: &PlotSelection) -> u64 {
    let mut h = 0u64;
    let mut mix = |v: u64| {
        h ^= v.wrapping_add(0x9e3779b97f4a7c15).wrapping_add(h << 6).wrapping_add(h >> 2);
    };
    mix(doc.revision);
    mix(selection.0.len() as u64);
    mix(filters.overlay_enabled as u64);
    mix(filters.only_selected as u64);
    // Type mask: one bit per kind (None == all on).
    for k in GeometryKind::all() {
        mix(filters.type_enabled(*k) as u64 + (*k as u64));
    }
    // Per-layer flags.
    mix(doc.doc.layers().len() as u64);
    for l in doc.doc.layers_ordered() {
        mix(l.id.raw());
        mix(l.visible as u64);
        mix(l.editable as u64);
        mix(l.selectable as u64);
        mix(l.active as u64);
        mix(l.order as u64);
        mix((l.opacity * 100.0) as u64);
    }
    // Selected elements' manual visibility (drives the eye button label).
    let mut sel: Vec<ElementId> = selection.0.iter().copied().collect();
    sel.sort_by_key(|id| id.raw());
    for id in sel {
        if let Some(e) = doc.doc.element(id) {
            mix(id.raw());
            mix(e.flags.visible_manual as u64);
        }
    }
    h
}

/// Resolve the active camera by projection match, falling back to any active one
/// (same rule the sync system uses) — returns the entity to bind the UI root to.
fn active_camera(cams: &Query<(Entity, &Camera, &Projection)>, mode: ViewMode) -> Option<Entity> {
    let mut fallback: Option<Entity> = None;
    for (e, c, p) in cams.iter() {
        if !c.is_active {
            continue;
        }
        let matches = match mode {
            ViewMode::Globe => matches!(p, Projection::Perspective(_)),
            ViewMode::Flat => matches!(p, Projection::Orthographic(_)),
        };
        if matches {
            return Some(e);
        }
        if fallback.is_none() {
            fallback = Some(e);
        }
    }
    fallback
}

/// Spawn the docked panel root (a Startup system). The button tree is filled in
/// by [`panel_sync_system`]; here we only create the empty right-docked column.
pub(crate) fn panel_startup(mut commands: Commands, mut root: ResMut<PanelRootEntity>) {
    let entity = commands
        .spawn((
            PanelRoot,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(8.0),
                top: Val::Px(8.0),
                bottom: Val::Px(8.0),
                width: Val::Px(240.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.12, 0.13, 0.16, 0.82)),
            GlobalZIndex(2500),
        ))
        .id();
    root.root = Some(entity);
}

/// Keep the panel root's `TargetCamera` pointed at the active camera every frame
/// (the multi-camera UI invariant). Writes only when the binding changes so idle
/// frames do not emit change ticks.
pub(crate) fn bind_panel_camera(
    mut commands: Commands,
    ctx: Res<crate::resources::PlotViewCtx>,
    cams: Query<(Entity, &Camera, &Projection)>,
    roots: Query<(Entity, Option<&TargetCamera>), With<PanelRoot>>,
) {
    if roots.is_empty() {
        return;
    }
    let Some(active) = active_camera(&cams, ctx.mode) else {
        return;
    };
    for (e, bound) in roots.iter() {
        if bound.map(|t| t.0) != Some(active) {
            commands.entity(e).insert(TargetCamera(active));
        }
    }
}

/// Map a pressed panel button to its action.
pub(crate) fn panel_click_system(
    interactions: Query<(&Interaction, &PanelButton), Changed<Interaction>>,
    mut plot_doc: ResMut<PlotDocument>,
    mut history: ResMut<PlotHistory>,
    mut filters: ResMut<PlotFilters>,
    mut selection: ResMut<PlotSelection>,
) {
    for (interaction, button) in interactions.iter() {
        if matches!(interaction, Interaction::Pressed) {
            apply_panel_action(button.0, &mut plot_doc, &mut history, &mut filters, &mut selection);
        }
    }
}

/// Rebuild the button tree whenever the panel signature changes.
#[allow(clippy::too_many_arguments)]
pub(crate) fn panel_sync_system(
    mut commands: Commands,
    plot_doc: Res<PlotDocument>,
    filters: Res<PlotFilters>,
    selection: Res<PlotSelection>,
    root_ent: Single<Entity, With<PanelRoot>>,
    mut last: Local<u64>,
) {
    let sig = panel_signature(&plot_doc, &filters, &selection);
    if sig == *last {
        return;
    }
    *last = sig;

    let root = *root_ent;
    commands.entity(root).despawn_descendants();

    // Snapshot everything we need to render (avoid borrowing `commands` while we
    // still hold document refs).
    let overlay_on = filters.overlay_enabled;
    let focus_on = filters.only_selected;
    let kinds: Vec<(GeometryKind, bool)> = GeometryKind::all()
        .iter()
        .map(|k| (*k, filters.type_enabled(*k)))
        .collect();
    let layers: Vec<(LayerId, String, bool, bool, bool, bool, f32)> = plot_doc
        .doc
        .layers_ordered()
        .into_iter()
        .rev()
        .map(|l| {
            (
                l.id,
                l.name.clone(),
                l.visible,
                l.editable,
                l.selectable,
                l.active,
                l.opacity,
            )
        })
        .collect();
    let sel_count = selection.0.len();
    let sel_visible = selected_ids(&selection)
        .first()
        .and_then(|id| plot_doc.doc.element(*id))
        .map(|e| e.flags.visible_manual)
        .unwrap_or(true);

    commands.entity(root).with_children(|col| {
        col.spawn(section_title("标绘面板 · Plot"));
        col.spawn(button(PanelAction::ToggleOverlay, on_off(overlay_on, "总显隐 ON", "总显隐 OFF")));
        col.spawn(button(
            PanelAction::ToggleOnlySelected,
            on_off(focus_on, "仅选中 ON", "仅选中 OFF"),
        ));

        col.spawn(section_title("类型 Type"));
        col.spawn(
            Node {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                column_gap: Val::Px(3.0),
                row_gap: Val::Px(3.0),
                ..default()
            },
        )
        .with_children(|row| {
            for (kind, on) in &kinds {
                row.spawn(button(PanelAction::ToggleType(*kind), on_off(*on, kind_label(*kind), "")));
            }
        });

        col.spawn(section_title("图层 Layers"));
        col.spawn(
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                flex_grow: 1.0,
                overflow: Overflow::clip_y(),
                ..default()
            },
        )
        .with_children(|list| {
            for (id, name, visible, editable, selectable, active, opacity) in &layers {
                list.spawn(
                    Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(3.0),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                )
                .with_children(|r| {
                    let tag = if *active { "▶" } else { "  " };
                    r.spawn(label(format!(
                        "{tag}{name} · {:.0}%",
                        opacity * 100.0
                    )));
                    r.spawn(button(
                        PanelAction::ToggleLayerVisible(*id),
                        on_off(*visible, "👁", "－"),
                    ));
                    r.spawn(button(
                        PanelAction::ToggleLayerLock(*id),
                        on_off(*editable, "开", "锁"),
                    ));
                    r.spawn(button(
                        PanelAction::ToggleLayerSelectable(*id),
                        on_off(*selectable, "选", "－"),
                    ));
                    r.spawn(button(PanelAction::FocusLayer(*id), "置"));
                    r.spawn(button(PanelAction::NudgeLayer(*id, 1), "↑"));
                    r.spawn(button(PanelAction::NudgeLayer(*id, -1), "↓"));
                    r.spawn(button(PanelAction::CycleLayerOpacity(*id), "透"));
                });
            }
            list.spawn(button(PanelAction::AddLayer, "＋新建层"));
        });

        col.spawn(section_title(format!("样式 Style (选中 {sel_count})")));
        col.spawn(
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(3.0),
                ..default()
            },
        )
        .with_children(|row| {
            for c in COLOR_PRESETS {
                row.spawn(color_swatch(PanelAction::SetSelectionColor(c), c));
            }
        });
        col.spawn(
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(3.0),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
        )
        .with_children(|row| {
            row.spawn(button(PanelAction::AdjustWidth(1.0), "粗+"));
            row.spawn(button(PanelAction::AdjustWidth(-1.0), "细-"));
            row.spawn(button(PanelAction::ToggleFill, "填充"));
            row.spawn(button(PanelAction::ToggleDepthTest, "深度"));
            row.spawn(button(PanelAction::ToggleShowFlat, "2D"));
            row.spawn(button(PanelAction::ToggleShowGlobe, "3D"));
        });
        col.spawn(
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(3.0),
                ..default()
            },
        )
        .with_children(|row| {
            row.spawn(button(PanelAction::DeleteSelection, "删除"));
            row.spawn(button(PanelAction::DuplicateSelection, "复制"));
            row.spawn(button(
                PanelAction::ToggleSelectionVisible,
                on_off(sel_visible, "隐", "显"),
            ));
        });
    });
}

fn section_title(text: impl Into<String>) -> Text {
    Text::new(text.into())
}

fn label(text: String) -> (Text, TextFont, TextColor) {
    (
        Text::new(text),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.9, 0.95)),
    )
}

/// "on" shows `on_text`; "off" shows `off_text` (often empty for a type chip).
fn on_off(on: bool, on_text: &'static str, off_text: &'static str) -> &'static str {
    if on {
        on_text
    } else {
        off_text
    }
}

fn button(action: PanelAction, text: &str) -> (
    PanelButton,
    Button,
    Node,
    BackgroundColor,
    Text,
    TextFont,
    TextColor,
) {
    (
        PanelButton(action),
        Button,
        Node {
            min_width: Val::Px(28.0),
            height: Val::Px(24.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.24, 0.34, 0.5, 1.0)),
        Text::new(text.to_string()),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 1.0, 1.0)),
    )
}

fn color_swatch(action: PanelAction, c: Rgba) -> (PanelButton, Button, Node, BackgroundColor) {
    (
        PanelButton(action),
        Button,
        Node {
            width: Val::Px(22.0),
            height: Val::Px(22.0),
            ..default()
        },
        BackgroundColor(Color::srgba(c[0], c[1], c[2], c[3])),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_plot::geo::GeoPoint;
    use cesium_plot::model::geometry::Geometry;
    use cesium_plot::ops::HistoryStack;

    fn fresh() -> (PlotDocument, PlotHistory, PlotFilters, PlotSelection) {
        let doc = PlotDocument {
            doc: cesium_plot::model::Document::with_default_layer(),
            revision: 0,
            dirty: true,
        };
        (
            doc,
            PlotHistory(HistoryStack::new()),
            PlotFilters::default(),
            PlotSelection::default(),
        )
    }

    fn add_point(doc: &mut PlotDocument, lon: f64, lat: f64) -> ElementId {
        let layer = doc.doc.active_layer().unwrap();
        let ne = doc.doc.make_element("p", Geometry::Point(GeoPoint::surface(lon, lat)));
        let id = ne.id;
        doc.doc.add_element_to_layer(layer, ne);
        id
    }

    #[test]
    fn overlay_and_focus_toggles_route_to_filters() {
        let (mut d, mut h, mut f, mut s) = fresh();
        apply_panel_action(PanelAction::ToggleOverlay, &mut d, &mut h, &mut f, &mut s);
        assert!(!f.overlay_enabled);
        apply_panel_action(PanelAction::ToggleOverlay, &mut d, &mut h, &mut f, &mut s);
        assert!(f.overlay_enabled);
        // Toggling focus on seeds the selected set from the live selection.
        let e = add_point(&mut d, 1.0, 2.0);
        s.select_one(e);
        apply_panel_action(PanelAction::ToggleOnlySelected, &mut d, &mut h, &mut f, &mut s);
        assert!(f.only_selected);
        assert!(f.selected.contains(&e));
    }

    #[test]
    fn type_toggle_flips_enabled_mask() {
        let (mut d, mut h, mut f, mut s) = fresh();
        assert!(f.type_enabled(GeometryKind::Polygon));
        apply_panel_action(PanelAction::ToggleType(GeometryKind::Polygon), &mut d, &mut h, &mut f, &mut s);
        assert!(!f.type_enabled(GeometryKind::Polygon));
        assert!(f.type_enabled(GeometryKind::Point));
    }

    #[test]
    fn layer_tree_controls_mutate_document() {
        let (mut d, mut h, mut f, mut s) = fresh();
        let a = d.doc.active_layer().unwrap();
        let b = d.doc.new_layer("B");
        // Visibility / lock / selectable toggles.
        apply_panel_action(PanelAction::ToggleLayerVisible(a), &mut d, &mut h, &mut f, &mut s);
        assert!(!d.doc.layer(a).unwrap().visible);
        apply_panel_action(PanelAction::ToggleLayerLock(a), &mut d, &mut h, &mut f, &mut s);
        assert!(!d.doc.layer(a).unwrap().editable);
        apply_panel_action(PanelAction::ToggleLayerSelectable(a), &mut d, &mut h, &mut f, &mut s);
        assert!(!d.doc.layer(a).unwrap().selectable);
        // Focus moves the active marker; nudge changes order; opacity cycles.
        apply_panel_action(PanelAction::FocusLayer(b), &mut d, &mut h, &mut f, &mut s);
        assert_eq!(d.doc.active_layer(), Some(b));
        let before = d.doc.layer(b).unwrap().order;
        apply_panel_action(PanelAction::NudgeLayer(b, 1), &mut d, &mut h, &mut f, &mut s);
        assert_eq!(d.doc.layer(b).unwrap().order, before + 1);
        assert_eq!(d.doc.layer(b).unwrap().opacity, 1.0);
        apply_panel_action(PanelAction::CycleLayerOpacity(b), &mut d, &mut h, &mut f, &mut s);
        assert_eq!(d.doc.layer(b).unwrap().opacity, OPACITY_STEPS[1]);
        // Add layer grows the tree and focuses it.
        let n = d.doc.layers().len();
        apply_panel_action(PanelAction::AddLayer, &mut d, &mut h, &mut f, &mut s);
        assert_eq!(d.doc.layers().len(), n + 1);
        assert_eq!(d.doc.active_layer(), d.doc.layers().last().map(|l| l.id));
    }

    #[test]
    fn style_edits_are_undoable_and_only_touch_selection() {
        let (mut d, mut h, mut f, mut s) = fresh();
        let e1 = add_point(&mut d, 0.0, 0.0);
        let e2 = add_point(&mut d, 1.0, 1.0);
        s.select_one(e1);
        apply_panel_action(
            PanelAction::SetSelectionColor([1.0, 0.0, 0.0, 1.0]),
            &mut d,
            &mut h,
            &mut f,
            &mut s,
        );
        assert_eq!(d.doc.element(e1).unwrap().style.color, [1.0, 0.0, 0.0, 1.0]);
        // The unselected element keeps its default colour.
        assert_eq!(
            d.doc.element(e2).unwrap().style.color,
            cesium_plot::model::Style::default().color
        );
        // Undo restores the previous style.
        h.0.undo(&mut d.doc);
        assert_eq!(
            d.doc.element(e1).unwrap().style.color,
            cesium_plot::model::Style::default().color
        );
        // A no-op style edit records nothing (empty composite is dropped).
        let before_len = h.0.undo_len();
        apply_panel_action(
            PanelAction::SetSelectionColor(cesium_plot::model::Style::default().color),
            &mut d,
            &mut h,
            &mut f,
            &mut s,
        );
        assert_eq!(h.0.undo_len(), before_len);
    }

    #[test]
    fn delete_and_duplicate_and_visibility_route_through_history() {
        let (mut d, mut h, mut f, mut s) = fresh();
        let e = add_point(&mut d, 5.0, 6.0);
        // Delete → element gone, undo restores.
        s.select_one(e);
        apply_panel_action(PanelAction::DeleteSelection, &mut d, &mut h, &mut f, &mut s);
        assert!(d.doc.element(e).is_none());
        assert!(s.0.is_empty(), "delete clears the selection");
        h.0.undo(&mut d.doc);
        assert!(d.doc.element(e).is_some());
        // Duplicate → new element appears (count grows), undo removes it.
        let count0 = d.doc.element_count();
        s.select_one(e);
        apply_panel_action(PanelAction::DuplicateSelection, &mut d, &mut h, &mut f, &mut s);
        assert_eq!(d.doc.element_count(), count0 + 1);
        h.0.undo(&mut d.doc);
        assert_eq!(d.doc.element_count(), count0);
        // Toggle manual visibility → flag flips, reversible.
        s.select_one(e);
        apply_panel_action(PanelAction::ToggleSelectionVisible, &mut d, &mut h, &mut f, &mut s);
        assert!(!d.doc.element(e).unwrap().flags.visible_manual);
        h.0.undo(&mut d.doc);
        assert!(d.doc.element(e).unwrap().flags.visible_manual);
    }

    #[test]
    fn signature_is_stable_until_state_changes() {
        let (mut d, mut h, mut f, mut s) = fresh();
        let a = d.doc.active_layer().unwrap();
        let e = add_point(&mut d, 0.0, 0.0);
        s.select_one(e);
        let base = panel_signature(&d, &f, &s);
        assert_eq!(base, panel_signature(&d, &f, &s), "idle frames are stable");
        // Turning a layer off (revision bump + flag flip) changes the signature.
        apply_panel_action(PanelAction::ToggleLayerVisible(a), &mut d, &mut h, &mut f, &mut s);
        assert_ne!(base, panel_signature(&d, &f, &s));
    }

    #[test]
    fn opacity_cycle_wraps() {
        assert_eq!(next_opacity(1.0), OPACITY_STEPS[1]);
        assert_eq!(next_opacity(OPACITY_STEPS[3]), OPACITY_STEPS[0]);
        // Unknown value restarts the cycle at full.
        assert_eq!(next_opacity(0.33), OPACITY_STEPS[0]);
    }
}
