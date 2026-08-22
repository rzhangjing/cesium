//! Ported from `packages/engine/Source/Core/PolygonHierarchy.js`.

use crate::cartesian3::Cartesian3;

/// A hierarchy of linear rings which define a polygon and its holes.
/// The holes themselves may also have holes which nest inner polygons.
#[derive(Debug, Clone, Default)]
pub struct PolygonHierarchy {
    /// A linear ring defining the outer boundary of the polygon or hole.
    pub positions: Vec<Cartesian3>,
    /// An array of polygon hierarchies defining holes in the polygon.
    pub holes: Vec<PolygonHierarchy>,
}

impl PolygonHierarchy {
    pub fn new(positions: Vec<Cartesian3>, holes: Vec<PolygonHierarchy>) -> Self {
        Self { positions, holes }
    }
}
