//! Ported from `packages/engine/Source/Core/GeometryFactory.js`.
//!
//! Base class for all geometry creation utility classes that can be
//! passed to [`GeometryInstance`] for asynchronous geometry creation.

use crate::geometry::Geometry;

/// Abstract base for geometry factories usable with `GeometryInstance`.
pub trait GeometryFactory {
    /// Computes the geometric representation, including vertices and indices.
    fn create_geometry(&self) -> Option<Geometry>;
}
