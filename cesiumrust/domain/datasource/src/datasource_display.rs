//! DataSourceDisplay: central coordinator for entity visualization.
//!
//! Maps to CesiumJS `DataSources/DataSourceDisplay.js`
//!
//! Coordinates all visualizers (geometry, billboard, label, point, model, path)
//! for a collection of data sources, updating them each frame.

use cesium_geospatial::Ellipsoid;

use crate::entity_collection::{DataSource, EntityCollection};
use crate::geometry_updater::EntityGeometry;
use crate::primitives::{
    Billboard, BillboardCollection, Label, LabelCollection, PointPrimitive,
    PointPrimitiveCollection,
};
use crate::visualizer::GeometryVisualizer;

/// The display state of all data sources.
///
/// Maps to CesiumJS `DataSources/DataSourceDisplay.js`
#[derive(Debug)]
pub struct DataSourceDisplay {
    /// The geometry visualizer.
    geometry_visualizer: GeometryVisualizer,
    /// Billboard collection for all entities.
    pub billboards: BillboardCollection,
    /// Label collection for all entities.
    pub labels: LabelCollection,
    /// Point primitive collection for all entities.
    pub points: PointPrimitiveCollection,
    /// The ellipsoid used for coordinate conversion.
    ellipsoid: Ellipsoid,
    /// Whether the display has been initialized.
    initialized: bool,
    /// Last update time.
    last_time: f64,
}

impl DataSourceDisplay {
    /// Creates a new data source display.
    pub fn new(ellipsoid: Ellipsoid) -> Self {
        Self {
            geometry_visualizer: GeometryVisualizer::new(ellipsoid),
            billboards: BillboardCollection::new(),
            labels: LabelCollection::new(),
            points: PointPrimitiveCollection::new(),
            ellipsoid,
            initialized: false,
            last_time: 0.0,
        }
    }

    /// Creates a new data source display with WGS84 ellipsoid.
    pub fn wgs84() -> Self {
        Self::new(Ellipsoid::WGS84)
    }

    /// Updates the display for the given entity collection at the given time.
    ///
    /// This is the main per-frame update method. It:
    /// 1. Updates geometry visualizer
    /// 2. Syncs billboard/label/point collections with entities
    pub fn update(&mut self, entities: &EntityCollection, time: f64) {
        // Update geometry
        self.geometry_visualizer.update(entities, time);

        // Sync billboard/label/point collections
        if !self.initialized || (time - self.last_time).abs() > f64::EPSILON {
            self.sync_primitives(entities, time);
            self.initialized = true;
        }

        self.last_time = time;
    }

    /// Syncs billboard, label, and point collections with entity graphics.
    fn sync_primitives(&mut self, entities: &EntityCollection, time: f64) {
        self.billboards.clear();
        self.labels.clear();
        self.points.clear();

        for entity in entities.values() {
            if !entity.show {
                continue;
            }

            let position = entity
                .position
                .get_value(time)
                .map(|p| {
                    let carto = cesium_geospatial::Cartographic::from_radians(p[0], p[1], p[2]);
                    let cart = self.ellipsoid.cartographic_to_cartesian(&carto);
                    [cart.x, cart.y, cart.z]
                })
                .unwrap_or([0.0; 3]);

            // Billboard
            if let Some(ref bb_graphics) = entity.billboard {
                let show = bb_graphics.show.get_value(time).copied().unwrap_or(true);
                if show {
                    let billboard = Billboard {
                        show: true,
                        position,
                        scale: bb_graphics.scale.get_value(time).copied().unwrap_or(1.0),
                        color: bb_graphics.color.get_value(time).copied().unwrap_or(crate::property::Color::WHITE),
                        rotation: bb_graphics.rotation.get_value(time).copied().unwrap_or(0.0),
                        width: bb_graphics.width.get_value(time).copied(),
                        height: bb_graphics.height.get_value(time).copied(),
                        image: bb_graphics.image.get_value(time).cloned(),
                        id: Some(entity.id.clone()),
                        ..Default::default()
                    };
                    self.billboards.add(billboard);
                }
            }

            // Label
            if let Some(ref label_graphics) = entity.label {
                let show = label_graphics.show.get_value(time).copied().unwrap_or(true);
                if show {
                    let label = Label {
                        show: true,
                        position,
                        text: label_graphics.text.get_value(time).cloned().unwrap_or_default(),
                        font: label_graphics.font.get_value(time).cloned().unwrap_or_else(|| "30px sans-serif".to_string()),
                        fill_color: label_graphics.fill_color.get_value(time).copied().unwrap_or(crate::property::Color::WHITE),
                        outline_color: label_graphics.outline_color.get_value(time).copied().unwrap_or(crate::property::Color::BLACK),
                        outline_width: label_graphics.outline_width.get_value(time).copied().unwrap_or(2.0),
                        id: Some(entity.id.clone()),
                        ..Default::default()
                    };
                    self.labels.add(label);
                }
            }

            // Point
            if let Some(ref point_graphics) = entity.point {
                let show = point_graphics.show.get_value(time).copied().unwrap_or(true);
                if show {
                    let point = PointPrimitive {
                        show: true,
                        position,
                        color: point_graphics.color.get_value(time).copied().unwrap_or(crate::property::Color::WHITE),
                        outline_color: point_graphics.outline_color.get_value(time).copied().unwrap_or(crate::property::Color::BLACK),
                        outline_width: point_graphics.outline_width.get_value(time).copied().unwrap_or(0.0),
                        pixel_size: point_graphics.pixel_size.get_value(time).copied().unwrap_or(1.0),
                        id: Some(entity.id.clone()),
                        ..Default::default()
                    };
                    self.points.add(point);
                }
            }
        }
    }

    /// Gets the geometry visualizer.
    pub fn geometry_visualizer(&self) -> &GeometryVisualizer {
        &self.geometry_visualizer
    }

    /// Gets geometry for a specific entity.
    pub fn get_entity_geometry(&self, entity_id: &str) -> Option<&EntityGeometry> {
        self.geometry_visualizer.get_geometry(entity_id)
    }

    /// Total number of geometry instances.
    pub fn geometry_instance_count(&self) -> usize {
        self.geometry_visualizer.instance_count()
    }

    /// Number of billboards.
    pub fn billboard_count(&self) -> usize {
        self.billboards.len()
    }

    /// Number of labels.
    pub fn label_count(&self) -> usize {
        self.labels.len()
    }

    /// Number of points.
    pub fn point_count(&self) -> usize {
        self.points.len()
    }

    /// Marks the display as needing a full rebuild.
    pub fn mark_dirty(&mut self) {
        self.geometry_visualizer.mark_dirty();
        self.initialized = false;
    }
}

/// A data source display that manages multiple data sources.
///
/// Maps to CesiumJS `DataSources/DataSourceDisplay.js` with multiple sources
#[derive(Debug)]
pub struct MultiDataSourceDisplay {
    /// The underlying display.
    display: DataSourceDisplay,
    /// Tracked data sources.
    sources: Vec<DataSource>,
}

impl MultiDataSourceDisplay {
    /// Creates a new multi-data-source display.
    pub fn new(ellipsoid: Ellipsoid) -> Self {
        Self {
            display: DataSourceDisplay::new(ellipsoid),
            sources: Vec::new(),
        }
    }

    /// Creates a new multi-data-source display with WGS84 ellipsoid.
    pub fn wgs84() -> Self {
        Self::new(Ellipsoid::WGS84)
    }

    /// Adds a data source.
    pub fn add_data_source(&mut self, source: DataSource) {
        self.sources.push(source);
        self.display.mark_dirty();
    }

    /// Removes a data source by name.
    pub fn remove_data_source(&mut self, name: &str) -> Option<DataSource> {
        if let Some(idx) = self.sources.iter().position(|s| s.name == name) {
            self.display.mark_dirty();
            Some(self.sources.remove(idx))
        } else {
            None
        }
    }

    /// Updates all data sources at the given time.
    pub fn update(&mut self, time: f64) {
        // Merge all entities from all sources
        let mut merged = EntityCollection::new();
        for source in &self.sources {
            for entity in source.entities.values() {
                merged.add(entity.clone());
            }
        }
        self.display.update(&merged, time);
    }

    /// Gets the underlying display.
    pub fn display(&self) -> &DataSourceDisplay {
        &self.display
    }

    /// Number of data sources.
    pub fn source_count(&self) -> usize {
        self.sources.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::*;
    use crate::property::Property;

    fn make_entities() -> EntityCollection {
        let mut collection = EntityCollection::new();

        // Entity with box geometry
        collection.add(
            Entity::new("box-1")
                .with_position(0.0, 0.0, 0.0)
                .with_box(BoxGraphics {
                    dimensions: Property::Constant([100.0, 100.0, 100.0]),
                    ..Default::default()
                }),
        );

        // Entity with billboard
        collection.add(
            Entity::new("bb-1")
                .with_position(0.1, 0.1, 0.0)
                .with_billboard(BillboardGraphics {
                    image: Property::Constant("marker.png".to_string()),
                    scale: Property::Constant(2.0),
                    ..Default::default()
                }),
        );

        // Entity with label
        collection.add(
            Entity::new("label-1")
                .with_position(0.2, 0.2, 0.0)
                .with_label(LabelGraphics {
                    text: Property::Constant("Hello".to_string()),
                    ..Default::default()
                }),
        );

        // Entity with point
        collection.add(
            Entity::new("point-1")
                .with_position(0.3, 0.3, 0.0)
                .with_point(PointGraphics {
                    pixel_size: Property::Constant(10.0),
                    color: Property::Constant(crate::property::Color::RED),
                    ..Default::default()
                }),
        );

        collection
    }

    #[test]
    fn test_data_source_display_update() {
        let mut display = DataSourceDisplay::wgs84();
        let entities = make_entities();

        display.update(&entities, 0.0);

        assert_eq!(display.geometry_instance_count(), 1); // box
        assert_eq!(display.billboard_count(), 1);
        assert_eq!(display.label_count(), 1);
        assert_eq!(display.point_count(), 1);
    }

    #[test]
    fn test_data_source_display_geometry() {
        let mut display = DataSourceDisplay::wgs84();
        let entities = make_entities();

        display.update(&entities, 0.0);

        let geo = display.get_entity_geometry("box-1").unwrap();
        assert_eq!(geo.fill_instances.len(), 1);
    }

    #[test]
    fn test_data_source_display_hidden_entity() {
        let mut display = DataSourceDisplay::wgs84();
        let mut entities = EntityCollection::new();

        let mut entity = Entity::new("hidden-bb")
            .with_position(0.0, 0.0, 0.0)
            .with_billboard(BillboardGraphics {
                image: Property::Constant("test.png".to_string()),
                ..Default::default()
            });
        entity.show = false;
        entities.add(entity);

        display.update(&entities, 0.0);
        assert_eq!(display.billboard_count(), 0);
    }

    #[test]
    fn test_multi_data_source_display() {
        let mut multi = MultiDataSourceDisplay::wgs84();

        let mut source1 = DataSource::new("source-1");
        source1.entities.add(
            Entity::new("s1-box")
                .with_position(0.0, 0.0, 0.0)
                .with_box(BoxGraphics {
                    dimensions: Property::Constant([50.0, 50.0, 50.0]),
                    ..Default::default()
                }),
        );

        let mut source2 = DataSource::new("source-2");
        source2.entities.add(
            Entity::new("s2-point")
                .with_position(0.1, 0.1, 0.0)
                .with_point(PointGraphics {
                    pixel_size: Property::Constant(5.0),
                    ..Default::default()
                }),
        );

        multi.add_data_source(source1);
        multi.add_data_source(source2);
        assert_eq!(multi.source_count(), 2);

        multi.update(0.0);
        assert_eq!(multi.display().geometry_instance_count(), 1);
        assert_eq!(multi.display().point_count(), 1);
    }

    #[test]
    fn test_multi_data_source_remove() {
        let mut multi = MultiDataSourceDisplay::wgs84();
        multi.add_data_source(DataSource::new("temp"));
        assert_eq!(multi.source_count(), 1);

        let removed = multi.remove_data_source("temp");
        assert!(removed.is_some());
        assert_eq!(multi.source_count(), 0);
    }

    #[test]
    fn test_display_mark_dirty() {
        let mut display = DataSourceDisplay::wgs84();
        let entities = make_entities();

        display.update(&entities, 0.0);
        let count1 = display.geometry_instance_count();

        display.mark_dirty();
        display.update(&entities, 0.0);
        let count2 = display.geometry_instance_count();

        assert_eq!(count1, count2);
    }
}
