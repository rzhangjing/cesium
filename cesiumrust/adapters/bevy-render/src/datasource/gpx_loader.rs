//! GPX data source loader for Bevy.
//!
//! Parses GPX files via `cesium_gpx::parser` and converts tracks
//! to PolylineGraphics and waypoints to PointGraphics.

use bevy::prelude::*;
use cesium_datasource::entity::Entity as DomainEntity;
use cesium_datasource::property::Property;
use cesium_gpx::parser::{gpx_to_datasource, parse_gpx_simple};

use crate::entity::components::{
    CesiumEntity, EntityWrapper, NeedsVisualUpdate, TimeDynamicProperties,
};

/// Resource that tracks pending GPX file loads.
#[derive(Resource, Default)]
pub struct GpxLoadQueue {
    /// Files to load (paths to .gpx files).
    pub files: Vec<String>,
}

/// Plugin for GPX data source loading.
pub struct GpxLoadPlugin;

impl Plugin for GpxLoadPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GpxLoadQueue>()
            .add_systems(Update, gpx_load_system);
    }
}

/// System that loads GPX files and spawns entities.
fn gpx_load_system(
    mut commands: Commands,
    mut queue: ResMut<GpxLoadQueue>,
) {
    if queue.files.is_empty() {
        return;
    }

    let files: Vec<String> = queue.files.drain(..).collect();

    for file_path in &files {
        let content = match std::fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to read GPX file {}: {}", file_path, e);
                continue;
            }
        };

        let doc = match parse_gpx_simple(&content) {
            Ok(d) => d,
            Err(e) => {
                error!("Failed to parse GPX file {}: {}", file_path, e);
                continue;
            }
        };

        let ds = gpx_to_datasource(&doc);
        let entity_count = ds.entities.len();
        info!(
            "Loaded {} entities from GPX file {}",
            entity_count, file_path
        );

        for domain_entity in ds.entities.values() {
            spawn_gpx_entity(&mut commands, domain_entity);
        }
    }
}

/// Spawns a single GPX entity into the Bevy ECS.
fn spawn_gpx_entity(commands: &mut Commands, domain_entity: &DomainEntity) {
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

/// Helper to load a GPX file by adding it to the queue.
pub fn load_gpx_file(queue: &mut GpxLoadQueue, path: impl Into<String>) {
    queue.files.push(path.into());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpx_load_queue_default() {
        let queue = GpxLoadQueue::default();
        assert!(queue.files.is_empty());
    }

    #[test]
    fn test_load_gpx_file() {
        let mut queue = GpxLoadQueue::default();
        load_gpx_file(&mut queue, "test/data/route.gpx");
        assert_eq!(queue.files.len(), 1);
    }
}
