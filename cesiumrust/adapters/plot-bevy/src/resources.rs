//! Bridge resources shared between the app and the plotting overlay.
//!
//! These are view-side state carriers: the app (which owns the cameras and
//! `MapMode`) writes [`PlotViewCtx`] every frame, and the bridge's view-sync /
//! picking / interaction systems read it. From M2 the bridge also owns the
//! scene document ([`PlotDocument`]), the visibility switches ([`PlotFilters`])
//! and the element → entity reconciliation registry ([`PlotVisuals`]).
//!
//! The canonical projection mode is the core's [`cesium_plot::model::ViewMode`];
//! the bridge aliases it (rather than importing the app's `MapMode`) so the
//! adapter never depends on the application layer (DDD).

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use cesium_plot::model::ids::ElementId;
use cesium_plot::model::{Document, Filters, PickHit, ViewMode};
use cesium_plot::ops::{HistoryStack, SnapConfig};

/// Which projection the overlay renders against — the single workspace-wide
/// definition, aliased from the pure core (see [`cesium_plot::model::ViewMode`]).
pub type PlotViewMode = ViewMode;

/// Per-frame view context handed to the overlay: active projection mode and the
/// metrics needed to convert screen ↔ world. Written by the app's
/// `sync_plot_view_ctx` system; read by the bridge's reprojection / picking.
#[derive(Resource, Clone, Copy, Debug)]
pub struct PlotViewCtx {
    /// Active projection mode.
    pub mode: PlotViewMode,
    /// Flat-map scale (pixels per world unit); `0.0` until the 2D camera runs.
    pub flat_zoom: f32,
    /// Primary window width in logical pixels.
    pub screen_w: f32,
    /// Primary window height in logical pixels.
    pub screen_h: f32,
}

impl Default for PlotViewCtx {
    fn default() -> Self {
        Self {
            mode: PlotViewMode::default(),
            flat_zoom: 0.0,
            screen_w: 0.0,
            screen_h: 0.0,
        }
    }
}

/// Input-capture gate: when `true`, the plotting tool owns the pointer, so the
/// 3D orbit and 2D pan/zoom camera systems must stand down this frame to avoid
/// the camera fighting the edit.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlotInputCapture(pub bool);

impl PlotInputCapture {
    /// Whether pointer input is currently captured by the plot overlay.
    #[inline]
    pub fn is_captured(&self) -> bool {
        self.0
    }
}

/// The scene document the overlay is a view of, plus a change counter the sync
/// systems read to decide whether to re-reconcile. Structural edits go through
/// [`PlotDocument::mark_dirty`] so the revision always advances with content.
#[derive(Resource)]
pub struct PlotDocument {
    /// The framework-free document.
    pub doc: Document,
    /// Monotonic revision, bumped on every content change (`mark_dirty`).
    pub revision: u64,
    /// Set whenever the document changed since the last full reconcile pass;
    /// cleared by the sync system after it catches up.
    pub dirty: bool,
}

impl Default for PlotDocument {
    fn default() -> Self {
        Self {
            doc: Document::default(),
            revision: 0,
            dirty: true,
        }
    }
}

impl PlotDocument {
    /// Record a content change: advance the revision and flag for reconcile.
    pub fn mark_dirty(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.dirty = true;
    }
}

/// The overlay's visibility switches (plan §10), wrapped so the bridge can
/// `Deref`/`DerefMut` through to [`Filters`]. Defaults to the master switch ON
/// with no other restriction — with an empty document nothing is drawn anyway,
/// so the windowed baseline is unaffected.
#[derive(Resource)]
pub struct PlotFilters(pub Filters);

impl Default for PlotFilters {
    fn default() -> Self {
        Self(Filters::enabled())
    }
}

impl std::ops::Deref for PlotFilters {
    type Target = Filters;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for PlotFilters {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Per-element view-side bookkeeping: which ECS entities currently render it
/// (a mesh and / or a label), so the next reconcile pass can update or despawn
/// them instead of leaking.
#[derive(Debug, Clone, Default)]
pub struct VisualEntry {
    /// Entity carrying the element's mesh (point / icon / polyline ribbon).
    pub mesh: Option<Entity>,
    /// The mesh asset handle, kept so ribbon geometry can be updated in place.
    pub mesh_handle: Option<Handle<Mesh>>,
    /// The element's material, kept so colour / opacity edits apply in place.
    pub mat: Option<Handle<bevy::pbr::StandardMaterial>>,
    /// Entity carrying a filled face (polygon / rect / circle / ellipse).
    pub fill: Option<Entity>,
    /// The face's triangulated mesh handle (rebuilt per content change).
    pub fill_handle: Option<Handle<Mesh>>,
    /// The face fill material.
    pub fill_mat: Option<Handle<bevy::pbr::StandardMaterial>>,
    /// Entity carrying the face's screen-constant-width outline stroke.
    pub outline: Option<Entity>,
    /// The outline ribbon mesh handle (rewritten every frame).
    pub outline_handle: Option<Handle<Mesh>>,
    /// The outline material.
    pub outline_mat: Option<Handle<bevy::pbr::StandardMaterial>>,
    /// Entity carrying the element's label text node.
    pub label: Option<Entity>,
}

/// Registry mapping each element to its live visual entities. Owned by the sync
/// system; consulted every reconcile pass.
#[derive(Resource, Default)]
pub struct PlotVisuals {
    /// Element → its visual entry.
    pub entries: HashMap<ElementId, VisualEntry>,
    /// Shared unit-quad mesh handle for point / icon billboards, built lazily.
    pub quad: Option<Handle<Mesh>>,
}

/// Marker on a mesh entity spawned to render one element.
#[derive(Component, Clone, Copy, Debug)]
pub struct PlotVisual {
    /// The element this entity draws.
    pub element: ElementId,
}

/// Marker on a UI text entity spawned to render one element's label.
#[derive(Component, Clone, Copy, Debug)]
pub struct PlotLabel {
    /// The element whose label this is.
    pub element: ElementId,
}

/// The current selection set (plan §8). Cross-layer; the pick system replaces it
/// on a plain click and the interaction FSM extends it (ctrl / box select) later.
#[derive(Resource, Default, Clone, PartialEq, Eq)]
pub struct PlotSelection(pub HashSet<ElementId>);

impl PlotSelection {
    /// Whether `id` is currently selected.
    #[inline]
    pub fn contains(&self, id: ElementId) -> bool {
        self.0.contains(&id)
    }
    /// Replace the selection with a single element (returns false if unchanged).
    pub fn select_one(&mut self, id: ElementId) -> bool {
        if self.0.len() == 1 && self.0.contains(&id) {
            return false;
        }
        self.0.clear();
        self.0.insert(id);
        true
    }
    /// Clear the selection (returns false if it was already empty).
    pub fn clear(&mut self) -> bool {
        if self.0.is_empty() {
            return false;
        }
        self.0.clear();
        true
    }
}

/// The element (if any) currently under the cursor, as resolved by the pick
/// system each frame. Read by the sync system for hover feedback and by the
/// interaction FSM (M5) for drag / edit decisions.
#[derive(Resource, Default, Clone, Copy, PartialEq)]
pub struct PlotHover(pub Option<PickHit>);

/// The undo / redo stack (plan §8 / §14, M6). Every structural edit — a draw
/// commit, a delete / duplicate, a move / vertex / rotate / scale — is applied
/// to the [`PlotDocument`] and recorded here in one step, so the pure
/// [`HistoryStack`] contract (verified headless in the core) drives the live
/// overlay unchanged.
#[derive(Resource, Default)]
pub struct PlotHistory(pub HistoryStack);

/// Coordinate-snapping tunables for the draw FSM (plan §16 M9 "吸附"), wrapping
/// the core's pure [`SnapConfig`]. `Deref`/`DerefMut` straight through to it. The
/// default is **disabled**, so wiring it into `interaction_system` never changes
/// existing draw behaviour until the user (or the app) switches it on.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct PlotSnap(pub SnapConfig);

impl std::ops::Deref for PlotSnap {
    type Target = SnapConfig;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for PlotSnap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
