//! Document operations (plan §6 / §8): the pure command / history / transform
//! layer the interaction state machine and the edit bridge drive. M5 shipped the
//! draw-commit draft fold and the add / remove commands; M6 adds the reversible
//! geometry / style / visibility commands, the geometry transforms (translate /
//! rotate / scale / vertex-edit) and the [`HistoryStack`] undo / redo recorder.
//! M9 rounds the toolbelt out with coordinate [`snap`]ing and geodesic
//! [`measure`] read-outs — both pure, so they unit-test without an engine.

pub mod command;
pub mod history;
pub mod measure;
pub mod snap;
pub mod transform;

pub use command::{commit_draft, DrawKind, PlotCommand};
pub use history::HistoryStack;
pub use measure::{
    measure_area_m2, measure_length_m, path_length_m, polygon_area_m2, ring_area_m2,
    ring_length_m,
};
pub use snap::{snap, snap_to_grid, SnapConfig, SnapResult};
pub use transform::{rotate, scale, set_vertex, translate};
