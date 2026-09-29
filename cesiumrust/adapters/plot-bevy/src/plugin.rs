//! `CesiumPlotBridgePlugin` —— 仅在窗口模式下注册的叠加层桥接。
//!
//! M2–M6 注册共享资源、将场景文档投影到渲染层 3（同时 2D / 3D）的
//! 视图同步系统、拾取系统（悬停 + 点击选择）、绘制 FSM 与编辑层（移动 /
//! 复制 / 删除 / 撤销重做）。该插件只在应用的窗口分
//! 支上添加，因此 headless 离屏基线根本不会实例化
//! 它，并保持字节一致。

use bevy::prelude::*;

use crate::edit::{drag_move_system, edit_system, PlotDrag};
use crate::interaction::{
    draw_preview_system, interaction_system, PlotDrawFinished, PlotInteraction, PlotSetTool,
};
use crate::picking::{pick_system, PlotContextMenu, PlotHoverChanged, PlotPickCache, PlotSelectionChanged};
use crate::panel::{
    bind_panel_camera, panel_click_system, panel_startup, panel_sync_system, PanelRootEntity,
};
use crate::resources::{
    PlotDocument, PlotFilters, PlotHistory, PlotHover, PlotInputCapture, PlotSelection, PlotSnap,
    PlotViewCtx, PlotVisuals,
};
use crate::sync::sync_visuals;
use crate::ui::{plot_toolbar, toolbar_click_system};

/// 接线标绘叠加层桥接的 Bevy 插件。仅由应用在窗口分支上注册，
/// 因此 headless 离屏捕获保持不变。
pub struct CesiumPlotBridgePlugin;

impl Plugin for CesiumPlotBridgePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlotViewCtx>()
            .init_resource::<PlotInputCapture>()
            .init_resource::<PlotDocument>()
            .init_resource::<PlotFilters>()
            .init_resource::<PlotVisuals>()
            .init_resource::<PlotSelection>()
            .init_resource::<PlotHover>()
            .init_resource::<PlotInteraction>()
            .init_resource::<PlotHistory>()
            .init_resource::<PlotDrag>()
            .init_resource::<PlotSnap>()
            .init_resource::<PlotPickCache>()
            .init_resource::<crate::sync::SyncState>()
            .init_resource::<crate::sync::PlotShapeCache>()
            .init_resource::<crate::ui::PlotToolbarRoot>()
            .init_resource::<PanelRootEntity>()
            .add_event::<PlotHoverChanged>()
            .add_event::<PlotSelectionChanged>()
            .add_event::<PlotContextMenu>()
            .add_event::<PlotSetTool>()
            .add_event::<PlotDrawFinished>()
            .add_systems(Startup, (plot_toolbar, panel_startup))
            .add_systems(
                Update,
                (
                    sync_visuals,
                    pick_system,
                    interaction_system,
                    draw_preview_system,
                    edit_system,
                    drag_move_system,
                    toolbar_click_system,
                    bind_panel_camera,
                    panel_click_system,
                    panel_sync_system,
                ),
            );
    }
}
