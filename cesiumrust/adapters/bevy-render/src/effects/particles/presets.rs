//! 粒子系统预设：提供火焰/烟雾/雪/火花/降雨等常用效果的快捷构造。
//!
//! 每个函数接收发射器位置，返回一个配置好的 [`ParticleSystem`]；简单预设
//! 直接委托领域层工厂，复杂预设（火花/雨）在此组装 [`ParticleSystemConfig`]。
use cesium_effects::{
    EmitterShape, ParticleForce, ParticleSystem, ParticleSystemConfig,
};
use glam::DVec3;

/// 创建火焰粒子预设。
///
/// # 参数
/// - `emitter_position`：发射器世界坐标
/// # 返回
/// 配置好的火焰 [`ParticleSystem`]
pub fn fire_preset(emitter_position: DVec3) -> ParticleSystem {
    ParticleSystem::fire(emitter_position)
}

/// 创建烟雾粒子预设。
///
/// # 参数
/// - `emitter_position`：发射器世界坐标
/// # 返回
/// 配置好的烟雾 [`ParticleSystem`]
pub fn smoke_preset(emitter_position: DVec3) -> ParticleSystem {
    ParticleSystem::smoke(emitter_position)
}

/// 创建降雪粒子预设。
///
/// # 参数
/// - `emitter_position`：发射器世界坐标
/// # 返回
/// 配置好的降雪 [`ParticleSystem`]
pub fn snow_preset(emitter_position: DVec3) -> ParticleSystem {
    ParticleSystem::snow(emitter_position)
}

/// 创建火花粒子预设（短寿命、快初速、受重力，单次喷射）。
///
/// # 参数
/// - `emitter_position`：发射器世界坐标
/// # 返回
/// 组装完成的火花 [`ParticleSystem`]
pub fn spark_preset(emitter_position: DVec3) -> ParticleSystem {
    // 点发射器、高温黄→透明、不循环并施加向下的重力。
    let config = ParticleSystemConfig {
        emitter_shape: EmitterShape::Point,
        emission_rate: 30.0,
        min_lifetime: 0.1,
        max_lifetime: 1.0,
        min_speed: 5.0,
        max_speed: 15.0,
        min_size: 0.05,
        max_size: 0.2,
        start_color: [1.0, 0.9, 0.3, 1.0],
        end_color: [0.0, 0.0, 0.0, 0.0],
        max_particles: 200,
        looping: false,
        forces: vec![ParticleForce::Gravity {
            acceleration: DVec3::new(0.0, -9.81, 0.0),
        }],
        ..Default::default()
    };
    ParticleSystem::new(config, emitter_position)
}

/// 创建降雨粒子预设（大矩形发射器、向下高速、循环发射）。
///
/// # 参数
/// - `emitter_position`：发射器（雨幕中心）世界坐标
/// # 返回
/// 发射方向沿 -Y 的雨滴 [`ParticleSystem`]
pub fn rain_preset(emitter_position: DVec3) -> ParticleSystem {
    // 扁平盒状发射器覆盖一片区域，半透明蓝、循环、重力加速下落。
    let config = ParticleSystemConfig {
        emitter_shape: EmitterShape::Box {
            half_extents: DVec3::new(20.0, 0.1, 20.0),
        },
        emission_rate: 200.0,
        min_lifetime: 1.0,
        max_lifetime: 3.0,
        min_speed: 15.0,
        max_speed: 30.0,
        min_size: 0.05,
        max_size: 0.15,
        start_color: [0.3, 0.5, 0.9, 0.6],
        end_color: [0.3, 0.5, 0.9, 0.1],
        max_particles: 3000,
        looping: true,
        forces: vec![
            ParticleForce::Gravity {
                acceleration: DVec3::new(0.0, -20.0, 0.0),
            },
            ParticleForce::Wind {
                velocity: DVec3::new(0.0, 0.0, 0.0),
            },
        ],
        ..Default::default()
    };
    let mut sys = ParticleSystem::new(config, emitter_position);
    sys.emitter_direction = DVec3::NEG_Y;
    sys
}
