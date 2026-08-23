//! Ported from `packages/engine/Source/DataSources/GeometryUpdater.js`.
//!
//! Interface for geometry updaters that create geometry instances from entity data.

use crate::entity::Entity;

/// Interface for updaters that create geometry instances from entity data.
///
/// Each geometry updater handles a specific type of entity geometry
/// (e.g., box, corridor, cylinder) and produces GeometryInstance objects.
pub trait GeometryUpdater {
    /// Gets the entity associated with this updater.
    fn entity_id(&self) -> &str;

    /// Gets whether this updater handles fill geometry.
    fn fill_enabled(&self) -> bool;

    /// Gets whether this updater handles outline geometry.
    fn outline_enabled(&self) -> bool;

    /// Gets whether this updater is on a surface (ground).
    fn is_on_surface(&self) -> bool;

    /// Gets whether the geometry is closed (for extruded geometry).
    fn is_closed(&self) -> bool;
}

/// The type of geometry produced by an updater.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryType {
    /// Fill geometry.
    Fill,
    /// Outline geometry.
    Outline,
    /// Both fill and outline.
    FillAndOutline,
}
