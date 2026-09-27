//! Label rendering via bevy_ui text glued to projected geographic anchors.
//!
//! Labels are the one overlay primitive that lives in the UI pass rather than
//! the render layer: a single [`PlotUiRoot`] node owns a [`TargetCamera`] that
//! the sync system re-points at whichever camera is active, and every label is a
//! child whose absolute `left` / `top` track the on-screen projection of its
//! anchor. Because the projection comes from the same [`Camera`] the meshes are
//! measured against, a label sits exactly where its geometry is drawn in both
//! the 2D and the 3D view (plan §2 "同显").

use bevy::prelude::*;
use cesium_plot::model::geometry::LabelGeometry;
use cesium_plot::model::{LabelAnchor, Style};

use crate::resources::PlotLabel;

/// Marker for the single UI root every plot label parents to.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PlotUiRoot;

/// Spawn the plot UI root (a full-window, transparent container). Only one
/// exists; the sync system creates it lazily and keeps its `TargetCamera`
/// pointed at the active camera.
pub fn spawn_ui_root(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
            GlobalZIndex(1000),
            PlotUiRoot,
        ))
        .id()
}

/// Project a world point to the active camera's viewport, returning logical
/// pixel coordinates (top-left origin). `None` when the point is behind the
/// camera or the projection fails.
pub fn world_to_screen(camera: &Camera, ct: &GlobalTransform, world: Vec3) -> Option<Vec2> {
    camera.world_to_viewport(ct, world).ok()
}

/// Per-anchor pixel translation applied to a label's projected screen point so
/// the text box is aligned as the [`LabelAnchor`] requests. `text_px` is the
/// rough measured size of the text box.
pub fn anchor_offset(anchor: LabelAnchor, offset_px: [f32; 2], text_px: Vec2) -> Vec2 {
    let (ax, ay) = match anchor {
        LabelAnchor::Center => (-text_px.x * 0.5, -text_px.y * 0.5),
        LabelAnchor::Left => (0.0, -text_px.y * 0.5),
        LabelAnchor::Right => (-text_px.x, -text_px.y * 0.5),
        LabelAnchor::Top => (-text_px.x * 0.5, 0.0),
        LabelAnchor::Bottom => (-text_px.x * 0.5, -text_px.y),
    };
    Vec2::new(ax + offset_px[0], ay + offset_px[1])
}

/// Spawn a label text node under `root` for one element. The node starts
/// off-screen; the sync system writes its absolute position every frame.
pub fn spawn_label(
    commands: &mut Commands,
    root: Entity,
    element: cesium_plot::model::ids::ElementId,
    geo: &LabelGeometry,
    style: &Style,
) -> Entity {
    let (size_px, rgba) = match &style.text {
        Some(t) => (t.size_px, t.color),
        None => (16.0, style.color),
    };
    commands
        .spawn((
            PlotLabel { element },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(-9999.0),
                top: Val::Px(-9999.0),
                ..default()
            },
            Text::new(geo.text.clone()),
            TextFont {
                font_size: size_px,
                ..default()
            },
            TextColor(Color::srgba(rgba[0], rgba[1], rgba[2], rgba[3])),
        ))
        .set_parent(root)
        .id()
}
