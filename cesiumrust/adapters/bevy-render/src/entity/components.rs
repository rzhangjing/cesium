//! Entity-specific Bevy components.
//!
//! These components map to CesiumJS entity types (PointGraphics,
//! PolylineGraphics, PolygonGraphics, BillboardGraphics, ModelGraphics).
//! Domain types from `cesium_datasource` remain the source of truth;
//! these are used for GPU-ready rendering state.

use bevy::prelude::*;
use cesium_datasource::entity::Entity as DomainEntity;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_time::TimeIntervalCollection;

/// Resource wrapping the globe ellipsoid.
#[derive(Resource, Deref, DerefMut)]
pub struct GlobeEllipsoid(pub Ellipsoid);

impl Default for GlobeEllipsoid {
    fn default() -> Self {
        Self(Ellipsoid::WGS84)
    }
}

#[derive(Component, Deref, DerefMut, Clone)]
pub struct EntityWrapper(pub DomainEntity);

impl EntityWrapper {
    pub fn new(entity: DomainEntity) -> Self {
        Self(entity)
    }
}

#[derive(Component, Clone)]
pub struct CesiumEntity {
    pub entity_id: String,
    pub name: String,
    pub description: Option<String>,
    pub show: bool,
    pub availability: Option<TimeIntervalCollection<()>>,
}

impl CesiumEntity {
    pub fn new(entity_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            entity_id: entity_id.into(),
            name: name.into(),
            description: None,
            show: true,
            availability: None,
        }
    }
}

/// Marker for entities that need their visualization created/updated.
#[derive(Component)]
pub struct NeedsVisualUpdate;

/// Marker for entities whose visualization has been built.
#[derive(Component)]
pub struct VisualizationBuilt;

/// Marker for billboard entities (face camera each frame).
#[derive(Component)]
pub struct BillboardTag;

#[derive(Component, Clone)]
pub struct PointGraphicsComponent {
    pub pixel_size: f32,
    pub color: [f32; 4],
    pub outline_color: [f32; 4],
    pub outline_width: f32,
}

impl Default for PointGraphicsComponent {
    fn default() -> Self {
        Self {
            pixel_size: 1.0,
            color: [1.0, 1.0, 1.0, 1.0],
            outline_color: [0.0, 0.0, 0.0, 1.0],
            outline_width: 0.0,
        }
    }
}

#[derive(Component, Clone)]
pub struct PolylineGraphicsComponent {
    pub positions: Vec<glam::DVec3>,
    pub width: f32,
    pub material_color: [f32; 4],
    pub clamp_to_ground: bool,
}

impl Default for PolylineGraphicsComponent {
    fn default() -> Self {
        Self {
            positions: Vec::new(),
            width: 1.0,
            material_color: [1.0, 1.0, 1.0, 1.0],
            clamp_to_ground: false,
        }
    }
}

#[derive(Component, Clone)]
pub struct PolygonGraphicsComponent {
    pub positions: Vec<glam::DVec3>,
    pub holes: Vec<Vec<glam::DVec3>>,
    pub height: f64,
    pub extruded_height: Option<f64>,
    pub material_color: [f32; 4],
    pub outline: bool,
    pub outline_color: [f32; 4],
}

impl Default for PolygonGraphicsComponent {
    fn default() -> Self {
        Self {
            positions: Vec::new(),
            holes: Vec::new(),
            height: 0.0,
            extruded_height: None,
            material_color: [1.0, 1.0, 1.0, 1.0],
            outline: false,
            outline_color: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

#[derive(Component, Clone)]
pub struct BillboardGraphicsComponent {
    pub image_url: Option<String>,
    pub scale: f32,
    pub color: [f32; 4],
}

impl Default for BillboardGraphicsComponent {
    fn default() -> Self {
        Self {
            image_url: None,
            scale: 1.0,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

#[derive(Component, Clone)]
pub struct ModelGraphicsComponent {
    pub uri: String,
    pub scale: f32,
    pub minimum_pixel_size: f32,
}

impl Default for ModelGraphicsComponent {
    fn default() -> Self {
        Self {
            uri: String::new(),
            scale: 1.0,
            minimum_pixel_size: 0.0,
        }
    }
}

#[derive(Component)]
pub struct TimeDynamicProperties {
    pub has_interpolated_position: bool,
    pub has_interpolated_color: bool,
    pub has_interpolated_orientation: bool,
    pub has_availability: bool,
}

impl Default for TimeDynamicProperties {
    fn default() -> Self {
        Self {
            has_interpolated_position: false,
            has_interpolated_color: false,
            has_interpolated_orientation: false,
            has_availability: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cesium_entity_defaults() {
        let entity = CesiumEntity::new("test-01", "Test Entity");
        assert_eq!(entity.entity_id, "test-01");
        assert_eq!(entity.name, "Test Entity");
        assert!(entity.show);
        assert!(entity.description.is_none());
        assert!(entity.availability.is_none());
    }

    #[test]
    fn test_point_graphics_defaults() {
        let pg = PointGraphicsComponent::default();
        assert!((pg.pixel_size - 1.0).abs() < 1e-6);
        assert_eq!(pg.color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(pg.outline_color, [0.0, 0.0, 0.0, 1.0]);
        assert!((pg.outline_width - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_polygon_graphics_with_holes() {
        let mut pg = PolygonGraphicsComponent::default();
        pg.holes.push(vec![glam::DVec3::new(0.0, 0.0, 0.0)]);
        assert_eq!(pg.holes.len(), 1);
    }
}
