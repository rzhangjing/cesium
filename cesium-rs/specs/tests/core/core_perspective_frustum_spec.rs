//! Tests for `cesium_core::PerspectiveFrustum`.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::perspective_frustum::PerspectiveFrustum;

#[test]
fn default_creates_frustum() {
    let f = PerspectiveFrustum::default();
    assert_eq!(f.near, 1.0);
    assert_eq!(f.far, 500_000_000.0);
}

#[test]
fn update_sets_off_center_from_fov() {
    let mut f = PerspectiveFrustum::new();
    f.fov = Some(std::f64::consts::FRAC_PI_3);
    f.aspect_ratio = Some(1.0);
    f.update();
    // off_center is private, but compute_culling_volume should work
    let pos = Cartesian3::new(0.0, 0.0, 0.0);
    let dir = Cartesian3::new(0.0, 0.0, -1.0);
    let up = Cartesian3::new(0.0, 1.0, 0.0);
    let cv = f.compute_culling_volume(&pos, &dir, &up);
    // Just verify it doesn't panic and returns 6 planes
    assert_eq!(cv.planes.len(), 6);
}
