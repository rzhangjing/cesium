//! Tests for `cesium_core::OrthographicFrustum`.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::orthographic_frustum::OrthographicFrustum;

#[test]
fn default_creates_frustum() {
    let f = OrthographicFrustum::default();
    assert_eq!(f.near, 1.0);
}

#[test]
fn compute_culling_volume_returns_six_planes() {
    let mut f = OrthographicFrustum::new();
    f.width = Some(10.0);
    f.aspect_ratio = Some(1.0);
    let pos = Cartesian3::new(0.0, 0.0, 0.0);
    let dir = Cartesian3::new(0.0, 0.0, -1.0);
    let up = Cartesian3::new(0.0, 1.0, 0.0);
    let cv = f.compute_culling_volume(&pos, &dir, &up);
    assert_eq!(cv.planes.len(), 6);
}
