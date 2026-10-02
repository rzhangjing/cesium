//! 影像子模块聚合：图层管理、瓦片加载与混合系统。
//!
//! [`CesiumImageryPlugin`] 统一注册影像相关资源并挂载请求/加载/混合系统。
pub mod blend_system;
pub mod layer_manager;
pub mod tile_loader;

use bevy::prelude::*;

pub use blend_system::{
    imagery_apply_system, imagery_blend_compute_system, ImageryBlendCache,
};
pub use layer_manager::{ImageryLayerManager, ImageryLayerDescriptor};
pub use tile_loader::{
    imagery_tile_load_system, imagery_tile_request_system, ImageryCache, ImageryPendingLoads,
};

/// 注册影像资源并挂载其系统的 Bevy 插件。
pub struct CesiumImageryPlugin;

impl Plugin for CesiumImageryPlugin {
    /// 初始化图层/缓存/待加载/混合资源，并在 PreUpdate 请求、Update 加载与混合。
    ///
    /// # 参数
    /// - `app`：Bevy 应用
    fn build(&self, app: &mut App) {
        // 请求与加载分阶段：PreUpdate 先算出待拉瓦片，Update 再下载与混合。
        app.init_resource::<ImageryLayerManager>()
            .init_resource::<ImageryCache>()
            .init_resource::<ImageryPendingLoads>()
            .init_resource::<ImageryBlendCache>()
            .add_systems(PreUpdate, imagery_tile_request_system)
            .add_systems(
                Update,
                (imagery_tile_load_system, imagery_apply_system, imagery_blend_compute_system),
            );
    }
}
