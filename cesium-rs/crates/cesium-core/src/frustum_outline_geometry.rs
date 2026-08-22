//! Ported from `packages/engine/Source/Core/FrustumOutlineGeometry.js`.

use crate::cartesian3::Cartesian3;
use crate::cartesian4::Cartesian4;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FrustumOutlineGeometry {
    origin: Cartesian3,
    orientation: Cartesian4,
    near: f64,
    far: f64,
    fov: f64,
    aspect_ratio: f64,
}

impl FrustumOutlineGeometry {
    pub fn new(origin: Cartesian3, orientation: Cartesian4, near: f64, far: f64, fov: f64, aspect_ratio: f64) -> Self {
        Self { origin, orientation, near, far, fov, aspect_ratio }
    }
    // TODO: create_geometry
}
