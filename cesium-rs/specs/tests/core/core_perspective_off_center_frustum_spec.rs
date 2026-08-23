//! Tests for `cesium_core::PerspectiveOffCenterFrustum`.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::perspective_off_center_frustum::PerspectiveOffCenterFrustum;

#[test]
fn default_creates_frustum() {
    let f = PerspectiveOffCenterFrustum::default();
    assert_eq!(f.near, 1.0);
    assert_eq!(f.far, 500_000_000.0);
}

#[test]
fn compute_projection_matrix_returns_valid_matrix() {
    let mut f = PerspectiveOffCenterFrustum::new();
    f.left = Some(-1.0);
    f.right = Some(1.0);
    f.bottom = Some(-1.0);
    f.top = Some(1.0);
    let m = f.compute_projection_matrix();
    // col(0,0) should be 2*near/(right-left) = 2*1/2 = 1.0
    assert!((m.elements[0] - 1.0).abs() < 1e-10);
}

#[test]
fn compute_culling_volume_returns_six_planes() {
    let mut f = PerspectiveOffCenterFrustum::new();
    f.left = Some(-1.0);
    f.right = Some(1.0);
    f.bottom = Some(-1.0);
    f.top = Some(1.0);
    let pos = Cartesian3::new(0.0, 0.0, 0.0);
    let dir = Cartesian3::new(0.0, 0.0, -1.0);
    let up = Cartesian3::new(0.0, 1.0, 0.0);
    let cv = f.compute_culling_volume(&pos, &dir, &up);
    assert_eq!(cv.planes.len(), 6);
}
