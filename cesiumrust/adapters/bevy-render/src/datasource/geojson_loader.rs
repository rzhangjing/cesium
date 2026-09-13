//! GeoJSON data source loader for Bevy.
//!
//! Loads .geojson files, parses them via the domain `parse_geojson` function,
//! and spawns appropriate entity types (Point → PointGraphics,
//! LineString → PolylineGraphics, Polygon → PolygonGraphics).

use bevy::prelude::*;
use cesium_datasource::entity::Entity as DomainEntity;
use cesium_datasource::geojson::{parse_geojson, GeoJsonOptions};
use cesium_datasource::property::Property;

use crate::entity::components::{
    CesiumEntity, EntityWrapper, NeedsVisualUpdate, TimeDynamicProperties,
};

/// Resource that tracks pending GeoJSON file loads.
#[derive(Resource, Default)]
pub struct GeoJsonLoadQueue {
    /// Files to load (paths to .geojson files).
    pub files: Vec<String>,
}

/// Plugin for GeoJSON data source loading.
pub struct GeoJsonLoadPlugin;

impl Plugin for GeoJsonLoadPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GeoJsonLoadQueue>()
            .add_systems(Update, geojson_load_system);
    }
}

/// System that loads .geojson files and spawns entities.
fn geojson_load_system(
    mut commands: Commands,
    mut queue: ResMut<GeoJsonLoadQueue>,
) {
    if queue.files.is_empty() {
        return;
    }

    let files: Vec<String> = queue.files.drain(..).collect();
    let options = GeoJsonOptions::default();

    for file_path in &files {
        let content = match std::fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to read GeoJSON file {}: {}", file_path, e);
                continue;
            }
        };

        let ds = match parse_geojson(&content, &options) {
            Ok(d) => d,
            Err(e) => {
                error!("Failed to parse GeoJSON file {}: {}", file_path, e);
                continue;
            }
        };

        let entity_count = ds.entities.len();
        info!(
            "Loaded {} entities from GeoJSON file {}",
            entity_count, file_path
        );

        for domain_entity in ds.entities.values() {
            spawn_geojson_entity(&mut commands, domain_entity);
        }
    }
}

/// Spawns a single GeoJSON entity into the Bevy ECS.
fn spawn_geojson_entity(commands: &mut Commands, domain_entity: &DomainEntity) {
    let cesium_entity = CesiumEntity {
        entity_id: domain_entity.id.clone(),
        name: domain_entity.name.clone().unwrap_or_default(),
        description: domain_entity.description.clone(),
        show: domain_entity.show,
        availability: domain_entity.availability.clone(),
    };

    let mut time_dyn = TimeDynamicProperties::default();
    if matches!(domain_entity.position, Property::Sampled(_)) {
        time_dyn.has_interpolated_position = true;
    }
    if domain_entity.availability.is_some() {
        time_dyn.has_availability = true;
    }

    commands.spawn((
        EntityWrapper::new(domain_entity.clone()),
        cesium_entity,
        time_dyn,
        NeedsVisualUpdate,
        Transform::IDENTITY,
        Visibility::Visible,
    ));
}

/// Helper to load a GeoJSON file by adding it to the queue.
pub fn load_geojson_file(queue: &mut GeoJsonLoadQueue, path: impl Into<String>) {
    queue.files.push(path.into());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geojson_load_queue_default() {
        let queue = GeoJsonLoadQueue::default();
        assert!(queue.files.is_empty());
    }

    #[test]
    fn test_load_geojson_file() {
        let mut queue = GeoJsonLoadQueue::default();
        load_geojson_file(&mut queue, "test/data/points.geojson");
        assert_eq!(queue.files.len(), 1);
    }
}
