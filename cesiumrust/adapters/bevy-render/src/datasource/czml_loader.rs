//! CZML data source loader for Bevy.
//!
//! Loads .czml files, parses them via the domain `parse_czml` function,
//! and spawns CesiumEntity + Graphics components into the ECS.

use bevy::prelude::*;
use cesium_datasource::czml::parse_czml;
use cesium_datasource::entity::Entity as DomainEntity;
use cesium_datasource::property::Property;

use crate::entity::components::{
    CesiumEntity, EntityWrapper, NeedsVisualUpdate, TimeDynamicProperties,
};

/// Resource that tracks pending CZML file loads.
#[derive(Resource, Default)]
pub struct CzmlLoadQueue {
    /// Files to load (paths to .czml files).
    pub files: Vec<String>,
}

/// Plugin for CZML data source loading.
pub struct CzmlLoadPlugin;

impl Plugin for CzmlLoadPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CzmlLoadQueue>()
            .add_systems(Update, czml_load_system);
    }
}

/// System that loads .czml files from the queue and spawns entities.
fn czml_load_system(
    mut commands: Commands,
    mut queue: ResMut<CzmlLoadQueue>,
) {
    if queue.files.is_empty() {
        return;
    }

    let files: Vec<String> = queue.files.drain(..).collect();

    for file_path in &files {
        let content = match std::fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to read CZML file {}: {}", file_path, e);
                continue;
            }
        };

        let ds = match parse_czml(&content) {
            Ok(d) => d,
            Err(e) => {
                error!("Failed to parse CZML file {}: {}", file_path, e);
                continue;
            }
        };

        let entity_count = ds.entities.len();
        info!("Loaded {} entities from CZML file {}", entity_count, file_path);

        for domain_entity in ds.entities.values() {
            spawn_czml_entity(&mut commands, domain_entity);
        }
    }
}

/// Spawns a single CZML entity into the Bevy ECS.
fn spawn_czml_entity(commands: &mut Commands, domain_entity: &DomainEntity) {
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

/// Helper to load a CZML file by adding it to the queue.
pub fn load_czml_file(queue: &mut CzmlLoadQueue, path: impl Into<String>) {
    queue.files.push(path.into());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_czml_load_queue_default() {
        let queue = CzmlLoadQueue::default();
        assert!(queue.files.is_empty());
    }

    #[test]
    fn test_load_czml_file() {
        let mut queue = CzmlLoadQueue::default();
        load_czml_file(&mut queue, "test/czml/simple.czml");
        assert_eq!(queue.files.len(), 1);
        assert_eq!(queue.files[0], "test/czml/simple.czml");
    }
}
