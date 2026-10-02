//! 用于 Bevy 的 GPX 数据源加载器。
//!
//! 通过 `cesium_gpx::parser` 解析 GPX 文件，并将轨迹（tracks）
//! 转为 PolylineGraphics，将航点（waypoints）转为 PointGraphics。

use bevy::prelude::*;
use cesium_datasource::entity::Entity as DomainEntity;
use cesium_datasource::property::Property;
use cesium_gpx::parser::{gpx_to_datasource, parse_gpx_simple};

use crate::entity::components::{
    CesiumEntity, EntityWrapper, NeedsVisualUpdate, TimeDynamicProperties,
};

/// 跟踪待处理 GPX 文件加载的资源。
#[derive(Resource, Default)]
pub struct GpxLoadQueue {
    /// 待加载的文件（指向 .gpx 文件的路径）。
    pub files: Vec<String>,
}

/// 用于 GPX 数据源加载的插件。
pub struct GpxLoadPlugin;

impl Plugin for GpxLoadPlugin {
    /// 注册队列资源并挂载加载系统。
    ///
    /// # 参数
    /// - `app`：Bevy 应用
    fn build(&self, app: &mut App) {
        app.init_resource::<GpxLoadQueue>()
            .add_systems(Update, gpx_load_system);
    }
}

/// 加载 GPX 文件并生成实体的系统。
///
/// # 参数
/// - `commands`：实体命令（生成实体）
/// - `queue`：待加载文件队列（可写，处理后清空）
fn gpx_load_system(
    mut commands: Commands,
    mut queue: ResMut<GpxLoadQueue>,
) {
    // 队列为空时无事可做。
    if queue.files.is_empty() {
        return;
    }

    // 先排空队列，避免处理过程中长期借用。
    let files: Vec<String> = queue.files.drain(..).collect();

    for file_path in &files {
        // 读取磁盘文本（失败则记录并跳过）。
        let content = match std::fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to read GPX file {}: {}", file_path, e);
                continue;
            }
        };

        // 解析为 GPX 文档（失败则记录并跳过）。
        let doc = match parse_gpx_simple(&content) {
            Ok(d) => d,
            Err(e) => {
                error!("Failed to parse GPX file {}: {}", file_path, e);
                continue;
            }
        };

        // 转为领域数据源并逐个生成实体。
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

/// 将单个 GPX 实体生成到 Bevy ECS 中。
///
/// # 参数
/// - `commands`：实体命令
/// - `domain_entity`：领域层实体
fn spawn_gpx_entity(commands: &mut Commands, domain_entity: &DomainEntity) {
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

/// Helper：通过将 GPX 文件加入队列来加载它。
///
/// # 参数
/// - `queue`：待加载队列
/// - `path`：.gpx 文件路径
pub fn load_gpx_file(queue: &mut GpxLoadQueue, path: impl Into<String>) {
    queue.files.push(path.into());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 默认队列应为空。
    fn test_gpx_load_queue_default() {
        let queue = GpxLoadQueue::default();
        assert!(queue.files.is_empty());
    }

    #[test]
    /// load_gpx_file 应将路径追参加队列。
    fn test_load_gpx_file() {
        let mut queue = GpxLoadQueue::default();
        load_gpx_file(&mut queue, "test/data/route.gpx");
        assert_eq!(queue.files.len(), 1);
    }
}
