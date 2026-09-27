//! A minimal draw toolbar (plan §8 / §17.5): a small strip of buttons, one per
//! [`DrawKind`], that publish [`PlotSetTool`] events the interaction FSM picks
//! up. The overlay owns it and only spawns on the windowed branch (headless
//! never adds the plugin, so baselines stay byte-exact); the app may also drive
//! tools directly through the events, keeping this UI optional.

use bevy::prelude::*;

use cesium_plot::ops::DrawKind;

use crate::interaction::{PlotSetTool, PlotTool};

/// The tool each toolbar button selects.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct ToolButton(PlotTool);

/// Marker for the toolbar root so it is not re-spawned.
#[derive(Component)]
struct ToolbarRoot;

/// Resource remembering the spawned toolbar root (present for symmetry with the
/// plugin's other resources; the root is spawned once in [`plot_toolbar`]).
#[derive(Resource, Default)]
pub struct PlotToolbarRoot {
    /// The root UI entity, once spawned.
    pub root: Option<Entity>,
}

/// The label + tool for each button, in display order.
const BUTTONS: &[(&str, DrawKind)] = &[
    ("点", DrawKind::Point),
    ("线", DrawKind::Polyline),
    ("面", DrawKind::Polygon),
    ("矩形", DrawKind::Rectangle),
    ("圆", DrawKind::Circle),
    ("取消", /* sentinel */ DrawKind::Point),
];

/// Spawn the toolbar (a Startup system). Buttons sit top-left; the last one is a
/// special cancel back to idle.
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
                    // Overlay the label on the button by parenting both to a
                    // relative row cell.
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

/// Turn a pressed toolbar button into a [`PlotSetTool`] event (Bevy 0.15 has no
/// `Interaction::Clicked`, so the press edge is the trigger).
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
