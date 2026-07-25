//! Panorama rendering (Equirectangular + CubeMap).
//!
//! Maps to CesiumJS:
//! - `Scene/EquirectangularPanorama.js`
//! - `Scene/CubeMapPanorama.js`
//! - `Scene/PanoramaProvider.js`

use glam::DMat4;

/// Default panorama radius in meters.
pub const DEFAULT_PANORAMA_RADIUS: f64 = 100000.0;

/// An equirectangular panorama rendered on a sphere.
///
/// Maps to CesiumJS `Scene/EquirectangularPanorama.js`.
#[derive(Debug, Clone, PartialEq)]
pub struct EquirectangularPanorama {
    /// 4x4 transformation matrix defining position and orientation.
    pub transform: DMat4,
    /// Image URL or resource identifier.
    pub image: String,
    /// Radius of the panorama sphere in meters.
    pub radius: f64,
    /// Number of times to repeat the texture horizontally.
    pub repeat_horizontal: f64,
    /// Number of times to repeat the texture vertically.
    pub repeat_vertical: f64,
    /// Credit/attribution string.
    pub credit: Option<String>,
    /// Whether the panorama is visible.
    pub show: bool,
}

impl Default for EquirectangularPanorama {
    fn default() -> Self {
        Self {
            transform: DMat4::IDENTITY,
            image: String::new(),
            radius: DEFAULT_PANORAMA_RADIUS,
            repeat_horizontal: 1.0,
            repeat_vertical: 1.0,
            credit: None,
            show: true,
        }
    }
}

impl EquirectangularPanorama {
    /// Create a new equirectangular panorama with an image.
    pub fn new(image: impl Into<String>) -> Self {
        Self {
            image: image.into(),
            ..Default::default()
        }
    }

    /// Create with transform and image.
    pub fn with_transform(transform: DMat4, image: impl Into<String>) -> Self {
        Self {
            transform,
            image: image.into(),
            ..Default::default()
        }
    }

    /// Set the radius.
    pub fn set_radius(&mut self, radius: f64) -> &mut Self {
        self.radius = radius;
        self
    }

    /// Set horizontal repeat.
    pub fn set_repeat_horizontal(&mut self, repeat: f64) -> &mut Self {
        self.repeat_horizontal = repeat;
        self
    }

    /// Set vertical repeat.
    pub fn set_repeat_vertical(&mut self, repeat: f64) -> &mut Self {
        self.repeat_vertical = repeat;
        self
    }

    /// Set the credit.
    pub fn set_credit(&mut self, credit: impl Into<String>) -> &mut Self {
        self.credit = Some(credit.into());
        self
    }

    /// Compute the texture coordinate for a given direction.
    ///
    /// Direction should be a unit vector in local space.
    /// Returns (u, v) in [0, 1] range (before repeat).
    pub fn direction_to_uv(&self, direction: glam::DVec3) -> [f64; 2] {
        let dir = direction.normalize();
        // Longitude: atan2(y, x) -> [-π, π] -> [0, 1]
        let lon = dir.y.atan2(dir.x);
        let u = (lon + std::f64::consts::PI) / std::f64::consts::TAU;

        // Latitude: asin(z) -> [-π/2, π/2] -> [0, 1]
        let lat = dir.z.asin();
        let v = (lat + std::f64::consts::FRAC_PI_2) / std::f64::consts::PI;

        [u * self.repeat_horizontal, v * self.repeat_vertical]
    }

    /// Compute a direction vector from texture coordinates.
    ///
    /// UV should be in [0, 1] range (after repeat division).
    pub fn uv_to_direction(&self, u: f64, v: f64) -> glam::DVec3 {
        let u_norm = u / self.repeat_horizontal;
        let v_norm = v / self.repeat_vertical;

        let lon = u_norm * std::f64::consts::TAU - std::f64::consts::PI;
        let lat = v_norm * std::f64::consts::PI - std::f64::consts::FRAC_PI_2;

        let cos_lat = lat.cos();
        glam::DVec3::new(
            cos_lat * lon.cos(),
            cos_lat * lon.sin(),
            lat.sin(),
        )
    }
}

/// A cube map panorama rendered from 6 face images.
///
/// Maps to CesiumJS `Scene/CubeMapPanorama.js`.
#[derive(Debug, Clone, PartialEq)]
pub struct CubeMapPanorama {
    /// 4x4 transformation matrix.
    pub transform: DMat4,
    /// Image URLs for the 6 faces: [+X, -X, +Y, -Y, +Z, -Z].
    pub faces: [String; 6],
    /// Radius of the panorama sphere in meters.
    pub radius: f64,
    /// Credit/attribution string.
    pub credit: Option<String>,
    /// Whether the panorama is visible.
    pub show: bool,
}

impl Default for CubeMapPanorama {
    fn default() -> Self {
        Self {
            transform: DMat4::IDENTITY,
            faces: [
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ],
            radius: DEFAULT_PANORAMA_RADIUS,
            credit: None,
            show: true,
        }
    }
}

impl CubeMapPanorama {
    /// Create a new cube map panorama with 6 face images.
    pub fn new(faces: [String; 6]) -> Self {
        Self {
            faces,
            ..Default::default()
        }
    }

    /// Check if all faces have images.
    pub fn is_complete(&self) -> bool {
        self.faces.iter().all(|f| !f.is_empty())
    }

    /// Determine which face a direction vector maps to.
    ///
    /// Returns face index (0-5) and (u, v) coordinates on that face.
    pub fn direction_to_face_uv(&self, direction: glam::DVec3) -> (usize, [f64; 2]) {
        let dir = direction.normalize();
        let ax = dir.x.abs();
        let ay = dir.y.abs();
        let az = dir.z.abs();

        if ax >= ay && ax >= az {
            if dir.x > 0.0 {
                // +X face
                let u = (-dir.z / ax + 1.0) * 0.5;
                let v = (-dir.y / ax + 1.0) * 0.5;
                (0, [u, v])
            } else {
                // -X face
                let u = (dir.z / ax + 1.0) * 0.5;
                let v = (-dir.y / ax + 1.0) * 0.5;
                (1, [u, v])
            }
        } else if ay >= ax && ay >= az {
            if dir.y > 0.0 {
                // +Y face
                let u = (dir.x / ay + 1.0) * 0.5;
                let v = (dir.z / ay + 1.0) * 0.5;
                (2, [u, v])
            } else {
                // -Y face
                let u = (dir.x / ay + 1.0) * 0.5;
                let v = (-dir.z / ay + 1.0) * 0.5;
                (3, [u, v])
            }
        } else if dir.z > 0.0 {
            // +Z face
            let u = (dir.x / az + 1.0) * 0.5;
            let v = (dir.y / az + 1.0) * 0.5;
            (4, [u, v])
        } else {
            // -Z face
            let u = (dir.x / az + 1.0) * 0.5;
            let v = (-dir.y / az + 1.0) * 0.5;
            (5, [u, v])
        }
    }
}

/// Panorama provider trait for loading panorama data.
pub trait PanoramaProvider {
    /// Get the panorama type name.
    fn provider_type(&self) -> &str;

    /// Check if the provider is ready.
    fn is_ready(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;

    #[test]
    fn test_equirectangular_default() {
        let pano = EquirectangularPanorama::default();
        assert_eq!(pano.transform, DMat4::IDENTITY);
        assert_eq!(pano.radius, DEFAULT_PANORAMA_RADIUS);
        assert_eq!(pano.repeat_horizontal, 1.0);
        assert_eq!(pano.repeat_vertical, 1.0);
        assert!(pano.show);
        assert!(pano.credit.is_none());
    }

    #[test]
    fn test_equirectangular_new() {
        let pano = EquirectangularPanorama::new("panorama.jpg");
        assert_eq!(pano.image, "panorama.jpg");
    }

    #[test]
    fn test_equirectangular_with_transform() {
        let transform = DMat4::from_translation(DVec3::new(100.0, 200.0, 300.0));
        let pano = EquirectangularPanorama::with_transform(transform, "test.png");
        assert_eq!(pano.transform, transform);
        assert_eq!(pano.image, "test.png");
    }

    #[test]
    fn test_equirectangular_uv_roundtrip() {
        let pano = EquirectangularPanorama::new("test.jpg");

        // Test forward direction (lon=0, lat=0)
        let dir = DVec3::new(1.0, 0.0, 0.0);
        let uv = pano.direction_to_uv(dir);
        assert!((uv[0] - 0.5).abs() < 1e-10); // u = 0.5 at lon=0
        assert!((uv[1] - 0.5).abs() < 1e-10); // v = 0.5 at lat=0

        // Roundtrip
        let dir_back = pano.uv_to_direction(uv[0], uv[1]);
        assert!((dir_back - dir).length() < 1e-10);
    }

    #[test]
    fn test_equirectangular_uv_poles() {
        let pano = EquirectangularPanorama::new("test.jpg");

        // North pole (lat = π/2)
        let north = DVec3::new(0.0, 0.0, 1.0);
        let uv_north = pano.direction_to_uv(north);
        assert!((uv_north[1] - 1.0).abs() < 1e-10);

        // South pole (lat = -π/2)
        let south = DVec3::new(0.0, 0.0, -1.0);
        let uv_south = pano.direction_to_uv(south);
        assert!(uv_south[1].abs() < 1e-10);
    }

    #[test]
    fn test_equirectangular_repeat() {
        let mut pano = EquirectangularPanorama::new("test.jpg");
        pano.set_repeat_horizontal(2.0);
        pano.set_repeat_vertical(3.0);

        let dir = DVec3::new(1.0, 0.0, 0.0);
        let uv = pano.direction_to_uv(dir);
        assert!((uv[0] - 1.0).abs() < 1e-10); // 0.5 * 2
        assert!((uv[1] - 1.5).abs() < 1e-10); // 0.5 * 3
    }

    #[test]
    fn test_cubemap_default() {
        let pano = CubeMapPanorama::default();
        assert!(!pano.is_complete());
        assert_eq!(pano.radius, DEFAULT_PANORAMA_RADIUS);
    }

    #[test]
    fn test_cubemap_new() {
        let faces = [
            "px.jpg".to_string(),
            "nx.jpg".to_string(),
            "py.jpg".to_string(),
            "ny.jpg".to_string(),
            "pz.jpg".to_string(),
            "nz.jpg".to_string(),
        ];
        let pano = CubeMapPanorama::new(faces);
        assert!(pano.is_complete());
    }

    #[test]
    fn test_cubemap_direction_to_face() {
        let pano = CubeMapPanorama::default();

        // +X direction -> face 0
        let (face, uv) = pano.direction_to_face_uv(DVec3::new(1.0, 0.0, 0.0));
        assert_eq!(face, 0);
        assert!((uv[0] - 0.5).abs() < 1e-10);
        assert!((uv[1] - 0.5).abs() < 1e-10);

        // -X direction -> face 1
        let (face, _) = pano.direction_to_face_uv(DVec3::new(-1.0, 0.0, 0.0));
        assert_eq!(face, 1);

        // +Y direction -> face 2
        let (face, _) = pano.direction_to_face_uv(DVec3::new(0.0, 1.0, 0.0));
        assert_eq!(face, 2);

        // -Y direction -> face 3
        let (face, _) = pano.direction_to_face_uv(DVec3::new(0.0, -1.0, 0.0));
        assert_eq!(face, 3);

        // +Z direction -> face 4
        let (face, _) = pano.direction_to_face_uv(DVec3::new(0.0, 0.0, 1.0));
        assert_eq!(face, 4);

        // -Z direction -> face 5
        let (face, _) = pano.direction_to_face_uv(DVec3::new(0.0, 0.0, -1.0));
        assert_eq!(face, 5);
    }

    #[test]
    fn test_equirectangular_builder() {
        let mut pano = EquirectangularPanorama::new("test.jpg");
        pano.set_radius(50000.0)
            .set_repeat_horizontal(2.0)
            .set_repeat_vertical(1.5)
            .set_credit("Test Credit");

        assert_eq!(pano.radius, 50000.0);
        assert_eq!(pano.repeat_horizontal, 2.0);
        assert_eq!(pano.repeat_vertical, 1.5);
        assert_eq!(pano.credit, Some("Test Credit".to_string()));
    }
}
