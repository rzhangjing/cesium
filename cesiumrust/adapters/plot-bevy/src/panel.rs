//! 图层 / 属性 / 可见性面板（计划 §9, §10, M7）。
//!
//! 所有用户面向的操作都折叠为一个可撤销的动作枚举
//! （[`PanelAction`]），由自由函数 [`apply_panel_action`] 应用。该
//! 函数只操作普通资源——[`PlotDocument`]、[`PlotHistory`]、
//! [`PlotFilters`] 和 [`PlotSelection`]——因此*整个*面板契约（层
//! 树顺序 / 可见性 / 锁定 / 聚焦 / 不透明度，十维
//! 可见性开关，以及已选元素的样式编辑器）可以 headless 单测，
//! 与 M6 编辑桥接一致。Bevy UI 是薄壳：
//!  * [`panel_startup`] 创建停靠根节点；
//!  * [`bind_panel_camera`] 每帧保持其 `TargetCamera` 指向当前激活相机
//!    （多相机 UI 不变量——参见 labels 模块注释），
//!    因此面板在 2D 和 3D 视图中都可见；
//!  * [`panel_click_system`] 将按钮按下映射为其 [`PanelAction`]；
//!  * [`panel_sync_system`] 当从文档 / 筛选 / 选择派生的签名变化时重建按钮树。
//!
//! 面板仅存在于窗口分支（整个桥接插件也是如此），
//! 因此 headless 离屏基线永远不会实例化它。

use bevy::prelude::*;

use cesium_plot::model::geometry::GeometryKind;
use cesium_plot::model::ids::LayerId;
use cesium_plot::model::{ElementId, ViewMode};
use cesium_plot::model::Rgba;

use crate::edit::{apply_command, delete_commands, duplicate_commands, style_command};
use crate::resources::{PlotDocument, PlotFilters, PlotHistory, PlotSelection};

/// 面板控件能做的一个离散操作。`Copy` 以便它可以搭载在按钮组件上
/// 并在测试中直接回放。
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PanelAction {
    // §10.1 / §10.8 总开关 + 聚焦开关。
    ToggleOverlay,
    ToggleOnlySelected,
    /// §10.5 类型维度：翻转一个几何类型。
    ToggleType(GeometryKind),
    // §9 / §10.2 层树控件。
    ToggleLayerVisible(LayerId),
    ToggleLayerLock(LayerId),
    ToggleLayerSelectable(LayerId),
    FocusLayer(LayerId),
    NudgeLayer(LayerId, i32),
    CycleLayerOpacity(LayerId),
    AddLayer,
    // 已选元素样式编辑器（可撤销）。
    SetSelectionColor(Rgba),
    AdjustWidth(f32),
    ToggleFill,
    ToggleDepthTest,
    ToggleShowFlat,
    ToggleShowGlobe,
    // 已选元素批量操作（可撤销）。
    DeleteSelection,
    DuplicateSelection,
    ToggleSelectionVisible,
}

/// 不透明 UI 标记：携带按钮所触发的动作。
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct PanelButton(PanelAction);

/// 停靠面板根的标记，以便相机绑定 + 重建系统找到它。
#[derive(Component)]
pub(crate) struct PanelRoot;

/// 记住已创建面板根的资源（在 [`panel_startup`] 中创建一次）。
#[derive(Resource, Default)]
pub struct PanelRootEntity {
    pub root: Option<Entity>,
}

/// 层不透明度预设的循环（面板使用单循环按钮而非
/// 滑条——足以触发 §9 不透明度乘数）。
const OPACITY_STEPS: [f32; 4] = [1.0, 0.75, 0.5, 0.25];

/// 样式编辑器提供的色板颜色。
const COLOR_PRESETS: [Rgba; 5] = [
    [0.90, 0.20, 0.20, 1.0], // 红
    [0.20, 0.75, 0.30, 1.0], // 绿
    [0.20, 0.45, 0.90, 1.0], // 蓝
    [0.95, 0.75, 0.15, 1.0], // 琥珀
    [0.95, 0.95, 0.95, 1.0], // 白
];

/// 选择集的按元素 id 排序的快照（确定性）。
fn selected_ids(selection: &PlotSelection) -> Vec<ElementId> {
    let mut ids: Vec<ElementId> = selection.0.iter().copied().collect();
    ids.sort_by_key(|id| id.raw());
    ids
}

/// [`OPACITY_STEPS`] 中 `cur` 之后的下一个不透明度预设。
fn next_opacity(cur: f32) -> f32 {
    for i in 0..OPACITY_STEPS.len() {
        if (cur - OPACITY_STEPS[i]).abs() < 1e-3 {
            return OPACITY_STEPS[(i + 1) % OPACITY_STEPS.len()];
        }
    }
    OPACITY_STEPS[0]
}

/// 几何类型的按钮标签（类型筛选行）。
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

/// 应用一个面板动作。这是每个控件经过的唯一入口，
/// 也是 M7 单元测试的核心——无 ECS、无窗口。
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
            // 从实时选择集注入聚焦集，因此开启时保持
            // 当前选取可见（评估器读取 `filters.selected`）。
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
            // duplicate_commands 铸造新 id 并暂存 AddElement 步；新 id
            // 记录在（apply_command 后已应用的）文档上。通过 diff
            // 前后数量来选择它们是多余的——
            // 直接清除并让用户重新拾取；保持编辑可撤销。
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

/// 在当前选择集上构建 + 应用一个可撤销的样式命令。
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

/// 将面板相关状态折叠为一个小的签名，以便只在真正发生变化时才重建按钮树
/// （空闲帧不会 churn 实体）。
fn panel_signature(doc: &PlotDocument, filters: &PlotFilters, selection: &PlotSelection) -> u64 {
    let mut h = 0u64;
    let mut mix = |v: u64| {
        h ^= v.wrapping_add(0x9e3779b97f4a7c15).wrapping_add(h << 6).wrapping_add(h >> 2);
    };
    mix(doc.revision);
    mix(selection.0.len() as u64);
    mix(filters.overlay_enabled as u64);
    mix(filters.only_selected as u64);
    // 类型掩码：每种类型一个 bit（None == 全开）。
    for k in GeometryKind::all() {
        mix(filters.type_enabled(*k) as u64 + (*k as u64));
    }
    // 每层标志。
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
    // 已选元素的手动可见性（驱动眼睛按钮标签）。
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

/// 通过投影匹配解析激活相机，回退到任意一个激活的
/// （与同步系统使用相同的规则）——返回要绑定 UI 根节点的实体。
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

/// 创建停靠面板根节点（一个 Startup 系统）。按钮树由
/// [`panel_sync_system`] 填充；这里只创建空的右侧停靠列。
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

/// 每帧保持面板根的 `TargetCamera` 指向激活相机
/// （多相机 UI 不变量）。只在绑定变化时写入，因此空闲
/// 帧不会发出 change tick。
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

/// 将按下的面板按钮映射为其动作。
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

/// 当面板签名变化时重建按钮树。
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

    // 快照渲染所需的一切（避免在持有文档引用时
    // 借用 `commands`）。
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

/// "on" 显示 `on_text`；"off" 显示 `off_text`（类型 chip 常为空）。
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
        // 开启聚焦时从实时选择集注入已选集。
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
        // 可见性 / 锁定 / 可选择性切换。
        apply_panel_action(PanelAction::ToggleLayerVisible(a), &mut d, &mut h, &mut f, &mut s);
        assert!(!d.doc.layer(a).unwrap().visible);
        apply_panel_action(PanelAction::ToggleLayerLock(a), &mut d, &mut h, &mut f, &mut s);
        assert!(!d.doc.layer(a).unwrap().editable);
        apply_panel_action(PanelAction::ToggleLayerSelectable(a), &mut d, &mut h, &mut f, &mut s);
        assert!(!d.doc.layer(a).unwrap().selectable);
        // 聚焦移动活动标记；nudge 改变顺序；不透明度循环。
        apply_panel_action(PanelAction::FocusLayer(b), &mut d, &mut h, &mut f, &mut s);
        assert_eq!(d.doc.active_layer(), Some(b));
        let before = d.doc.layer(b).unwrap().order;
        apply_panel_action(PanelAction::NudgeLayer(b, 1), &mut d, &mut h, &mut f, &mut s);
        assert_eq!(d.doc.layer(b).unwrap().order, before + 1);
        assert_eq!(d.doc.layer(b).unwrap().opacity, 1.0);
        apply_panel_action(PanelAction::CycleLayerOpacity(b), &mut d, &mut h, &mut f, &mut s);
        assert_eq!(d.doc.layer(b).unwrap().opacity, OPACITY_STEPS[1]);
        // 新建层增长树并聚焦它。
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
        // 未选元素保持其默认颜色。
        assert_eq!(
            d.doc.element(e2).unwrap().style.color,
            cesium_plot::model::Style::default().color
        );
        // 撤销恢复之前的样式。
        h.0.undo(&mut d.doc);
        assert_eq!(
            d.doc.element(e1).unwrap().style.color,
            cesium_plot::model::Style::default().color
        );
        // 空操作样式编辑不记录任何内容（空 composite 被丢弃）。
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
        // 删除 → 元素消失，撤销恢复。
        s.select_one(e);
        apply_panel_action(PanelAction::DeleteSelection, &mut d, &mut h, &mut f, &mut s);
        assert!(d.doc.element(e).is_none());
        assert!(s.0.is_empty(), "delete clears the selection");
        h.0.undo(&mut d.doc);
        assert!(d.doc.element(e).is_some());
        // 复制 → 新元素出现（计数增长），撤销移除它。
        let count0 = d.doc.element_count();
        s.select_one(e);
        apply_panel_action(PanelAction::DuplicateSelection, &mut d, &mut h, &mut f, &mut s);
        assert_eq!(d.doc.element_count(), count0 + 1);
        h.0.undo(&mut d.doc);
        assert_eq!(d.doc.element_count(), count0);
        // 切换手动可见性 → 标志翻转，可撤销。
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
        // 关闭一层（revision bump + 标志翻转）改变签名。
        apply_panel_action(PanelAction::ToggleLayerVisible(a), &mut d, &mut h, &mut f, &mut s);
        assert_ne!(base, panel_signature(&d, &f, &s));
    }

    #[test]
    fn opacity_cycle_wraps() {
        assert_eq!(next_opacity(1.0), OPACITY_STEPS[1]);
        assert_eq!(next_opacity(OPACITY_STEPS[3]), OPACITY_STEPS[0]);
        // 未知值从头开始循环为全不透明。
        assert_eq!(next_opacity(0.33), OPACITY_STEPS[0]);
    }
}
