//! 用于 Bevy 的 GeoJSON 数据源加载器。
//!
//! 加载 .geojson 文件，通过领域的 `parse_geojson` 函数解析它们，
//! 并生成适当的实体类型（Point → PointGraphics，
//! LineString → PolylineGraphics，Polygon → PolygonGraphics）。

use bevy::prelude::*;
use cesium_datasource::entity::Entity as DomainEntity;
use cesium_datasource::geojson::{parse_geojson, GeoJsonOptions};
use cesium_datasource::property::Property;

use crate::entity::components::{
    CesiumEntity, EntityWrapper, NeedsVisualUpdate, TimeDynamicProperties,
};

/// 跟踪待处理 GeoJSON 文件加载的资源。
#[derive(Resource, Default)]
pub struct GeoJsonLoadQueue {
    /// 待加载的文件（指向 .geojson 文件的路径）。
    pub files: Vec<String>,
}

/// 用于 GeoJSON 数据源加载的插件。
pub struct GeoJsonLoadPlugin;

impl Plugin for GeoJsonLoadPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GeoJsonLoadQueue>()
            .add_systems(Update, geojson_load_system);
    }
}

/// 加载 .geojson 文件并生成实体的系统。
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

/// 将单个 GeoJSON 实体生成到 Bevy ECS 中。
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

/// Helper：通过将 GeoJSON 文件加入队列来加载它。
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
