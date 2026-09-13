//! Screen-space picking: convert screen coordinates to world rays.
//!
//! Maps to CesiumJS `Scene/Scene.js` pick methods:
//! - `Scene.pick`
//! - `Scene.drillPick`
//! - `Camera.getPickRay`

use cesium_camera::Camera;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::ray::Ray;
use glam::{DVec2, DVec3, DVec4};

/// Viewport dimensions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    /// Width in pixels.
    pub width: f64,
    /// Height in pixels.
    pub height: f64,
}

impl Viewport {
    /// Creates a new viewport.
    pub fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    /// Aspect ratio (width / height).
    pub fn aspect_ratio(&self) -> f64 {
        self.width / self.height
    }
}

/// Computes a picking ray from screen coordinates.
///
/// # Arguments
/// * `screen_position` - Screen coordinates (pixels, origin top-left)
/// * `viewport` - Viewport dimensions
/// * `camera` - The camera
///
/// # Returns
/// A ray in world space, or None if the ray cannot be computed
pub fn get_pick_ray(
    screen_position: DVec2,
    viewport: &Viewport,
    camera: &Camera,
) -> Option<Ray> {
    if viewport.width <= 0.0 || viewport.height <= 0.0 {
        return None;
    }

    // Convert screen coordinates to NDC (-1 to 1)
    let ndc_x = (2.0 * screen_position.x / viewport.width) - 1.0;
    let ndc_y = 1.0 - (2.0 * screen_position.y / viewport.height); // Flip Y

    // Compute the inverse view-projection matrix
    let view_proj = camera.view_projection_matrix();
    let inv_view_proj = view_proj.inverse();

    // Unproject near and far points
    let near_point = DVec4::new(ndc_x, ndc_y, -1.0, 1.0);
    let far_point = DVec4::new(ndc_x, ndc_y, 1.0, 1.0);

    let near_world = inv_view_proj * near_point;
    let far_world = inv_view_proj * far_point;

    // Perspective divide
    let near_world = DVec3::new(
        near_world.x / near_world.w,
        near_world.y / near_world.w,
        near_world.z / near_world.w,
    );
    let far_world = DVec3::new(
        far_world.x / far_world.w,
        far_world.y / far_world.w,
        far_world.z / far_world.w,
    );

    let direction = (far_world - near_world).normalize();

    Some(Ray::new(near_world, direction))
}

/// Computes the intersection of a pick ray with the ellipsoid surface.
///
/// # Arguments
/// * `ray` - The picking ray
/// * `ellipsoid` - The ellipsoid to intersect
///
/// # Returns
/// The intersection point in ECEF, or None if no intersection
pub fn pick_ellipsoid(ray: &Ray, ellipsoid: &Ellipsoid) -> Option<DVec3> {
    // Ray-ellipsoid intersection using quadratic formula
    // Ellipsoid: x²/a² + y²/b² + z²/c² = 1
    let radii = ellipsoid.radii();
    let inv_radii_sq = DVec3::new(
        1.0 / (radii.x * radii.x),
        1.0 / (radii.y * radii.y),
        1.0 / (radii.z * radii.z),
    );

    let origin = ray.origin;
    let direction = ray.direction;

    // Quadratic coefficients: at² + bt + c = 0
    let a = direction.x * direction.x * inv_radii_sq.x
        + direction.y * direction.y * inv_radii_sq.y
        + direction.z * direction.z * inv_radii_sq.z;

    let b = 2.0 * (origin.x * direction.x * inv_radii_sq.x
        + origin.y * direction.y * inv_radii_sq.y
        + origin.z * direction.z * inv_radii_sq.z);

    let c = origin.x * origin.x * inv_radii_sq.x
        + origin.y * origin.y * inv_radii_sq.y
        + origin.z * origin.z * inv_radii_sq.z
        - 1.0;

    let discriminant = b * b - 4.0 * a * c;

    if discriminant < 0.0 {
        return None; // No intersection
    }

    let sqrt_disc = discriminant.sqrt();
    let t1 = (-b - sqrt_disc) / (2.0 * a);
    let t2 = (-b + sqrt_disc) / (2.0 * a);

    // Choose the nearest positive intersection
    let t = if t1 > 0.0 {
        t1
    } else if t2 > 0.0 {
        t2
    } else {
        return None; // Both intersections behind the ray
    };

    Some(ray.origin + ray.direction * t)
}

/// Converts a world position to screen coordinates.
///
/// # Arguments
/// * `world_position` - Position in ECEF
/// * `viewport` - Viewport dimensions
/// * `camera` - The camera
///
/// # Returns
/// Screen coordinates (pixels), or None if behind the camera
pub fn world_to_screen(
    world_position: DVec3,
    viewport: &Viewport,
    camera: &Camera,
) -> Option<DVec2> {
    let view_proj = camera.view_projection_matrix();
    let clip = view_proj * DVec4::new(world_position.x, world_position.y, world_position.z, 1.0);

    // Behind camera check
    if clip.w <= 0.0 {
        return None;
    }

    // Perspective divide → NDC
    let ndc_x = clip.x / clip.w;
    let ndc_y = clip.y / clip.w;
    let ndc_z = clip.z / clip.w;

    // Outside clip volume
    if !(-1.0..=1.0).contains(&ndc_z) {
        return None;
    }

    // NDC → screen
    let screen_x = (ndc_x + 1.0) * 0.5 * viewport.width;
    let screen_y = (1.0 - ndc_y) * 0.5 * viewport.height; // Flip Y

    Some(DVec2::new(screen_x, screen_y))
}

/// Computes the window center as a DVec2.
pub fn window_center(viewport: &Viewport) -> DVec2 {
    DVec2::new(viewport.width * 0.5, viewport.height * 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_camera() -> Camera {
        // Camera above the equator looking at the center
        Camera::new(
            DVec3::new(6378137.0 * 3.0, 0.0, 0.0),
            DVec3::new(-1.0, 0.0, 0.0),
            DVec3::new(0.0, 0.0, 1.0),
        )
    }

    #[test]
    fn test_viewport() {
        let vp = Viewport::new(1920.0, 1080.0);
        assert!((vp.aspect_ratio() - 16.0 / 9.0).abs() < 1e-10);
    }

    #[test]
    fn test_get_pick_ray_center() {
        let camera = create_test_camera();
        let viewport = Viewport::new(800.0, 600.0);
        let center = DVec2::new(400.0, 300.0);

        let ray = get_pick_ray(center, &viewport, &camera).unwrap();

        // Ray origin should be near the camera (at the near plane)
        let dist_to_camera = (ray.origin - camera.position).length();
        assert!(dist_to_camera < camera.position.length() * 0.1);

        // Ray direction should be roughly towards -X (looking at center of Earth)
        assert!(ray.direction.x < -0.9);
    }

    #[test]
    fn test_get_pick_ray_invalid_viewport() {
        let camera = create_test_camera();
        let viewport = Viewport::new(0.0, 0.0);

        let ray = get_pick_ray(DVec2::new(100.0, 100.0), &viewport, &camera);
        assert!(ray.is_none());
    }

    #[test]
    fn test_pick_ellipsoid_hit() {
        let camera = create_test_camera();
        let viewport = Viewport::new(800.0, 600.0);
        let center = DVec2::new(400.0, 300.0);

        let ray = get_pick_ray(center, &viewport, &camera).unwrap();
        let hit = pick_ellipsoid(&ray, &Ellipsoid::WGS84);

        assert!(hit.is_some());
        let hit_point = hit.unwrap();

        // Hit point should be on the ellipsoid surface
        let radii = Ellipsoid::WGS84.radii();
        let normalized = DVec3::new(
            hit_point.x / radii.x,
            hit_point.y / radii.y,
            hit_point.z / radii.z,
        );
        assert!((normalized.length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_pick_ellipsoid_miss() {
        // Ray pointing away from the ellipsoid
        let ray = Ray::new(
            DVec3::new(6378137.0 * 3.0, 0.0, 0.0),
            DVec3::new(1.0, 0.0, 0.0), // Pointing away
        );

        let hit = pick_ellipsoid(&ray, &Ellipsoid::WGS84);
        assert!(hit.is_none());
    }

    #[test]
    fn test_world_to_screen_center() {
        let camera = create_test_camera();
        let viewport = Viewport::new(800.0, 600.0);

        // A point directly in front of the camera should project to screen center
        let world_point = DVec3::new(6378137.0 * 2.0, 0.0, 0.0);
        let screen = world_to_screen(world_point, &viewport, &camera);

        assert!(screen.is_some());
        let screen = screen.unwrap();
        // Should be near center
        assert!((screen.x - 400.0).abs() < 50.0);
        assert!((screen.y - 300.0).abs() < 50.0);
    }

    #[test]
    fn test_world_to_screen_behind_camera() {
        let camera = create_test_camera();
        let viewport = Viewport::new(800.0, 600.0);

        // A point behind the camera
        let world_point = DVec3::new(6378137.0 * 5.0, 0.0, 0.0);
        let screen = world_to_screen(world_point, &viewport, &camera);

        assert!(screen.is_none());
    }

    #[test]
    fn test_window_center() {
        let viewport = Viewport::new(1920.0, 1080.0);
        let center = window_center(&viewport);
        assert!((center.x - 960.0).abs() < 1e-10);
        assert!((center.y - 540.0).abs() < 1e-10);
    }

    #[test]
    fn test_pick_ray_offset() {
        let camera = create_test_camera();
        let viewport = Viewport::new(800.0, 600.0);

        // Pick at top-left corner
        let ray = get_pick_ray(DVec2::new(0.0, 0.0), &viewport, &camera).unwrap();

        // Should be different from center ray
        let center_ray = get_pick_ray(DVec2::new(400.0, 300.0), &viewport, &camera).unwrap();

        // Directions should differ
        let dot = ray.direction.dot(center_ray.direction);
        assert!(dot < 0.999); // Not the same direction
    }
}
