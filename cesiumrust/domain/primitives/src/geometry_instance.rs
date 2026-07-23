//! Geometry instances and appearance system.
//!
//! Maps to CesiumJS:
//! - `Scene/GeometryInstance.js`
//! - `Scene/Appearance.js`
//! - `Scene/MaterialAppearance.js`
//! - `Scene/PerInstanceColorAppearance.js`

use cesium_geospatial::bounding::BoundingSphere;
use glam::{DMat4, DVec3};

/// A geometry instance with transform and attributes.
///
/// Maps to CesiumJS `Scene/GeometryInstance.js`
#[derive(Debug, Clone)]
pub struct GeometryInstance {
    /// Unique identifier.
    pub id: String,
    /// Geometry type.
    pub geometry_type: GeometryType,
    /// Model matrix (local to world transform).
    pub model_matrix: DMat4,
    /// Per-instance color [r, g, b, a] (0.0-1.0).
    pub color: [f64; 4],
    /// Whether the instance is shown.
    pub show: bool,
    /// Computed bounding sphere.
    pub bounding_sphere: Option<BoundingSphere>,
}

impl GeometryInstance {
    /// Creates a new geometry instance.
    pub fn new(id: impl Into<String>, geometry_type: GeometryType) -> Self {
        Self {
            id: id.into(),
            geometry_type,
            model_matrix: DMat4::IDENTITY,
            color: [1.0, 1.0, 1.0, 1.0],
            show: true,
            bounding_sphere: None,
        }
    }

    /// Sets the model matrix.
    pub fn with_model_matrix(mut self, matrix: DMat4) -> Self {
        self.model_matrix = matrix;
        self
    }

    /// Sets the color.
    pub fn with_color(mut self, color: [f64; 4]) -> Self {
        self.color = color;
        self
    }

    /// Sets the position (translation only).
    pub fn with_position(mut self, position: DVec3) -> Self {
        self.model_matrix = DMat4::from_translation(position);
        self
    }

    /// Computes the world-space bounding sphere.
    pub fn compute_bounding_sphere(&mut self) {
        let local_bs = self.geometry_type.bounding_sphere();
        self.bounding_sphere = Some(local_bs.transform(&self.model_matrix));
    }
}

/// Geometry types that can be instanced.
#[derive(Debug, Clone, PartialEq)]
pub enum GeometryType {
    /// Box geometry.
    Box {
        /// Half-extents.
        half_extents: DVec3,
    },
    /// Sphere geometry.
    Sphere {
        /// Radius.
        radius: f64,
    },
    /// Cylinder geometry.
    Cylinder {
        /// Top radius.
        top_radius: f64,
        /// Bottom radius.
        bottom_radius: f64,
        /// Height.
        height: f64,
    },
    /// Ellipsoid geometry.
    Ellipsoid {
        /// Radii.
        radii: DVec3,
    },
    /// Rectangle geometry (geographic).
    Rectangle {
        /// West (radians).
        west: f64,
        /// South (radians).
        south: f64,
        /// East (radians).
        east: f64,
        /// North (radians).
        north: f64,
    },
    /// Polygon geometry.
    Polygon {
        /// Positions [lon, lat, height] in radians/meters.
        positions: Vec<[f64; 3]>,
    },
    /// Polyline geometry.
    Polyline {
        /// Positions [lon, lat, height] in radians/meters.
        positions: Vec<[f64; 3]>,
        /// Width in meters.
        width: f64,
    },
    /// Circle geometry.
    Circle {
        /// Center [lon, lat, height] in radians/meters.
        center: [f64; 3],
        /// Radius in meters.
        radius: f64,
    },
    /// Custom geometry with vertex data.
    Custom {
        /// Vertex count.
        vertex_count: u32,
        /// Bounding sphere.
        bounding_sphere: BoundingSphere,
    },
}

impl GeometryType {
    /// Returns the local-space bounding sphere for this geometry.
    pub fn bounding_sphere(&self) -> BoundingSphere {
        match self {
            Self::Box { half_extents } => {
                BoundingSphere::new(DVec3::ZERO, half_extents.length())
            }
            Self::Sphere { radius } => BoundingSphere::new(DVec3::ZERO, *radius),
            Self::Cylinder {
                top_radius,
                bottom_radius,
                height,
            } => {
                let max_radius = top_radius.max(*bottom_radius);
                let half_height = height / 2.0;
                BoundingSphere::new(DVec3::ZERO, (max_radius * max_radius + half_height * half_height).sqrt())
            }
            Self::Ellipsoid { radii } => {
                BoundingSphere::new(DVec3::ZERO, radii.x.max(radii.y).max(radii.z))
            }
            Self::Rectangle {
                west,
                south,
                east,
                north,
            } => {
                // Approximate bounding sphere
                let center_lon = (west + east) / 2.0;
                let center_lat = (south + north) / 2.0;
                let angular_radius = ((east - west) / 2.0).max((north - south) / 2.0);
                // Approximate radius on unit sphere
                let radius = angular_radius.sin() * 6378137.0;
                BoundingSphere::new(
                    DVec3::new(center_lon.cos() * center_lat.cos(), center_lon.sin() * center_lat.cos(), center_lat.sin()) * 6378137.0,
                    radius,
                )
            }
            Self::Polygon { positions } | Self::Polyline { positions, .. } => {
                if positions.is_empty() {
                    return BoundingSphere::new(DVec3::ZERO, 0.0);
                }
                // Compute centroid and max distance
                let mut center = DVec3::ZERO;
                for p in positions {
                    center += DVec3::new(
                        p[0].cos() * p[1].cos(),
                        p[0].sin() * p[1].cos(),
                        p[1].sin(),
                    ) * (6378137.0 + p[2]);
                }
                center /= positions.len() as f64;

                let mut max_dist = 0.0f64;
                for p in positions {
                    let pos = DVec3::new(
                        p[0].cos() * p[1].cos(),
                        p[0].sin() * p[1].cos(),
                        p[1].sin(),
                    ) * (6378137.0 + p[2]);
                    max_dist = max_dist.max((pos - center).length());
                }
                BoundingSphere::new(center, max_dist)
            }
            Self::Circle { center, radius } => {
                let pos = DVec3::new(
                    center[0].cos() * center[1].cos(),
                    center[0].sin() * center[1].cos(),
                    center[1].sin(),
                ) * (6378137.0 + center[2]);
                BoundingSphere::new(pos, *radius)
            }
            Self::Custom { bounding_sphere, .. } => *bounding_sphere,
        }
    }

    /// Returns the vertex count estimate for this geometry.
    pub fn estimated_vertex_count(&self) -> u32 {
        match self {
            Self::Box { .. } => 24,
            Self::Sphere { .. } => 1024,
            Self::Cylinder { .. } => 128,
            Self::Ellipsoid { .. } => 1024,
            Self::Rectangle { .. } => 4,
            Self::Polygon { positions } => positions.len() as u32,
            Self::Polyline { positions, .. } => positions.len() as u32 * 2,
            Self::Circle { .. } => 64,
            Self::Custom { vertex_count, .. } => *vertex_count,
        }
    }
}

/// Appearance defines how geometry is rendered.
///
/// Maps to CesiumJS `Scene/Appearance.js`
#[derive(Debug, Clone)]
pub struct Appearance {
    /// Whether the appearance is translucent.
    pub translucent: bool,
    /// Whether to render both faces.
    pub two_sided: bool,
    /// Whether to use flat shading.
    pub flat: bool,
    /// Material type.
    pub material: MaterialType,
    /// Render state.
    pub render_state: RenderState,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            translucent: false,
            two_sided: false,
            flat: false,
            material: MaterialType::Color([1.0, 1.0, 1.0, 1.0]),
            render_state: RenderState::default(),
        }
    }
}

impl Appearance {
    /// Creates a new appearance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a per-instance color appearance.
    pub fn per_instance_color() -> Self {
        Self {
            material: MaterialType::PerInstanceColor,
            ..Default::default()
        }
    }

    /// Sets the material.
    pub fn with_material(mut self, material: MaterialType) -> Self {
        self.material = material;
        self
    }

    /// Sets whether the appearance is translucent.
    pub fn with_translucent(mut self, translucent: bool) -> Self {
        self.translucent = translucent;
        self
    }
}

/// Material types.
#[derive(Debug, Clone, PartialEq)]
pub enum MaterialType {
    /// Solid color.
    Color([f64; 4]),
    /// Use per-instance color.
    PerInstanceColor,
    /// Image texture.
    Image {
        /// Texture URL.
        url: String,
        /// Repeat in X.
        repeat_x: f64,
        /// Repeat in Y.
        repeat_y: f64,
    },
    /// Diffuse map.
    DiffuseMap {
        /// Texture URL.
        url: String,
    },
    /// Normal map.
    NormalMap {
        /// Texture URL.
        url: String,
    },
    /// Grid pattern.
    Grid {
        /// Grid color.
        color: [f64; 4],
        /// Number of cells.
        cells: u32,
    },
    /// Stripe pattern.
    Stripe {
        /// Even color.
        even_color: [f64; 4],
        /// Odd color.
        odd_color: [f64; 4],
        /// Repeat count.
        repeat: f64,
    },
}

/// Render state configuration.
#[derive(Debug, Clone)]
pub struct RenderState {
    /// Whether depth testing is enabled.
    pub depth_test: bool,
    /// Whether depth writing is enabled.
    pub depth_write: bool,
    /// Whether blending is enabled.
    pub blending: bool,
    /// Cull mode.
    pub cull_mode: CullMode,
}

impl Default for RenderState {
    fn default() -> Self {
        Self {
            depth_test: true,
            depth_write: true,
            blending: false,
            cull_mode: CullMode::Back,
        }
    }
}

/// Face culling mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CullMode {
    /// No culling.
    None,
    /// Cull front faces.
    Front,
    /// Cull back faces.
    #[default]
    Back,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geometry_instance_creation() {
        let instance = GeometryInstance::new("test", GeometryType::Sphere { radius: 100.0 });
        assert_eq!(instance.id, "test");
        assert!(instance.show);
        assert_eq!(instance.color, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_geometry_instance_builder() {
        let instance = GeometryInstance::new("test", GeometryType::Box { half_extents: DVec3::ONE })
            .with_color([1.0, 0.0, 0.0, 1.0])
            .with_position(DVec3::new(100.0, 200.0, 300.0));

        assert_eq!(instance.color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(instance.model_matrix.w_axis.truncate(), DVec3::new(100.0, 200.0, 300.0));
    }

    #[test]
    fn test_box_bounding_sphere() {
        let geometry = GeometryType::Box {
            half_extents: DVec3::new(10.0, 20.0, 30.0),
        };
        let bs = geometry.bounding_sphere();
        assert_eq!(bs.center, DVec3::ZERO);
        assert!((bs.radius - DVec3::new(10.0, 20.0, 30.0).length()).abs() < 1e-10);
    }

    #[test]
    fn test_sphere_bounding_sphere() {
        let geometry = GeometryType::Sphere { radius: 50.0 };
        let bs = geometry.bounding_sphere();
        assert_eq!(bs.center, DVec3::ZERO);
        assert_eq!(bs.radius, 50.0);
    }

    #[test]
    fn test_cylinder_bounding_sphere() {
        let geometry = GeometryType::Cylinder {
            top_radius: 10.0,
            bottom_radius: 20.0,
            height: 30.0,
        };
        let bs = geometry.bounding_sphere();
        // max_radius = 20, half_height = 15
        // radius = sqrt(20^2 + 15^2) = sqrt(625) = 25
        assert!((bs.radius - 25.0).abs() < 1e-10);
    }

    #[test]
    fn test_ellipsoid_bounding_sphere() {
        let geometry = GeometryType::Ellipsoid {
            radii: DVec3::new(100.0, 200.0, 150.0),
        };
        let bs = geometry.bounding_sphere();
        assert_eq!(bs.radius, 200.0); // Max of radii
    }

    #[test]
    fn test_polygon_bounding_sphere() {
        let geometry = GeometryType::Polygon {
            positions: vec![
                [0.0, 0.0, 0.0],
                [0.1, 0.0, 0.0],
                [0.1, 0.1, 0.0],
                [0.0, 0.1, 0.0],
            ],
        };
        let bs = geometry.bounding_sphere();
        assert!(bs.radius > 0.0);
    }

    #[test]
    fn test_empty_polygon_bounding_sphere() {
        let geometry = GeometryType::Polygon { positions: vec![] };
        let bs = geometry.bounding_sphere();
        assert_eq!(bs.radius, 0.0);
    }

    #[test]
    fn test_vertex_count_estimates() {
        assert_eq!(GeometryType::Box { half_extents: DVec3::ONE }.estimated_vertex_count(), 24);
        assert_eq!(GeometryType::Sphere { radius: 1.0 }.estimated_vertex_count(), 1024);
        assert_eq!(
            GeometryType::Polyline { positions: vec![[0.0; 3]; 5], width: 1.0 }.estimated_vertex_count(),
            10
        );
    }

    #[test]
    fn test_appearance_default() {
        let appearance = Appearance::default();
        assert!(!appearance.translucent);
        assert!(!appearance.two_sided);
        assert!(!appearance.flat);
    }

    #[test]
    fn test_per_instance_color_appearance() {
        let appearance = Appearance::per_instance_color();
        assert_eq!(appearance.material, MaterialType::PerInstanceColor);
    }

    #[test]
    fn test_material_types() {
        let color = MaterialType::Color([1.0, 0.0, 0.0, 1.0]);
        assert!(matches!(color, MaterialType::Color(_)));

        let grid = MaterialType::Grid {
            color: [0.0, 1.0, 0.0, 1.0],
            cells: 10,
        };
        assert!(matches!(grid, MaterialType::Grid { .. }));
    }

    #[test]
    fn test_render_state_default() {
        let state = RenderState::default();
        assert!(state.depth_test);
        assert!(state.depth_write);
        assert!(!state.blending);
        assert_eq!(state.cull_mode, CullMode::Back);
    }

    #[test]
    fn test_cull_mode_default() {
        assert_eq!(CullMode::default(), CullMode::Back);
    }
}
