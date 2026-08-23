//! Ported from `packages/engine/Source/DataSources/Visualizer.js`.
//!
//! Interface for visualizers that create and update scene primitives from entity data.

use crate::bounding_sphere_state::BoundingSphereState;
use crate::entity::Entity;

/// Interface for visualizers that synchronize entity data with scene primitives.
///
/// Each visualizer is responsible for a specific type of entity graphics
/// (e.g., billboards, labels, polylines).
///
/// In CesiumJS, Visualizer is an interface with:
/// - `update(time)` → boolean
/// - `getBoundingSphere(entity, result)` → BoundingSphereState
/// - `isDestroyed()` → boolean
/// - `destroy()`
pub trait Visualizer {
    /// Updates the primitives created by this visualizer to match the
    /// entity and time provided.
    ///
    /// Returns true if the update was successful, false otherwise.
    fn update(&mut self, time: f64) -> bool;

    /// Computes a bounding sphere for the given entity.
    ///
    /// Returns `BoundingSphereState::Done` if the result is valid,
    /// `BoundingSphereState::Pending` if still computing, or
    /// `BoundingSphereState::Failed` if the entity has no visualization.
    ///
    /// The default implementation returns `Failed` for all entities.
    fn get_bounding_sphere(&self, _entity: &Entity, _result: &mut [f64; 4]) -> BoundingSphereState {
        BoundingSphereState::Failed
    }

    /// Returns whether this visualizer has been destroyed.
    fn is_destroyed(&self) -> bool;

    /// Destroys the resources held by this visualizer.
    fn destroy(&mut self);
}
