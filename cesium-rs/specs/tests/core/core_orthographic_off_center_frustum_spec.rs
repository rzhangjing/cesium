//! Tests for `cesium_core::OrthographicOffCenterFrustum`.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::orthographic_off_center_frustum::OrthographicOffCenterFrustum;

#[test]
fn default_creates_frustum() {
    let f = OrthographicOffCenterFrustum::default();
    assert_eq!(f.near, 1.0);
}

#[test]
fn compute_projection_matrix_is_orthographic() {
    let mut f = OrthographicOffCenterFrustum::new();
    f.left = Some(-5.0);
    f.right = Some(5.0);
    f.bottom = Some(-5.0);
    f.top = Some(5.0);
    let m = f.compute_projection_matrix();
    // col(0,0) = 2/(right-left) = 2/10 = 0.2
    assert!((m.elements[0] - 0.2).abs() < 1e-10);
    // col(3,3) = 1.0 for orthographic
    assert!((m.elements[15] - 1.0).abs() < 1e-10);
}

#[test]
fn compute_culling_volume_returns_six_planes() {
    let mut f = OrthographicOffCenterFrustum::new();
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
