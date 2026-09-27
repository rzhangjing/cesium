//! `CesiumPlotBridgePlugin` — the windowed-only overlay bridge.
//!
//! M2–M6 register the shared resources, the view-sync system that projects the
//! scene document onto render layer 3 (2D / 3D simultaneously), the picking
//! system (hover + click-to-select), the draw FSM and the edit layer (move /
//! duplicate / delete / undo-redo). The plugin is only added on the windowed
//! branch of the app, so the headless offscreen baseline never even instantiates
//! it and stays byte-exact.

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

/// Bevy plugin wiring the plotting overlay bridge. Registered by the app only
/// on the windowed branch, so headless offscreen captures stay unchanged.
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
