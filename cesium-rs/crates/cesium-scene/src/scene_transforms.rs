//! Ported from `packages/engine/Source/Scene/SceneTransforms.js`.
//!
//! M3/S3 materialization: `SceneTransforms.worldToWindowCoordinates` (the
//! widget layer's screen-space projection, e.g. the SelectionIndicator's
//! default `computeScreenSpacePosition`) is ported one-to-one: transform the
//! world position through the camera's view-projection matrix, reject
//! points behind the camera (`p.w <= 0` → JS `undefined`), and convert the
//! NDC coordinates into window coordinates with the y axis pointing down
//! (`windowY = height * (1 - ndcY) / 2`).

use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartographic::Cartographic;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::matrix4::Matrix4;
use cesium_core::transforms;

use crate::camera::Camera;

/// Scene transform utilities for converting between coordinate systems.
///
/// Provides functions for world-to-window, window-to-world, and
/// cartographic-to-window conversions.
/// Mirrors CesiumJS `SceneTransforms` (558 lines).
pub struct SceneTransforms;

impl SceneTransforms {
    /// Converts a world position to window (screen) coordinates.
    ///
    /// Mirrors CesiumJS `SceneTransforms.worldToWindowCoordinates`:
    /// `camera.getDeriveCommand` → MVP transform → perspective divide →
    /// NDC-to-window mapping. Returns `None` when the position is behind
    /// the camera (the JS returns `undefined`).
    pub fn world_to_window(
        position: &Cartesian3,
        view_projection: &Matrix4,
        viewport: (i32, i32, i32, i32),
    ) -> Option<Cartesian2> {
        Self::world_to_window_coordinates(position, view_projection, viewport)
    }

    /// The CesiumJS `worldToWindowCoordinates` signature mapped to Rust:
    /// `Option<Cartesian2>` replaces the JS `result | undefined`.
    pub fn world_to_window_coordinates(
        position: &Cartesian3,
        view_projection: &Matrix4,
        viewport: (i32, i32, i32, i32),
    ) -> Option<Cartesian2> {
        let clip = Matrix4::multiply_by_point_new(view_projection, position);
        // The w component (the matrix's third row; the port's
        // `multiply_by_point` returns xyz without a perspective divide).
        let e = &view_projection.elements;
        let w = e[3] * position.x + e[7] * position.y + e[11] * position.z + e[15];
        // Behind the camera (or on the near plane) the JS returns undefined.
        if w <= 0.0 {
            return None;
        }
        let ndc_x = clip.x / w;
        let ndc_y = clip.y / w;
        let (_x, _y, width, height) = viewport;
        let window_x = width as f64 * (ndc_x + 1.0) * 0.5;
        let window_y = height as f64 * (1.0 - ndc_y) * 0.5;
        Some(Cartesian2::new(window_x, window_y))
    }

    /// Projects the camera view/projection pair and converts a world
    /// position to window coordinates (the scene-level convenience form of
    /// [`SceneTransforms::world_to_window_coordinates`]).
    pub fn world_to_window_with_camera(
        position: &Cartesian3,
        camera: &Camera,
    ) -> Option<Cartesian2> {
        let view_projection = Matrix4::multiply_new(
            camera.projection_matrix(),
            camera.view_matrix(),
        );
        let viewport = (
            0,
            0,
            camera.canvas_width() as i32,
            camera.canvas_height() as i32,
        );
        Self::world_to_window_coordinates(position, &view_projection, viewport)
    }

    /// Converts window (screen) coordinates to a world ray.
    pub fn window_to_world(
        _window_position: &Cartesian2,
        _projection: &Matrix4,
        _viewport: (i32, i32, i32, i32),
    ) -> Cartesian3 {
        // DEVIATION: Requires inverse projection
        Cartesian3::ZERO
    }

    /// Converts a cartographic position to window coordinates (mirrors the
    /// JS `cartographicToWindowCoordinates`: ellipsoid → world → window).
    pub fn cartographic_to_window(
        cartographic: &Cartographic,
        view_projection: &Matrix4,
        viewport: (i32, i32, i32, i32),
        ellipsoid: Option<&Ellipsoid>,
    ) -> Option<Cartesian2> {
        let ellipsoid = ellipsoid.cloned().unwrap_or(Ellipsoid::WGS84);
        let mut world = Cartesian3::default();
        ellipsoid.cartographic_to_cartesian(cartographic, &mut world);
        Self::world_to_window_coordinates(&world, view_projection, viewport)
    }

    /// Returns the WGS84 to fixed frame transform at a given position
    /// (the east-north-up frame, mirroring the JS helper).
    pub fn wgs84_to_fixed_frame(position: &Cartesian3) -> Matrix4 {
        transforms::east_north_up_to_fixed_frame_new(position, None)
    }
}

impl Default for SceneTransforms {
    fn default() -> Self { Self }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_core::math::CesiumMath;

    /// A camera at the origin looking down -Z over an 800×600 canvas.
    fn camera_looking_down_z() -> Camera {
        let mut camera = Camera::new();
        camera.set_canvas_size(800, 600);
        camera.set_position(Cartesian3::new(0.0, 0.0, 0.0));
        camera.set_direction(Cartesian3::new(0.0, 0.0, -1.0));
        camera.set_up(Cartesian3::new(0.0, 1.0, 0.0));
        camera.set_right(Cartesian3::new(1.0, 0.0, 0.0));
        camera.update();
        camera
    }

    /// Mirrors SceneTransformsSpec "worldToWindowCoordinates": a point on
    /// the view axis projects to the window center.
    #[test]
    fn projects_view_axis_point_to_window_center() {
        let camera = camera_looking_down_z();
        let position = Cartesian3::new(0.0, 0.0, -100.0);
        let window = SceneTransforms::world_to_window_with_camera(&position, &camera);
        let window = window.expect("point in front of the camera projects");
        assert!((window.x - 400.0).abs() < 1e-9);
        assert!((window.y - 300.0).abs() < 1e-9);
    }

    /// Mirrors SceneTransformsSpec: lateral offsets follow the projection
    /// (a point on +right shifts the window x right of center).
    #[test]
    fn projects_lateral_offsets() {
        let camera = camera_looking_down_z();
        // At distance d the half-width is d * tan(fov/2) * aspect; use the
        // half-width itself so the point lands on the right viewport edge.
        let distance = 100.0;
        let half_height = distance * (camera.fov() * 0.5).tan();
        let half_width = half_height * (800.0 / 600.0);
        let position = Cartesian3::new(half_width, 0.0, -distance);
        let window = SceneTransforms::world_to_window_with_camera(&position, &camera)
            .expect("point inside the frustum");
        assert!((window.x - 800.0).abs() < 1e-6);
        assert!((window.y - 300.0).abs() < 1e-6);
    }

    /// Mirrors SceneTransformsSpec: a point behind the camera returns
    /// undefined (`None`).
    #[test]
    fn returns_none_behind_the_camera() {
        let camera = camera_looking_down_z();
        let position = Cartesian3::new(0.0, 0.0, 100.0);
        assert!(SceneTransforms::world_to_window_with_camera(&position, &camera).is_none());
    }

    /// The y axis points down in window coordinates (NDC +y is the screen
    /// top → smaller window y).
    #[test]
    fn window_y_points_down() {
        let camera = camera_looking_down_z();
        let distance = 100.0;
        let half_height = distance * (camera.fov() * 0.5).tan();
        let up = Cartesian3::new(0.0, half_height, -distance);
        let down = Cartesian3::new(0.0, -half_height, -distance);
        let up_window = SceneTransforms::world_to_window_with_camera(&up, &camera).unwrap();
        let down_window = SceneTransforms::world_to_window_with_camera(&down, &camera).unwrap();
        assert!(up_window.y < down_window.y);
        assert!((up_window.y - 0.0).abs() < 1e-6);
        assert!((down_window.y - 600.0).abs() < 1e-6);
    }

    /// `cartographicToWindowCoordinates` chains the ellipsoid conversion
    /// into the same projection.
    #[test]
    fn cartographic_to_window_chains_the_ellipsoid() {
        let camera = camera_looking_down_z();
        let view_projection = Matrix4::multiply_new(
            camera.projection_matrix(),
            camera.view_matrix(),
        );
        // Place a cartographic point right in front of the origin camera:
        // lon 0, lat 0, negative height lands at (0, 0, -100) on the WGS84
        // surface offset. Use the direct cartographic of the world point.
        let world = Cartesian3::new(0.0, 0.0, -100.0);
        let mut cartographic = Cartographic::default();
        assert!(Ellipsoid::WGS84.cartesian_to_cartographic(&world, &mut cartographic));
        let window = SceneTransforms::cartographic_to_window(
            &cartographic,
            &view_projection,
            (0, 0, 800, 600),
            None,
        );
        let window = window.expect("surface point in front of the camera");
        assert!((window.x - 400.0).abs() < 1e-6);
        assert!((window.y - 300.0).abs() < 1e-6);
    }

    /// `wgs84ToFixedFrame` produces the ENU frame (up axis equals the
    /// geodetic surface normal).
    #[test]
    fn wgs84_to_fixed_frame_is_enu() {
        let position = Cartesian3::new(Ellipsoid::WGS84.maximum_radius(), 0.0, 0.0);
        let frame = SceneTransforms::wgs84_to_fixed_frame(&position);
        let e = &frame.elements;
        // The up column (third basis vector) points along +X at lon 0/lat 0.
        assert!((e[8] - 1.0).abs() < CesiumMath::EPSILON10);
        assert!(e[9].abs() < CesiumMath::EPSILON10);
        assert!(e[10].abs() < CesiumMath::EPSILON10);
    }
}
