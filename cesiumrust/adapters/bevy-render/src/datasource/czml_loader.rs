//! 用于 Bevy 的 CZML 数据源加载器。
//!
//! 加载 .czml 文件，通过领域的 `parse_czml` 函数解析它们，
//! 并将 CesiumEntity + Graphics 组件生成到 ECS 中。

use bevy::prelude::*;
use cesium_datasource::czml::parse_czml;
use cesium_datasource::entity::Entity as DomainEntity;
use cesium_datasource::property::Property;

use crate::entity::components::{
    CesiumEntity, EntityWrapper, NeedsVisualUpdate, TimeDynamicProperties,
};

/// 跟踪待处理 CZML 文件加载的资源。
#[derive(Resource, Default)]
pub struct CzmlLoadQueue {
    /// 待加载的文件（指向 .czml 文件的路径）。
    pub files: Vec<String>,
}

/// 用于 CZML 数据源加载的插件。
pub struct CzmlLoadPlugin;

impl Plugin for CzmlLoadPlugin {
    /// 注册队列资源并挂载加载系统。
    ///
    /// # 参数
    /// - `app`：Bevy 应用
    fn build(&self, app: &mut App) {
        app.init_resource::<CzmlLoadQueue>()
            .add_systems(Update, czml_load_system);
    }
}

/// 从队列加载 .czml 文件并生成实体的系统。
///
/// # 参数
/// - `commands`：实体命令（生成实体）
/// - `queue`：待加载文件队列（可写，处理后清空）
fn czml_load_system(
    mut commands: Commands,
    mut queue: ResMut<CzmlLoadQueue>,
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
                error!("Failed to read CZML file {}: {}", file_path, e);
                continue;
            }
        };

        // 调用领域层解析器，失败则记录并跳过。
        let ds = match parse_czml(&content) {
            Ok(d) => d,
            Err(e) => {
                error!("Failed to parse CZML file {}: {}", file_path, e);
                continue;
            }
        };

        // 逐个生成数据源中的实体。
        let entity_count = ds.entities.len();
        info!("Loaded {} entities from CZML file {}", entity_count, file_path);

        for domain_entity in ds.entities.values() {
            spawn_czml_entity(&mut commands, domain_entity);
        }
    }
}

/// 将单个 CZML 实体生成到 Bevy ECS 中。
///
/// # 参数
/// - `commands`：实体命令
/// - `domain_entity`：领域层实体
fn spawn_czml_entity(commands: &mut Commands, domain_entity: &DomainEntity) {
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

/// Helper：通过将 CZML 文件加入队列来加载它。
///
/// # 参数
/// - `queue`：待加载队列
/// - `path`：.czml 文件路径
pub fn load_czml_file(queue: &mut CzmlLoadQueue, path: impl Into<String>) {
    queue.files.push(path.into());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 默认队列应为空。
    fn test_czml_load_queue_default() {
        let queue = CzmlLoadQueue::default();
        assert!(queue.files.is_empty());
    }

    #[test]
    /// load_czml_file 应将路径追加入队列。
    fn test_load_czml_file() {
        let mut queue = CzmlLoadQueue::default();
        load_czml_file(&mut queue, "test/czml/simple.czml");
        assert_eq!(queue.files.len(), 1);
        assert_eq!(queue.files[0], "test/czml/simple.czml");
    }
}
