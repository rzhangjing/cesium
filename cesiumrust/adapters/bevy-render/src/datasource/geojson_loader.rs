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
    /// 注册队列资源并挂载加载系统。
    ///
    /// # 参数
    /// - `app`：Bevy 应用
    fn build(&self, app: &mut App) {
        app.init_resource::<GeoJsonLoadQueue>()
            .add_systems(Update, geojson_load_system);
    }
}

/// 加载 .geojson 文件并生成实体的系统。
///
/// # 参数
/// - `commands`：实体命令（生成实体）
/// - `queue`：待加载文件队列（可写，处理后清空）
fn geojson_load_system(
    mut commands: Commands,
    mut queue: ResMut<GeoJsonLoadQueue>,
) {
    // 队列为空时无事可做。
    if queue.files.is_empty() {
        return;
    }

    // 先排空队列；使用默认 GeoJSON 选项（后续可传入自定义）。
    let files: Vec<String> = queue.files.drain(..).collect();
    let options = GeoJsonOptions::default();

    for file_path in &files {
        // 读取磁盘文本（失败则记录并跳过）。
        let content = match std::fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to read GeoJSON file {}: {}", file_path, e);
                continue;
            }
        };

        // 按选项解析为领域数据源，失败则记录并跳过。
        let ds = match parse_geojson(&content, &options) {
            Ok(d) => d,
            Err(e) => {
                error!("Failed to parse GeoJSON file {}: {}", file_path, e);
                continue;
            }
        };

        // 逐个生成数据源中的实体。
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
///
/// # 参数
/// - `commands`：实体命令
/// - `domain_entity`：领域层实体
fn spawn_geojson_entity(commands: &mut Commands, domain_entity: &DomainEntity) {
    let cesium_entity = CesiumEntity {
        entity_id: domain_entity.id.clone(),
        name: domain_entity.name.clone().unwrap_or_default(),
        description: domain_entity.description.clone(),
        show: domain_entity.show,
        availability: domain_entity.availability.clone(),
    };

    // 采样位置/可用性区间→标记为时动态，供动画系统后续处理。
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
///
/// # 参数
/// - `queue`：待加载队列
/// - `path`：.geojson 文件路径
pub fn load_geojson_file(queue: &mut GeoJsonLoadQueue, path: impl Into<String>) {
    queue.files.push(path.into());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 默认队列应为空。
    fn test_geojson_load_queue_default() {
        let queue = GeoJsonLoadQueue::default();
        assert!(queue.files.is_empty());
    }

    #[test]
    /// load_geojson_file 应将路径追加入队列。
    fn test_load_geojson_file() {
        let mut queue = GeoJsonLoadQueue::default();
        load_geojson_file(&mut queue, "test/data/points.geojson");
        assert_eq!(queue.files.len(), 1);
    }
}
