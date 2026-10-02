//! 一个极简的绘制工具栏（计划 §8 / §17.5）：一小条按钮，每个
//! [`DrawKind`] 一个，发布交互 FSM 会接收的 [`PlotSetTool`] 事件。叠加层
//! 拥有它，且只在窗口分支上 spawn（headless 从不添加该插件，因此基线
//! 保持字节一致）；应用也可直接通过事件驱动工具，使此 UI 保持可选。

use bevy::prelude::*;

use cesium_plot::ops::DrawKind;

use crate::interaction::{PlotSetTool, PlotTool};

/// 每个工具栏按钮所选的工具。
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct ToolButton(PlotTool);

/// 工具栏根节点的标记，以免被重复 spawn。
#[derive(Component)]
struct ToolbarRoot;

/// 记住已 spawn 的工具栏根节点的资源（为与插件的其他资源保持对称而存在；
/// 根节点在 [`plot_toolbar`] 中只 spawn 一次）。
#[derive(Resource, Default)]
pub struct PlotToolbarRoot {
    /// 已 spawn 的根 UI 实体。
    pub root: Option<Entity>,
}

/// 每个按钮的标签 + 工具，按显示顺序排列。
const BUTTONS: &[(&str, DrawKind)] = &[
    ("点", DrawKind::Point),
    ("线", DrawKind::Polyline),
    ("面", DrawKind::Polygon),
    ("矩形", DrawKind::Rectangle),
    ("圆", DrawKind::Circle),
    ("取消", /* 哨兵 */ DrawKind::Point),
];

/// spawn 工具栏（一个 Startup 系统）。按钮位于左上角；最后一个是一个
/// 回到 idle 的特殊取消按钮。
///
/// # 参数
/// - `commands`：spawn 根节点与逐按钮实体的 ECS 命令。
/// - `root_res`：记录已 spawn 根节点的资源，避免重复创建。
pub fn plot_toolbar(mut commands: Commands, mut root_res: ResMut<PlotToolbarRoot>) {
    let root = commands
        .spawn((
            ToolbarRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(8.0),
                top: Val::Px(8.0),
                column_gap: Val::Px(6.0),
                padding: UiRect::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.6)),
            GlobalZIndex(2000),
        ))
        .id();
    root_res.root = Some(root);

    for (label, kind) in BUTTONS {
        let tool = if *label == "取消" {
            PlotTool::Idle
        } else {
            PlotTool::Draw(*kind)
        };
        commands.entity(root).with_children(|c| {
            c.spawn((
                ToolButton(tool),
                Button,
                Node {
                    width: Val::Px(48.0),
                    height: Val::Px(28.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.2, 0.4, 0.7, 1.0)),
            ));
            c.spawn((
                Text::new(*label),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
                Node {
                    // 通过把标签与按钮同父到一个相对定位的行单元格，
                    // 将标签叠加在按钮上。
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Px(48.0),
                    height: Val::Px(28.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                GlobalZIndex(2001),
            ));
        });
    }
}

/// 将被按下的工具栏按钮转为一个 [`PlotSetTool`] 事件（Bevy 0.15 没有
/// `Interaction::Clicked`，因此按下沿就是触发器）。
///
/// # 参数
/// - `interactions`：携带 [`ToolButton`] 且状态变化的按钮查询。
/// - `tool_events`：向 FSM 发布所选工具的写出器。
pub(crate) fn toolbar_click_system(
    interactions: Query<(&Interaction, &ToolButton), Changed<Interaction>>,
    mut tool_events: EventWriter<PlotSetTool>,
) {
    for (interaction, button) in interactions.iter() {
        if matches!(interaction, Interaction::Pressed) {
            tool_events.send(PlotSetTool(button.0));
        }
    }
}
