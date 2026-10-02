//! 粒子子模块聚合：发射器组件、更新/渲染系统与预设。
//!
//! [`CesiumParticlePlugin`] 以固定顺序挂载生成→更新→渲染三个系统。
pub mod presets;
pub mod system;

use bevy::prelude::*;
pub use system::{
    particle_render_system, particle_spawn_system, particle_update_system,
    ParticleBurstResource, ParticleEmitterComponent, ParticleSystemComponent,
};

/// 挂载粒子生成/更新/渲染系统的 Bevy 插件。
pub struct CesiumParticlePlugin;

impl Plugin for CesiumParticlePlugin {
    /// 在 Update 阶段以 `.chain()` 固定“生成→更新→渲染”的执行顺序。
    ///
    /// # 参数
    /// - `app`：Bevy 应用
    fn build(&self, app: &mut App) {
        // 三者有数据依赖，必须链式串行而非并行。
        app.add_systems(Update, (
            particle_spawn_system,
            particle_update_system,
            particle_render_system,
        ).chain());
    }
}
