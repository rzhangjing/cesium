//! Ported from `packages/engine/Source/Core/CoplanarPolygonOutlineGeometry.js`.

use crate::cartesian3::Cartesian3;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CoplanarPolygonOutlineGeometry {
    positions: Vec<Cartesian3>,
}

impl CoplanarPolygonOutlineGeometry {
    pub fn new(positions: Vec<Cartesian3>) -> Self {
        Self { positions }
    }
    // TODO: create_geometry
}
