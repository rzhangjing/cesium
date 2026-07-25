//! Entity clustering and entity view (camera follow).
//!
//! Maps to CesiumJS:
//! - `DataSources/EntityCluster.js`
//! - `DataSources/EntityView.js`

use crate::entity::Entity;
use crate::entity_collection::EntityCollection;
use std::collections::HashMap;

/// Grid cell key: (cell_x, cell_y).
type GridKey = (i64, i64);
/// Entity position entry: (entity_id, position).
type EntityPos = (String, [f64; 3]);
/// Spatial hash grid for clustering.
type ClusterGrid = HashMap<GridKey, Vec<EntityPos>>;

/// A cluster of nearby entities.
#[derive(Debug, Clone)]
pub struct Cluster {
    /// The centroid position [lon_rad, lat_rad, height_m].
    pub position: [f64; 3],
    /// Entity IDs in this cluster.
    pub entity_ids: Vec<String>,
    /// Number of entities in the cluster.
    pub count: usize,
}

impl Cluster {
    /// Returns true if this cluster contains only one entity.
    pub fn is_single(&self) -> bool {
        self.count <= 1
    }
}

/// Configuration for entity clustering.
///
/// Maps to CesiumJS `DataSources/EntityCluster.js`
#[derive(Debug, Clone)]
pub struct EntityClusterOptions {
    /// Whether clustering is enabled.
    pub enabled: bool,
    /// The pixel range for clustering (entities within this range are clustered).
    pub pixel_range: f64,
    /// The minimum number of entities to form a cluster.
    pub minimum_cluster_size: usize,
}

impl Default for EntityClusterOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            pixel_range: 80.0,
            minimum_cluster_size: 2,
        }
    }
}

/// Entity clustering engine.
///
/// Groups nearby entities into clusters based on screen-space proximity.
/// In this domain implementation, we use a simple grid-based spatial hash
/// in cartographic space as an approximation of screen-space clustering.
///
/// Maps to CesiumJS `DataSources/EntityCluster.js`
#[derive(Debug)]
pub struct EntityCluster {
    /// Cluster options.
    pub options: EntityClusterOptions,
    /// Current clusters.
    clusters: Vec<Cluster>,
    /// Grid cell size in radians (approximation of pixel range).
    cell_size: f64,
}

impl EntityCluster {
    /// Creates a new entity cluster with default options.
    pub fn new() -> Self {
        Self {
            options: EntityClusterOptions::default(),
            clusters: Vec::new(),
            cell_size: 0.01, // ~0.57 degrees
        }
    }

    /// Creates a new entity cluster with custom options.
    pub fn with_options(options: EntityClusterOptions) -> Self {
        let cell_size = options.pixel_range * 0.000125; // Approximate conversion
        Self {
            options,
            clusters: Vec::new(),
            cell_size,
        }
    }

    /// Updates clusters for the given entities at the given time.
    ///
    /// Uses a grid-based spatial hash to group nearby entities.
    pub fn update(&mut self, entities: &EntityCollection, time: f64) {
        self.clusters.clear();

        if !self.options.enabled {
            return;
        }

        // Grid-based clustering
        let mut grid: ClusterGrid = HashMap::new();

        for entity in entities.values() {
            if !entity.show {
                continue;
            }

            if let Some(pos) = entity.position.get_value(time) {
                let cell_x = (pos[0] / self.cell_size).floor() as i64;
                let cell_y = (pos[1] / self.cell_size).floor() as i64;
                grid.entry((cell_x, cell_y))
                    .or_default()
                    .push((entity.id.clone(), *pos));
            }
        }

        // Convert grid cells to clusters
        for ((_cx, _cy), members) in grid {
            if members.len() >= self.options.minimum_cluster_size {
                // Compute centroid
                let count = members.len();
                let mut lon_sum = 0.0;
                let mut lat_sum = 0.0;
                let mut h_sum = 0.0;
                let ids: Vec<String> = members.iter().map(|(id, _)| id.clone()).collect();
                for (_, pos) in &members {
                    lon_sum += pos[0];
                    lat_sum += pos[1];
                    h_sum += pos[2];
                }
                self.clusters.push(Cluster {
                    position: [
                        lon_sum / count as f64,
                        lat_sum / count as f64,
                        h_sum / count as f64,
                    ],
                    entity_ids: ids,
                    count,
                });
            } else {
                // Single entities (not clustered)
                for (id, pos) in members {
                    self.clusters.push(Cluster {
                        position: pos,
                        entity_ids: vec![id],
                        count: 1,
                    });
                }
            }
        }
    }

    /// Gets the current clusters.
    pub fn clusters(&self) -> &[Cluster] {
        &self.clusters
    }

    /// Number of clusters.
    pub fn cluster_count(&self) -> usize {
        self.clusters.len()
    }

    /// Number of actual clusters (count > 1).
    pub fn actual_cluster_count(&self) -> usize {
        self.clusters.iter().filter(|c| !c.is_single()).count()
    }

    /// Total number of clustered entities.
    pub fn clustered_entity_count(&self) -> usize {
        self.clusters.iter().filter(|c| !c.is_single()).map(|c| c.count).sum()
    }
}

impl Default for EntityCluster {
    fn default() -> Self {
        Self::new()
    }
}

/// Camera follow mode for EntityView.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EntityViewMode {
    /// Camera follows the entity position.
    #[default]
    Follow,
    /// Camera tracks the entity with a fixed offset.
    Track,
    /// Camera looks at the entity from a distance.
    LookAt,
}

/// Entity view: makes the camera follow/track an entity.
///
/// Maps to CesiumJS `DataSources/EntityView.js`
#[derive(Debug, Clone)]
pub struct EntityView {
    /// The entity ID being followed.
    pub entity_id: String,
    /// The view mode.
    pub mode: EntityViewMode,
    /// Offset from the entity [x, y, z] in meters (for Track/LookAt modes).
    pub offset: [f64; 3],
    /// Last known entity position [x, y, z] in Cartesian3.
    pub last_position: [f64; 3],
    /// Whether the view is active.
    pub active: bool,
}

impl EntityView {
    /// Creates a new entity view that follows the given entity.
    pub fn new(entity_id: impl Into<String>) -> Self {
        Self {
            entity_id: entity_id.into(),
            mode: EntityViewMode::Follow,
            offset: [0.0, 0.0, 0.0],
            last_position: [0.0; 3],
            active: true,
        }
    }

    /// Creates a tracking entity view with an offset.
    pub fn tracking(entity_id: impl Into<String>, offset: [f64; 3]) -> Self {
        Self {
            entity_id: entity_id.into(),
            mode: EntityViewMode::Track,
            offset,
            last_position: [0.0; 3],
            active: true,
        }
    }

    /// Updates the view for the given entity at the given time.
    ///
    /// Returns the target camera position (entity position + offset).
    pub fn update(
        &mut self,
        entity: &Entity,
        time: f64,
        ellipsoid: &cesium_geospatial::Ellipsoid,
    ) -> Option<[f64; 3]> {
        if !self.active {
            return None;
        }

        let pos = entity.position.get_value(time)?;
        let carto = cesium_geospatial::Cartographic::from_radians(pos[0], pos[1], pos[2]);
        let cart = ellipsoid.cartographic_to_cartesian(&carto);

        self.last_position = [cart.x, cart.y, cart.z];

        let target = match self.mode {
            EntityViewMode::Follow => [cart.x, cart.y, cart.z],
            EntityViewMode::Track | EntityViewMode::LookAt => [
                cart.x + self.offset[0],
                cart.y + self.offset[1],
                cart.z + self.offset[2],
            ],
        };

        Some(target)
    }

    /// Deactivates the view.
    pub fn deactivate(&mut self) {
        self.active = false;
    }

    /// Activates the view.
    pub fn activate(&mut self) {
        self.active = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::PointGraphics;
    use crate::property::Property;

    fn make_cluster_entities() -> EntityCollection {
        let mut collection = EntityCollection::new();
        // Group of 3 nearby entities
        collection.add(Entity::new("p1").with_position(0.0, 0.0, 0.0).with_point(PointGraphics::default()));
        collection.add(Entity::new("p2").with_position(0.001, 0.001, 0.0).with_point(PointGraphics::default()));
        collection.add(Entity::new("p3").with_position(0.002, 0.002, 0.0).with_point(PointGraphics::default()));
        // Isolated entity far away
        collection.add(Entity::new("p4").with_position(1.0, 1.0, 0.0).with_point(PointGraphics::default()));
        collection
    }

    #[test]
    fn test_entity_cluster_basic() {
        let mut cluster = EntityCluster::new();
        let entities = make_cluster_entities();

        cluster.update(&entities, 0.0);

        // Should have clusters
        assert!(cluster.cluster_count() > 0);
        // At least one actual cluster (count > 1)
        assert!(cluster.actual_cluster_count() >= 1);
    }

    #[test]
    fn test_entity_cluster_disabled() {
        let mut cluster = EntityCluster::with_options(EntityClusterOptions {
            enabled: false,
            ..Default::default()
        });
        let entities = make_cluster_entities();

        cluster.update(&entities, 0.0);
        assert_eq!(cluster.cluster_count(), 0);
    }

    #[test]
    fn test_entity_cluster_minimum_size() {
        let mut cluster = EntityCluster::with_options(EntityClusterOptions {
            enabled: true,
            pixel_range: 80.0,
            minimum_cluster_size: 5, // Require 5 to cluster
        });
        let entities = make_cluster_entities();

        cluster.update(&entities, 0.0);
        // No clusters should form (max 3 nearby)
        assert_eq!(cluster.actual_cluster_count(), 0);
    }

    #[test]
    fn test_entity_cluster_single_entities() {
        let mut cluster = EntityCluster::new();
        let mut entities = EntityCollection::new();
        entities.add(Entity::new("solo").with_position(0.5, 0.5, 0.0));

        cluster.update(&entities, 0.0);
        assert_eq!(cluster.cluster_count(), 1);
        assert!(cluster.clusters()[0].is_single());
    }

    #[test]
    fn test_entity_view_follow() {
        let entity = Entity::new("vehicle").with_position(0.0, 0.0, 1000.0);
        let ellipsoid = cesium_geospatial::Ellipsoid::WGS84;

        let mut view = EntityView::new("vehicle");
        let target = view.update(&entity, 0.0, &ellipsoid).unwrap();

        // Position should be on the ellipsoid surface + 1000m
        let dist = (target[0] * target[0] + target[1] * target[1] + target[2] * target[2]).sqrt();
        assert!(dist > 6371000.0); // Earth radius + height
    }

    #[test]
    fn test_entity_view_tracking() {
        let entity = Entity::new("sat").with_position(0.0, 0.0, 0.0);
        let ellipsoid = cesium_geospatial::Ellipsoid::WGS84;

        let mut view = EntityView::tracking("sat", [1000.0, 0.0, 0.0]);
        let target = view.update(&entity, 0.0, &ellipsoid).unwrap();

        // Target should be offset from entity position
        let entity_pos = ellipsoid.cartographic_to_cartesian(
            &cesium_geospatial::Cartographic::from_radians(0.0, 0.0, 0.0),
        );
        let dx = target[0] - entity_pos.x;
        assert!((dx - 1000.0).abs() < 1.0);
    }

    #[test]
    fn test_entity_view_inactive() {
        let entity = Entity::new("v").with_position(0.0, 0.0, 0.0);
        let ellipsoid = cesium_geospatial::Ellipsoid::WGS84;

        let mut view = EntityView::new("v");
        view.deactivate();
        assert!(view.update(&entity, 0.0, &ellipsoid).is_none());

        view.activate();
        assert!(view.update(&entity, 0.0, &ellipsoid).is_some());
    }

    #[test]
    fn test_cluster_centroid() {
        let mut cluster = EntityCluster::new();
        let mut entities = EntityCollection::new();
        entities.add(Entity::new("a").with_position(0.0, 0.0, 0.0));
        entities.add(Entity::new("b").with_position(0.002, 0.002, 0.0));

        cluster.update(&entities, 0.0);

        // Find the cluster with both entities
        let multi = cluster.clusters().iter().find(|c| c.count == 2);
        if let Some(c) = multi {
            // Centroid should be approximately at (0.001, 0.001)
            assert!((c.position[0] - 0.001).abs() < 0.002);
            assert!((c.position[1] - 0.001).abs() < 0.002);
        }
    }
}
