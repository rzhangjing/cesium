//! 用于 Bevy 的 KML 数据源加载器。
//!
//! 通过 `cesium_kml::parser` 解析 KML 文件，并将 Placemarks
//! 转为带样式继承的适当实体类型。

use bevy::prelude::*;
use cesium_datasource::entity::Entity as DomainEntity;
use cesium_datasource::property::Property;
use cesium_kml::parser::{kml_to_datasource, parse_kml_simple};

use crate::entity::components::{
    CesiumEntity, EntityWrapper, NeedsVisualUpdate, TimeDynamicProperties,
};

/// 跟踪待处理 KML 文件加载的资源。
#[derive(Resource, Default)]
pub struct KmlLoadQueue {
    /// 待加载的文件（指向 .kml/.kmz 文件的路径）。
    pub files: Vec<String>,
}

/// 用于 KML 数据源加载的插件。
pub struct KmlLoadPlugin;

impl Plugin for KmlLoadPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<KmlLoadQueue>()
            .add_systems(Update, kml_load_system);
    }
}

/// 加载 KML 文件并生成实体的系统。
fn kml_load_system(
    mut commands: Commands,
    mut queue: ResMut<KmlLoadQueue>,
) {
    if queue.files.is_empty() {
        return;
    }

    let files: Vec<String> = queue.files.drain(..).collect();

    for file_path in &files {
        let content = match std::fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to read KML file {}: {}", file_path, e);
                continue;
            }
        };

        let doc = match parse_kml_simple(&content) {
            Ok(d) => d,
            Err(e) => {
                error!("Failed to parse KML file {}: {}", file_path, e);
                continue;
            }
        };

        let ds = kml_to_datasource(&doc);
        let entity_count = ds.entities.len();
        info!(
            "Loaded {} entities from KML file {}",
            entity_count, file_path
        );

        for domain_entity in ds.entities.values() {
            spawn_kml_entity(&mut commands, domain_entity);
        }
    }
}

/// 将单个 KML 实体生成到 Bevy ECS 中。
fn spawn_kml_entity(commands: &mut Commands, domain_entity: &DomainEntity) {
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

/// Helper：通过将 KML 文件加入队列来加载它。
pub fn load_kml_file(queue: &mut KmlLoadQueue, path: impl Into<String>) {
    queue.files.push(path.into());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kml_load_queue_default() {
        let queue = KmlLoadQueue::default();
        assert!(queue.files.is_empty());
    }

    #[test]
    fn test_load_kml_file() {
        let mut queue = KmlLoadQueue::default();
        load_kml_file(&mut queue, "test/data/places.kml");
        assert_eq!(queue.files.len(), 1);
    }
}
