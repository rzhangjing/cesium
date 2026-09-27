//! cesium-plot-bevy — the Bevy bridge for the 2D/3D situational plotting
//! overlay.
//!
//! This adapter owns the ECS half of the feature: the shared bridge resources
//! the app writes each frame ([`resources::PlotViewCtx`],
//! [`resources::PlotInputCapture`]), and — from later milestones on — the view
//! sync / reprojection / picking / interaction systems that project the
//! framework-free [`cesium_plot`] scene document onto render layer 3.
//!
//! It never depends on the application layer; the app drives it purely through
//! these resources and the plugin ([`CesiumPlotBridgePlugin`]).
//!
//! Design doc: plan `cesium-plot_标绘系统总体设计`.

pub mod labels;
pub mod edit;
pub mod interaction;
pub mod io;
pub mod panel;
pub mod picking;
pub mod plugin;
pub mod reproject;
pub mod resources;
pub mod shapes;
pub mod surface;
pub mod sync;
pub mod ui;

pub use labels::PlotUiRoot;
pub use edit::{delete_commands, duplicate_commands, edit_system, translate_commands, PlotDrag};
pub use interaction::{
    screen_to_geo, PlotDrawFinished, PlotInteraction, PlotSetTool, PlotTool,
};
pub use picking::{pick_at, PlotContextMenu, PlotHoverChanged, PlotPickCache, PlotSelectionChanged};
pub use io::{export_geojson, import_geojson};
pub use panel::{apply_panel_action, kind_label, PanelAction, PanelRootEntity};
pub use plugin::CesiumPlotBridgePlugin;
pub use resources::{
    PlotDocument, PlotFilters, PlotHistory, PlotHover, PlotInputCapture, PlotLabel, PlotSelection,
    PlotSnap, PlotViewCtx, PlotViewMode, PlotVisual, PlotVisuals,
};
