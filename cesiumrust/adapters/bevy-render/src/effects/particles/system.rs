// 遗留的 CesiumJS 移植风格债（deferred.md #18）；在 M13 lint 清理时或本文件在其里程碑被重写时重新审视
//! 粒子系统适配层：将领域 `cesium_effects` 的粒子状态挂载到 Bevy
//! 实体，并在每帧推进与可视化。
//!
//! 三个阶段：[`particle_spawn_system`] 同步发射器位置并推进模拟；
//! [`particle_update_system`] 用 gizmos 把存活粒子画为小圆（开发可视化）；
//! [`particle_render_system`] 为未来的 GPU compute 渲染预留接口。
#![allow(unused_imports)]
use bevy::prelude::*;
use cesium_effects::{
    EmitterShape, ParticleBurst, ParticleSystem, ParticleSystemConfig,
};
use glam::{DVec3, Vec3};

/// 粒子系统组件：持有领域粒子系统与可见开关。
#[derive(Component, Debug)]
pub struct ParticleSystemComponent {
    /// 领域粒子系统（逐帧更新）。
    pub system: ParticleSystem,
    /// 是否参与更新/渲染。
    pub visible: bool,
}

impl ParticleSystemComponent {
    /// 用自定义配置与发射器位置构造。
    ///
    /// # 参数
    /// - `config`：粒子系统配置
    /// - `emitter_position`：初始发射器位置
    pub fn new(config: ParticleSystemConfig, emitter_position: DVec3) -> Self {
        Self {
            system: ParticleSystem::new(config, emitter_position),
            visible: true,
        }
    }

    /// 火焰预设（锥形发射、暖色）。
    ///
    /// # 参数
    /// - `emitter_position`：发射器位置
    pub fn fire(emitter_position: DVec3) -> Self {
        Self {
            system: ParticleSystem::fire(emitter_position),
            visible: true,
        }
    }

    /// 烟雾预设（球形发射、灰色）。
    ///
    /// # 参数
    /// - `emitter_position`：发射器位置
    pub fn smoke(emitter_position: DVec3) -> Self {
        Self {
            system: ParticleSystem::smoke(emitter_position),
            visible: true,
        }
    }

    /// 雪预设（盒形发射、白色）。
    ///
    /// # 参数
    /// - `emitter_position`：发射器位置
    pub fn snow(emitter_position: DVec3) -> Self {
        Self {
            system: ParticleSystem::snow(emitter_position),
            visible: true,
        }
    }
}

/// 发射器描述组件（位置/方向/形状），供外部控制发射参数。
#[derive(Component, Debug, Clone)]
pub struct ParticleEmitterComponent {
    /// 发射器世界位置。
    pub position: DVec3,
    /// 主发射方向。
    pub direction: DVec3,
    /// 发射器形状。
    pub shape: EmitterShape,
}

impl Default for ParticleEmitterComponent {
    /// 默认：原点、沿 +Y、点发射。
    fn default() -> Self {
        Self {
            position: DVec3::ZERO,
            direction: DVec3::Y,
            shape: EmitterShape::Point,
        }
    }
}

/// 一次性粒子爆发资源（缓存待触发的 burst 与累计时间）。
#[derive(Resource, Debug, Clone)]
pub struct ParticleBurstResource {
    /// 待触发的爆发列表。
    pub bursts: Vec<ParticleBurst>,
    /// 累计时间（秒）。
    pub time: f64,
}

impl Default for ParticleBurstResource {
    /// 默认：无爆发、时间 0。
    fn default() -> Self {
        Self {
            bursts: Vec::new(),
            time: 0.0,
        }
    }
}

impl ParticleBurstResource {
    /// 追加一个爆发到队列尾部。
    ///
    /// # 参数
    /// - `burst`：待触发的爆发
    pub fn add_burst(&mut self, burst: ParticleBurst) {
        self.bursts.push(burst);
    }
}

/// 推进系统：同步发射器位置到实体变换，逐帧推进粒子模拟。
///
/// # 参数
/// - `time`：帧时钟（提供 delta 与种子）
/// - `query`：粒子组件与全局变换
pub fn particle_spawn_system(
    time: Res<Time>,
    mut query: Query<(&mut ParticleSystemComponent, &GlobalTransform)>,
) {
    // 本帧时长与基于已用时间的确定性随机种子。
    let dt = time.delta_secs_f64();
    let rng_seed = (time.elapsed_secs_f64() * 1000.0) as u64;

    for (mut comp, transform) in query.iter_mut() {
        // 不可见的系统直接跳过，不做任何模拟。
        if !comp.visible {
            continue;
        }

        // 把实体平移（f32→f64）作为新的发射器位置。
        let pos = DVec3::new(
            transform.translation().x as f64,
            transform.translation().y as f64,
            transform.translation().z as f64,
        );
        comp.system.emitter_position = pos;

        // 推进一个时间步，使用同一种子保证可重现。
        comp.system.update(dt, rng_seed);
    }
}

/// 可视化系统：用 gizmos 把存活粒子画为小圆（仅开发展示）。
///
/// # 参数
/// - `gizmos`：线条/圆形绘制器
/// - `query`：粒子组件与全局变换
pub fn particle_update_system(
    mut gizmos: Gizmos,
    query: Query<(&ParticleSystemComponent, &GlobalTransform)>,
) {
    for (comp, transform) in query.iter() {
        // 实体父级位置（保留，供局部→世界坐标扩展）。
        let parent_pos = DVec3::new(
            transform.translation().x as f64,
            transform.translation().y as f64,
            transform.translation().z as f64,
        );

        for particle in &comp.system.particles {
            // 已消亡的粒子不绘制。
            if !particle.alive {
                continue;
            }
            // 按生命周期插值出颜色与当前尺寸。
            let color = comp.system.particle_color(particle);
            let pos = particle.position;
            let size = (particle.size * particle.current_scale()) as f32;

            // 世界坐标（f64→f32），gizmos 需要 f32。
            let world_pos = Vec3::new(
                (pos.x) as f32,
                (pos.y) as f32,
                (pos.z) as f32,
            );

            let _parent = Vec3::new(
                parent_pos.x as f32,
                parent_pos.y as f32,
                parent_pos.z as f32,
            );

            // 尺寸下限 0.05，避免粒子不可见。
            gizmos.circle(
                Isometry3d::new(world_pos, Quat::IDENTITY),
                size.max(0.05),
                Color::srgba(
                    color[0] as f32,
                    color[1] as f32,
                    color[2] as f32,
                    color[3] as f32,
                ),
            );
        }
    }
}

/// 渲染系统占位（当前无操作，为未来 GPU 通道预留）。
///
/// # 参数
/// - `query`：粒子组件与全局变换（当前未使用）
pub fn particle_render_system(
    query: Query<(&ParticleSystemComponent, &GlobalTransform)>,
) {
    for (_comp, _transform) in query.iter() {
        // GPU 渲染暂缓 —— 粒子当前通过 particle_update_system 中的 gizmos 绘制。
        // 未来的 compute-shader 通道将取代这里。
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_effects::ParticleForce;

    #[test]
    /// 验证 fire 预设使用锥形发射与暖色起始颜色。
    fn test_particle_system_component_fire() {
        let comp = ParticleSystemComponent::fire(DVec3::new(0.0, 0.0, 0.0));
        assert!(comp.visible);
        assert!(
            matches!(comp.system.config.emitter_shape, EmitterShape::Cone { .. })
        );
        assert_eq!(comp.system.config.start_color, [1.0, 0.8, 0.2, 1.0]);
    }

    #[test]
    /// 验证 smoke 预设使用球形发射与灰色起始颜色。
    fn test_particle_system_component_smoke() {
        let comp = ParticleSystemComponent::smoke(DVec3::new(1.0, 2.0, 3.0));
        assert!(comp.visible);
        assert!(
            matches!(comp.system.config.emitter_shape, EmitterShape::Sphere { .. })
        );
        assert_eq!(comp.system.config.start_color, [0.4, 0.4, 0.4, 0.8]);
    }

    #[test]
    /// 验证 snow 预设使用盒形发射与白色起始颜色。
    fn test_particle_system_component_snow() {
        let comp = ParticleSystemComponent::snow(DVec3::new(10.0, 100.0, 10.0));
        assert!(comp.visible);
        assert!(
            matches!(comp.system.config.emitter_shape, EmitterShape::Box { .. })
        );
        assert_eq!(comp.system.config.start_color, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    /// 验证多帧推进后仍有存活粒子（系统循环发射）。
    fn test_particle_lifecycle() {
        let mut comp = ParticleSystemComponent::fire(DVec3::ZERO);
        comp.system.config.emission_rate = 50.0;
        comp.system.config.min_lifetime = 0.5;
        comp.system.config.max_lifetime = 0.5;

        comp.system.update(0.1, 42);
        assert!(comp.system.particle_count() > 0);

        for _ in 0..20 {
            comp.system.update(0.1, 42);
        }

        // 粒子应仍然存活，因为系统循环
        // （有些可能已消亡但新的已发射）
        assert!(comp.system.particle_count() > 0);
    }

    #[test]
    /// 验证三个预设各自的发射率。
    fn test_preset_creation() {
        let fire = ParticleSystemComponent::fire(DVec3::ZERO);
        assert_eq!(fire.system.config.emission_rate, 50.0);

        let smoke = ParticleSystemComponent::smoke(DVec3::ZERO);
        assert_eq!(smoke.system.config.emission_rate, 20.0);

        let snow = ParticleSystemComponent::snow(DVec3::ZERO);
        assert_eq!(snow.system.config.emission_rate, 100.0);
    }
}
