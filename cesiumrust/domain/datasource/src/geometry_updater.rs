//! Geometry updater: converts Entity graphics properties into GeometryData.
//!
//! Maps to CesiumJS `DataSources/*GeometryUpdater.js`
//!
//! Each graphics type has an updater function that extracts property values
//! at a given time and produces fill/outline geometry instances.

use glam::DVec3;

use cesium_geospatial::geometry::{
    self, box_geometry, box_outline_geometry, cylinder_geometry, cylinder_outline_geometry,
    ellipse_geometry, ellipse_outline_geometry, plane_geometry, plane_outline_geometry,
    rectangle_geometry, rectangle_outline_geometry, GeometryData, VertexFormat,
};
use cesium_geospatial::geometry::corridor::{corridor_geometry, corridor_outline_geometry, CorridorOptions};
use cesium_geospatial::geometry::ellipse::EllipseOptions;
use cesium_geospatial::geometry::polyline_geo::{polyline_geometry, PolylineOptions};
use cesium_geospatial::geometry::polyline_volume::{polyline_volume_geometry, PolylineVolumeOptions};
use cesium_geospatial::geometry::wall::{wall_geometry, wall_outline_geometry, WallOptions};
use cesium_geospatial::{Cartographic, Ellipsoid, Rectangle};

use crate::entity::{
    BoxGraphics, CorridorGraphics, CylinderGraphics, EllipseGraphics, EllipsoidGraphics,
    Entity, PlaneGraphics, PolylineGraphics,
    PolylineVolumeGraphics, RectangleGraphics, WallGraphics,
};
use crate::property::Color;

/// A geometry instance ready for rendering.
///
/// Maps to CesiumJS `Core/GeometryInstance.js`
#[derive(Debug, Clone)]
pub struct GeometryInstance {
    /// The geometry data (positions, indices, normals, etc.).
    pub geometry: GeometryData,
    /// Model matrix (4x4 column-major, f64).
    pub model_matrix: [f64; 16],
    /// Fill color (RGBA 0..1).
    pub color: Color,
    /// Whether this is an outline instance.
    pub is_outline: bool,
    /// Entity ID that produced this instance.
    pub entity_id: String,
}

impl GeometryInstance {
    /// Creates a new geometry instance with identity model matrix.
    pub fn new(geometry: GeometryData, color: Color, is_outline: bool, entity_id: String) -> Self {
        Self {
            geometry,
            model_matrix: [
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 1.0,
            ],
            color,
            is_outline,
            entity_id,
        }
    }

    /// Sets the model matrix from a translation (position in Cartesian3).
    pub fn with_translation(mut self, translation: DVec3) -> Self {
        // Column-major 4x4 with translation in last column
        self.model_matrix[12] = translation.x;
        self.model_matrix[13] = translation.y;
        self.model_matrix[14] = translation.z;
        self
    }
}

/// Result of updating an entity's geometry at a given time.
#[derive(Debug, Clone, Default)]
pub struct EntityGeometry {
    /// Fill geometry instances.
    pub fill_instances: Vec<GeometryInstance>,
    /// Outline geometry instances.
    pub outline_instances: Vec<GeometryInstance>,
}

impl EntityGeometry {
    /// Returns true if there are no geometry instances.
    pub fn is_empty(&self) -> bool {
        self.fill_instances.is_empty() && self.outline_instances.is_empty()
    }

    /// Total number of instances.
    pub fn instance_count(&self) -> usize {
        self.fill_instances.len() + self.outline_instances.len()
    }
}

/// Converts a cartographic position [lon_rad, lat_rad, height_m] to Cartesian3.
pub fn cartographic_to_cartesian(pos: &[f64; 3], ellipsoid: &Ellipsoid) -> DVec3 {
    let carto = Cartographic::from_radians(pos[0], pos[1], pos[2]);
    ellipsoid.cartographic_to_cartesian(&carto)
}

/// Converts an array of cartographic positions to Cartesian3.
pub fn positions_to_cartesian(positions: &[[f64; 3]], ellipsoid: &Ellipsoid) -> Vec<DVec3> {
    positions
        .iter()
        .map(|p| cartographic_to_cartesian(p, ellipsoid))
        .collect()
}

/// Updates box graphics for an entity at the given time.
///
/// Maps to CesiumJS `DataSources/BoxGeometryUpdater.js`
pub fn update_box_graphics(
    entity: &Entity,
    graphics: &BoxGraphics,
    time: f64,
    ellipsoid: &Ellipsoid,
) -> EntityGeometry {
    let mut result = EntityGeometry::default();

    let show = graphics.show.get_value(time).copied().unwrap_or(true);
    if !show {
        return result;
    }

    let dimensions = match graphics.dimensions.get_value(time) {
        Some(d) => *d,
        None => return result,
    };

    let position = match entity.position.get_value(time) {
        Some(p) => cartographic_to_cartesian(p, ellipsoid),
        None => DVec3::ZERO,
    };

    let half = DVec3::new(dimensions[0] / 2.0, dimensions[1] / 2.0, dimensions[2] / 2.0);
    let vf = VertexFormat::ALL;

    let fill = graphics.fill.get_value(time).copied().unwrap_or(true);
    if fill {
        let color = graphics.material.get_value(time).copied().unwrap_or(Color::WHITE);
        let geo = box_geometry(-half, half, vf);
        result.fill_instances.push(
            GeometryInstance::new(geo, color, false, entity.id.clone())
                .with_translation(position),
        );
    }

    let outline = graphics.outline.get_value(time).copied().unwrap_or(false);
    if outline {
        let outline_color = graphics.outline_color.get_value(time).copied().unwrap_or(Color::BLACK);
        let geo = box_outline_geometry(-half, half);
        result.outline_instances.push(
            GeometryInstance::new(geo, outline_color, true, entity.id.clone())
                .with_translation(position),
        );
    }

    result
}

/// Updates cylinder graphics for an entity at the given time.
///
/// Maps to CesiumJS `DataSources/CylinderGeometryUpdater.js`
pub fn update_cylinder_graphics(
    entity: &Entity,
    graphics: &CylinderGraphics,
    time: f64,
    ellipsoid: &Ellipsoid,
) -> EntityGeometry {
    let mut result = EntityGeometry::default();

    let show = graphics.show.get_value(time).copied().unwrap_or(true);
    if !show {
        return result;
    }

    let length = match graphics.length.get_value(time) {
        Some(&l) => l,
        None => return result,
    };
    let top_radius = graphics.top_radius.get_value(time).copied().unwrap_or(0.0);
    let bottom_radius = graphics.bottom_radius.get_value(time).copied().unwrap_or(0.0);

    let position = match entity.position.get_value(time) {
        Some(p) => cartographic_to_cartesian(p, ellipsoid),
        None => DVec3::ZERO,
    };

    let slices = graphics.slices.get_value(time).copied().unwrap_or(128.0) as u32;
    let vf = VertexFormat::ALL;

    let fill = graphics.fill.get_value(time).copied().unwrap_or(true);
    if fill {
        let color = graphics.material.get_value(time).copied().unwrap_or(Color::WHITE);
        let geo = cylinder_geometry(length, top_radius, bottom_radius, slices, vf);
        result.fill_instances.push(
            GeometryInstance::new(geo, color, false, entity.id.clone())
                .with_translation(position),
        );
    }

    let outline = graphics.outline.get_value(time).copied().unwrap_or(false);
    if outline {
        let outline_color = graphics.outline_color.get_value(time).copied().unwrap_or(Color::BLACK);
        let geo = cylinder_outline_geometry(length, top_radius, bottom_radius, slices);
        result.outline_instances.push(
            GeometryInstance::new(geo, outline_color, true, entity.id.clone())
                .with_translation(position),
        );
    }

    result
}

/// Updates ellipse graphics for an entity at the given time.
///
/// Maps to CesiumJS `DataSources/EllipseGeometryUpdater.js`
pub fn update_ellipse_graphics(
    entity: &Entity,
    graphics: &EllipseGraphics,
    time: f64,
    ellipsoid: &Ellipsoid,
) -> EntityGeometry {
    let mut result = EntityGeometry::default();

    let show = graphics.show.get_value(time).copied().unwrap_or(true);
    if !show {
        return result;
    }

    let semi_major = match graphics.semi_major_axis.get_value(time) {
        Some(&v) => v,
        None => return result,
    };
    let semi_minor = match graphics.semi_minor_axis.get_value(time) {
        Some(&v) => v,
        None => return result,
    };

    let position = match entity.position.get_value(time) {
        Some(p) => cartographic_to_cartesian(p, ellipsoid),
        None => DVec3::ZERO,
    };

    let height = graphics.height.get_value(time).copied().unwrap_or(0.0);
    let rotation = graphics.rotation.get_value(time).copied().unwrap_or(0.0);
    let vf = VertexFormat::ALL;

    let options = EllipseOptions {
        center: position,
        semi_major_axis: semi_major,
        semi_minor_axis: semi_minor,
        height,
        rotation,
        st_rotation: 0.0,
        granularity: std::f64::consts::PI / 180.0,
        ellipsoid: *ellipsoid,
    };

    let fill = graphics.fill.get_value(time).copied().unwrap_or(true);
    if fill {
        let color = graphics.material.get_value(time).copied().unwrap_or(Color::WHITE);
        let geo = ellipse_geometry(&options, vf);
        result.fill_instances.push(
            GeometryInstance::new(geo, color, false, entity.id.clone()),
        );
    }

    let outline = graphics.outline.get_value(time).copied().unwrap_or(false);
    if outline {
        let outline_color = graphics.outline_color.get_value(time).copied().unwrap_or(Color::BLACK);
        let geo = ellipse_outline_geometry(&options);
        result.outline_instances.push(
            GeometryInstance::new(geo, outline_color, true, entity.id.clone()),
        );
    }

    result
}

/// Updates corridor graphics for an entity at the given time.
///
/// Maps to CesiumJS `DataSources/CorridorGeometryUpdater.js`
pub fn update_corridor_graphics(
    entity: &Entity,
    graphics: &CorridorGraphics,
    time: f64,
    ellipsoid: &Ellipsoid,
) -> EntityGeometry {
    let mut result = EntityGeometry::default();

    let show = graphics.show.get_value(time).copied().unwrap_or(true);
    if !show {
        return result;
    }

    let positions_raw = match graphics.positions.get_value(time) {
        Some(p) => p,
        None => return result,
    };
    let width = match graphics.width.get_value(time) {
        Some(&w) => w,
        None => return result,
    };

    let positions = positions_to_cartesian(positions_raw, ellipsoid);
    if positions.len() < 2 {
        return result;
    }

    let height = graphics.height.get_value(time).copied().unwrap_or(0.0);
    let granularity = graphics.granularity.get_value(time).copied().unwrap_or(std::f64::consts::PI / 180.0);
    let corner_type = match graphics.corner_type {
        crate::entity::CornerType::Rounded => cesium_geospatial::geometry::corridor::CornerType::Rounded,
        crate::entity::CornerType::Mitered => cesium_geospatial::geometry::corridor::CornerType::Mitered,
        crate::entity::CornerType::Beveled => cesium_geospatial::geometry::corridor::CornerType::Beveled,
    };

    let options = CorridorOptions {
        positions,
        width,
        height,
        granularity,
        corner_type,
        ellipsoid: *ellipsoid,
    };

    let vf = VertexFormat::ALL;

    let fill = graphics.fill.get_value(time).copied().unwrap_or(true);
    if fill {
        let color = graphics.material.get_value(time).copied().unwrap_or(Color::WHITE);
        let geo = corridor_geometry(&options, vf);
        result.fill_instances.push(
            GeometryInstance::new(geo, color, false, entity.id.clone()),
        );
    }

    let outline = graphics.outline.get_value(time).copied().unwrap_or(false);
    if outline {
        let outline_color = graphics.outline_color.get_value(time).copied().unwrap_or(Color::BLACK);
        let geo = corridor_outline_geometry(&options);
        result.outline_instances.push(
            GeometryInstance::new(geo, outline_color, true, entity.id.clone()),
        );
    }

    result
}

/// Updates rectangle graphics for an entity at the given time.
///
/// Maps to CesiumJS `DataSources/RectangleGeometryUpdater.js`
pub fn update_rectangle_graphics(
    _entity: &Entity,
    graphics: &RectangleGraphics,
    time: f64,
    ellipsoid: &Ellipsoid,
) -> EntityGeometry {
    let mut result = EntityGeometry::default();

    let show = graphics.show.get_value(time).copied().unwrap_or(true);
    if !show {
        return result;
    }

    let coords = match graphics.coordinates.get_value(time) {
        Some(c) => *c,
        None => return result,
    };

    let rect = Rectangle::new(coords[0], coords[1], coords[2], coords[3]);
    let height = graphics.height.get_value(time).copied().unwrap_or(0.0);
    let granularity = graphics.granularity.get_value(time).copied().unwrap_or(std::f64::consts::PI / 180.0);
    let vf = VertexFormat::ALL;

    let fill = graphics.fill.get_value(time).copied().unwrap_or(true);
    if fill {
        let color = graphics.material.get_value(time).copied().unwrap_or(Color::WHITE);
        let geo = rectangle_geometry(&rect, ellipsoid, granularity, height, vf);
        result.fill_instances.push(
            GeometryInstance::new(geo, color, false, _entity.id.clone()),
        );
    }

    let outline = graphics.outline.get_value(time).copied().unwrap_or(false);
    if outline {
        let outline_color = graphics.outline_color.get_value(time).copied().unwrap_or(Color::BLACK);
        let geo = rectangle_outline_geometry(&rect, ellipsoid, granularity);
        result.outline_instances.push(
            GeometryInstance::new(geo, outline_color, true, _entity.id.clone()),
        );
    }

    result
}

/// Updates wall graphics for an entity at the given time.
///
/// Maps to CesiumJS `DataSources/WallGeometryUpdater.js`
pub fn update_wall_graphics(
    entity: &Entity,
    graphics: &WallGraphics,
    time: f64,
    ellipsoid: &Ellipsoid,
) -> EntityGeometry {
    let mut result = EntityGeometry::default();

    let show = graphics.show.get_value(time).copied().unwrap_or(true);
    if !show {
        return result;
    }

    let positions_raw = match graphics.positions.get_value(time) {
        Some(p) => p,
        None => return result,
    };

    let positions = positions_to_cartesian(positions_raw, ellipsoid);
    if positions.len() < 2 {
        return result;
    }

    let minimum_heights = graphics.minimum_heights.get_value(time).cloned();
    let maximum_heights = graphics.maximum_heights.get_value(time).cloned();
    let granularity = graphics.granularity.get_value(time).copied().unwrap_or(std::f64::consts::PI / 180.0);
    let vf = VertexFormat::ALL;

    let options = WallOptions {
        positions,
        minimum_heights,
        maximum_heights,
        granularity,
        ellipsoid: *ellipsoid,
    };

    let fill = graphics.fill.get_value(time).copied().unwrap_or(true);
    if fill {
        let color = graphics.material.get_value(time).copied().unwrap_or(Color::WHITE);
        let geo = wall_geometry(&options, vf);
        result.fill_instances.push(
            GeometryInstance::new(geo, color, false, entity.id.clone()),
        );
    }

    let outline = graphics.outline.get_value(time).copied().unwrap_or(false);
    if outline {
        let outline_color = graphics.outline_color.get_value(time).copied().unwrap_or(Color::BLACK);
        let geo = wall_outline_geometry(&options);
        result.outline_instances.push(
            GeometryInstance::new(geo, outline_color, true, entity.id.clone()),
        );
    }

    result
}

/// Updates ellipsoid graphics for an entity at the given time.
///
/// Maps to CesiumJS `DataSources/EllipsoidGeometryUpdater.js`
pub fn update_ellipsoid_graphics(
    entity: &Entity,
    graphics: &EllipsoidGraphics,
    time: f64,
    ellipsoid: &Ellipsoid,
) -> EntityGeometry {
    let mut result = EntityGeometry::default();

    let show = graphics.show.get_value(time).copied().unwrap_or(true);
    if !show {
        return result;
    }

    let radii = match graphics.radii.get_value(time) {
        Some(r) => *r,
        None => return result,
    };

    let position = match entity.position.get_value(time) {
        Some(p) => cartographic_to_cartesian(p, ellipsoid),
        None => DVec3::ZERO,
    };

    let slices = graphics.slices.get_value(time).copied().unwrap_or(128.0) as u32;
    let stack_partitions = graphics.stack_partitions.get_value(time).copied().unwrap_or(64.0) as u32;
    let vf = VertexFormat::ALL;

    let fill = graphics.fill.get_value(time).copied().unwrap_or(true);
    if fill {
        let color = graphics.material.get_value(time).copied().unwrap_or(Color::WHITE);
        let geo = geometry::ellipsoid_geometry(
            DVec3::from(radii),
            stack_partitions,
            slices,
            vf,
        );
        result.fill_instances.push(
            GeometryInstance::new(geo, color, false, entity.id.clone())
                .with_translation(position),
        );
    }

    let outline = graphics.outline.get_value(time).copied().unwrap_or(false);
    if outline {
        let outline_color = graphics.outline_color.get_value(time).copied().unwrap_or(Color::BLACK);
        let geo = geometry::ellipsoid_outline_geometry(
            DVec3::from(radii),
            stack_partitions,
            slices,
        );
        result.outline_instances.push(
            GeometryInstance::new(geo, outline_color, true, entity.id.clone())
                .with_translation(position),
        );
    }

    result
}

/// Updates plane graphics for an entity at the given time.
///
/// Maps to CesiumJS `DataSources/PlaneGeometryUpdater.js`
pub fn update_plane_graphics(
    entity: &Entity,
    graphics: &PlaneGraphics,
    time: f64,
    ellipsoid: &Ellipsoid,
) -> EntityGeometry {
    let mut result = EntityGeometry::default();

    let show = graphics.show.get_value(time).copied().unwrap_or(true);
    if !show {
        return result;
    }

    let _plane_def = match graphics.plane.get_value(time) {
        Some(p) => p,
        None => return result,
    };

    let _dimensions = match graphics.dimensions.get_value(time) {
        Some(d) => *d,
        None => return result,
    };

    let position = match entity.position.get_value(time) {
        Some(p) => cartographic_to_cartesian(p, ellipsoid),
        None => DVec3::ZERO,
    };

    let vf = VertexFormat::ALL;

    let fill = graphics.fill.get_value(time).copied().unwrap_or(true);
    if fill {
        let color = graphics.material.get_value(time).copied().unwrap_or(Color::WHITE);
        let geo = plane_geometry(vf);
        result.fill_instances.push(
            GeometryInstance::new(geo, color, false, entity.id.clone())
                .with_translation(position),
        );
    }

    let outline = graphics.outline.get_value(time).copied().unwrap_or(false);
    if outline {
        let outline_color = graphics.outline_color.get_value(time).copied().unwrap_or(Color::BLACK);
        let geo = plane_outline_geometry();
        result.outline_instances.push(
            GeometryInstance::new(geo, outline_color, true, entity.id.clone())
                .with_translation(position),
        );
    }

    result
}

/// Updates polyline graphics for an entity at the given time.
///
/// Maps to CesiumJS `DataSources/PolylineGeometryUpdater.js`
pub fn update_polyline_graphics(
    entity: &Entity,
    graphics: &PolylineGraphics,
    time: f64,
    ellipsoid: &Ellipsoid,
) -> EntityGeometry {
    let mut result = EntityGeometry::default();

    let show = graphics.show.get_value(time).copied().unwrap_or(true);
    if !show {
        return result;
    }

    let positions_raw = match graphics.positions.get_value(time) {
        Some(p) => p,
        None => return result,
    };

    let positions = positions_to_cartesian(positions_raw, ellipsoid);
    if positions.len() < 2 {
        return result;
    }

    let width = graphics.width.get_value(time).copied().unwrap_or(1.0);
    let color = graphics.color.get_value(time).copied().unwrap_or(Color::WHITE);
    let granularity = std::f64::consts::PI / 180.0;
    let vf = VertexFormat::ALL;

    let options = PolylineOptions {
        positions,
        width,
        granularity,
        ellipsoid: *ellipsoid,
    };

    let geo = polyline_geometry(&options, vf);
    result.fill_instances.push(
        GeometryInstance::new(geo, color, false, entity.id.clone()),
    );

    result
}

/// Updates polyline volume graphics for an entity at the given time.
///
/// Maps to CesiumJS `DataSources/PolylineVolumeGeometryUpdater.js`
pub fn update_polyline_volume_graphics(
    entity: &Entity,
    graphics: &PolylineVolumeGraphics,
    time: f64,
    ellipsoid: &Ellipsoid,
) -> EntityGeometry {
    let mut result = EntityGeometry::default();

    let show = graphics.show.get_value(time).copied().unwrap_or(true);
    if !show {
        return result;
    }

    let positions_raw = match graphics.positions.get_value(time) {
        Some(p) => p,
        None => return result,
    };
    let shape = match graphics.shape.get_value(time) {
        Some(s) => s.clone(),
        None => return result,
    };

    let positions = positions_to_cartesian(positions_raw, ellipsoid);
    if positions.len() < 2 {
        return result;
    }

    let granularity = graphics.granularity.get_value(time).copied().unwrap_or(std::f64::consts::PI / 180.0);
    let vf = VertexFormat::ALL;

    let options = PolylineVolumeOptions {
        positions,
        shape,
        granularity,
        ellipsoid: *ellipsoid,
    };

    let fill = graphics.fill.get_value(time).copied().unwrap_or(true);
    if fill {
        let color = graphics.material.get_value(time).copied().unwrap_or(Color::WHITE);
        let geo = polyline_volume_geometry(&options, vf);
        result.fill_instances.push(
            GeometryInstance::new(geo, color, false, entity.id.clone()),
        );
    }

    result
}

/// Updates all geometry graphics for an entity at the given time.
///
/// This is the main entry point that dispatches to the appropriate updater
/// based on which graphics are defined on the entity.
pub fn update_entity_geometry(
    entity: &Entity,
    time: f64,
    ellipsoid: &Ellipsoid,
) -> EntityGeometry {
    let mut result = EntityGeometry::default();

    if !entity.show {
        return result;
    }

    if let Some(ref graphics) = entity.box_graphics {
        let geo = update_box_graphics(entity, graphics, time, ellipsoid);
        result.fill_instances.extend(geo.fill_instances);
        result.outline_instances.extend(geo.outline_instances);
    }

    if let Some(ref graphics) = entity.cylinder {
        let geo = update_cylinder_graphics(entity, graphics, time, ellipsoid);
        result.fill_instances.extend(geo.fill_instances);
        result.outline_instances.extend(geo.outline_instances);
    }

    if let Some(ref graphics) = entity.ellipse {
        let geo = update_ellipse_graphics(entity, graphics, time, ellipsoid);
        result.fill_instances.extend(geo.fill_instances);
        result.outline_instances.extend(geo.outline_instances);
    }

    if let Some(ref graphics) = entity.corridor {
        let geo = update_corridor_graphics(entity, graphics, time, ellipsoid);
        result.fill_instances.extend(geo.fill_instances);
        result.outline_instances.extend(geo.outline_instances);
    }

    if let Some(ref graphics) = entity.rectangle {
        let geo = update_rectangle_graphics(entity, graphics, time, ellipsoid);
        result.fill_instances.extend(geo.fill_instances);
        result.outline_instances.extend(geo.outline_instances);
    }

    if let Some(ref graphics) = entity.wall {
        let geo = update_wall_graphics(entity, graphics, time, ellipsoid);
        result.fill_instances.extend(geo.fill_instances);
        result.outline_instances.extend(geo.outline_instances);
    }

    if let Some(ref graphics) = entity.ellipsoid {
        let geo = update_ellipsoid_graphics(entity, graphics, time, ellipsoid);
        result.fill_instances.extend(geo.fill_instances);
        result.outline_instances.extend(geo.outline_instances);
    }

    if let Some(ref graphics) = entity.plane {
        let geo = update_plane_graphics(entity, graphics, time, ellipsoid);
        result.fill_instances.extend(geo.fill_instances);
        result.outline_instances.extend(geo.outline_instances);
    }

    if let Some(ref graphics) = entity.polyline {
        let geo = update_polyline_graphics(entity, graphics, time, ellipsoid);
        result.fill_instances.extend(geo.fill_instances);
        result.outline_instances.extend(geo.outline_instances);
    }

    if let Some(ref graphics) = entity.polyline_volume {
        let geo = update_polyline_volume_graphics(entity, graphics, time, ellipsoid);
        result.fill_instances.extend(geo.fill_instances);
        result.outline_instances.extend(geo.outline_instances);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::*;
    use crate::property::Property;

    fn wgs84() -> Ellipsoid {
        Ellipsoid::WGS84
    }

    #[test]
    fn test_update_box_graphics() {
        let entity = Entity::new("box-1")
            .with_position(0.0, 0.0, 0.0)
            .with_box(BoxGraphics {
                dimensions: Property::Constant([100.0, 200.0, 300.0]),
                ..Default::default()
            });

        let result = update_entity_geometry(&entity, 0.0, &wgs84());
        assert_eq!(result.fill_instances.len(), 1);
        assert_eq!(result.outline_instances.len(), 0);
        assert!(!result.fill_instances[0].geometry.positions.is_empty());
    }

    #[test]
    fn test_update_box_with_outline() {
        let entity = Entity::new("box-2")
            .with_position(0.0, 0.0, 0.0)
            .with_box(BoxGraphics {
                dimensions: Property::Constant([100.0, 100.0, 100.0]),
                outline: Property::Constant(true),
                ..Default::default()
            });

        let result = update_entity_geometry(&entity, 0.0, &wgs84());
        assert_eq!(result.fill_instances.len(), 1);
        assert_eq!(result.outline_instances.len(), 1);
        assert!(result.outline_instances[0].is_outline);
    }

    #[test]
    fn test_update_cylinder_graphics() {
        let entity = Entity::new("cyl-1")
            .with_position(0.0, 0.0, 0.0)
            .with_cylinder(CylinderGraphics {
                length: Property::Constant(400.0),
                top_radius: Property::Constant(100.0),
                bottom_radius: Property::Constant(100.0),
                ..Default::default()
            });

        let result = update_entity_geometry(&entity, 0.0, &wgs84());
        assert_eq!(result.fill_instances.len(), 1);
        assert!(!result.fill_instances[0].geometry.positions.is_empty());
    }

    #[test]
    fn test_update_ellipse_graphics() {
        let entity = Entity::new("ell-1")
            .with_position(0.0, 0.0, 0.0)
            .with_ellipsoid(EllipsoidGraphics {
                radii: Property::Constant([500000.0, 300000.0, 200000.0]),
                ..Default::default()
            });

        let result = update_entity_geometry(&entity, 0.0, &wgs84());
        assert_eq!(result.fill_instances.len(), 1);
    }

    #[test]
    fn test_update_corridor_graphics() {
        let entity = Entity::new("cor-1").with_corridor(CorridorGraphics {
            positions: Property::Constant(vec![
                [0.0, 0.0, 0.0],
                [0.05, 0.05, 0.0],
                [0.1, 0.0, 0.0],
            ]),
            width: Property::Constant(100000.0),
            ..Default::default()
        });

        let result = update_entity_geometry(&entity, 0.0, &wgs84());
        assert_eq!(result.fill_instances.len(), 1);
    }

    #[test]
    fn test_update_wall_graphics() {
        let entity = Entity::new("wall-1").with_wall(WallGraphics {
            positions: Property::Constant(vec![
                [0.0, 0.0, 0.0],
                [0.05, 0.0, 0.0],
                [0.1, 0.0, 0.0],
            ]),
            maximum_heights: Property::Constant(vec![100000.0, 100000.0, 100000.0]),
            ..Default::default()
        });

        let result = update_entity_geometry(&entity, 0.0, &wgs84());
        assert_eq!(result.fill_instances.len(), 1);
    }

    #[test]
    fn test_update_polyline_graphics() {
        let entity = Entity::new("line-1").with_polyline(PolylineGraphics {
            positions: Property::Constant(vec![
                [0.0, 0.0, 0.0],
                [0.05, 0.05, 0.0],
                [0.1, 0.0, 0.0],
            ]),
            width: Property::Constant(5.0),
            ..Default::default()
        });

        let result = update_entity_geometry(&entity, 0.0, &wgs84());
        assert_eq!(result.fill_instances.len(), 1);
    }

    #[test]
    fn test_hidden_entity_returns_empty() {
        let mut entity = Entity::new("hidden-1")
            .with_box(BoxGraphics {
                dimensions: Property::Constant([100.0, 100.0, 100.0]),
                ..Default::default()
            });
        entity.show = false;

        let result = update_entity_geometry(&entity, 0.0, &wgs84());
        assert!(result.is_empty());
    }

    #[test]
    fn test_show_false_returns_empty() {
        let entity = Entity::new("box-noshow")
            .with_position(0.0, 0.0, 0.0)
            .with_box(BoxGraphics {
                dimensions: Property::Constant([100.0, 100.0, 100.0]),
                show: Property::Constant(false),
                ..Default::default()
            });

        let result = update_entity_geometry(&entity, 0.0, &wgs84());
        assert!(result.is_empty());
    }

    #[test]
    fn test_geometry_instance_translation() {
        let geo = box_geometry(DVec3::splat(-1.0), DVec3::ONE, VertexFormat::ALL);
        let instance = GeometryInstance::new(geo, Color::RED, false, "test".to_string())
            .with_translation(DVec3::new(100.0, 200.0, 300.0));

        assert!((instance.model_matrix[12] - 100.0).abs() < 1e-10);
        assert!((instance.model_matrix[13] - 200.0).abs() < 1e-10);
        assert!((instance.model_matrix[14] - 300.0).abs() < 1e-10);
    }

    #[test]
    fn test_update_polyline_volume_graphics() {
        let entity = Entity::new("pv-1").with_polyline_volume(PolylineVolumeGraphics {
            positions: Property::Constant(vec![
                [0.0, 0.0, 0.0],
                [0.05, 0.0, 0.0],
                [0.1, 0.0, 0.0],
            ]),
            shape: Property::Constant(vec![
                [-5000.0, -5000.0],
                [5000.0, -5000.0],
                [5000.0, 5000.0],
                [-5000.0, 5000.0],
            ]),
            ..Default::default()
        });

        let result = update_entity_geometry(&entity, 0.0, &wgs84());
        assert_eq!(result.fill_instances.len(), 1);
    }
}
