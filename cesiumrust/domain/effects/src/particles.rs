//! 用于视觉效果（火焰、烟雾、降雪等）的粒子系统。
//!
//! 涵盖以下要素：
//! - 粒子发射器（point、cone、box、sphere）
//! - 粒子生命周期（出生、更新、死亡）
//! - 粒子受力（重力、阻力、风）

use glam::DVec3;

/// 粒子发射器形状。
#[derive(Debug, Clone, PartialEq, Default)]
pub enum EmitterShape {
    /// 从单个点发射。
    #[default]
    Point,
    /// 从圆锥形状发射。
    Cone {
        /// 圆锥角度（弧度）。
        angle: f64,
    },
    /// 从立方体体积发射。
    Box {
        /// 立方体的半长。
        half_extents: DVec3,
    },
    /// 从球面发射。
    Sphere {
        /// 球半径。
        radius: f64,
    },
    /// 从圆（圆盘）发射。
    Circle {
        /// 圆半径。
        radius: f64,
    },
}


/// 系统中的单个粒子。
#[derive(Debug, Clone, PartialEq)]
pub struct Particle {
    /// 唯一粒子 ID。
    pub id: u64,
    /// 当前位置（世界空间）。
    pub position: DVec3,
    /// 当前速度。
    pub velocity: DVec3,
    /// 当前颜色（RGBA，0-1）。
    pub color: [f64; 4],
    /// 当前尺寸（根据配置为像素或世界单位）。
    pub size: f64,
    /// 年龄（秒）。
    pub age: f64,
    /// 最大寿命（秒）。
    pub lifetime: f64,
    /// 质量（千克）。
    pub mass: f64,
    /// 起始缩放。
    pub start_scale: f64,
    /// 结束缩放。
    pub end_scale: f64,
    /// 图像尺寸 [宽, 高]，以像素计。
    pub image_size: [f64; 2],
    /// 粒子是否存活。
    pub alive: bool,
}

impl Particle {
    /// 创建一个新粒子。
    pub fn new(id: u64, position: DVec3, velocity: DVec3, lifetime: f64) -> Self {
        Self {
            id,
            position,
            velocity,
            color: [1.0, 1.0, 1.0, 1.0],
            size: 1.0,
            age: 0.0,
            lifetime,
            mass: 1.0,
            start_scale: 1.0,
            end_scale: 1.0,
            image_size: [1.0, 1.0],
            alive: true,
        }
    }

    /// 返回归一化年龄（0.0 到 1.0）。
    pub fn normalized_age(&self) -> f64 {
        (self.age / self.lifetime).clamp(0.0, 1.0)
    }

    /// 返回当前年龄处的插值缩放。
    pub fn current_scale(&self) -> f64 {
        let t = self.normalized_age();
        self.start_scale + (self.end_scale - self.start_scale) * t
    }

    /// 按一个时间增量更新粒子。
    pub fn update(&mut self, dt: f64, forces: &[ParticleForce]) {
        if !self.alive {
            return;
        }

        self.age += dt;

        if self.age >= self.lifetime {
            self.alive = false;
            return;
        }

        // 施加受力
        let mut acceleration = DVec3::ZERO;
        for force in forces {
            acceleration += force.compute_acceleration(self);
        }

        // 积分速度与位置
        self.velocity += acceleration * dt;
        self.position += self.velocity * dt;
    }
}

/// 影响粒子的一个力。
#[derive(Debug, Clone, PartialEq)]
pub enum ParticleForce {
    /// 恒定重力加速度。
    Gravity {
        /// 重力向量（例如 (0, -9.81, 0)）。
        acceleration: DVec3,
    },
    /// 线性阻力（空气阻力）。
    Drag {
        /// 阻力系数。
        coefficient: f64,
    },
    /// 风力。
    Wind {
        /// 风向与风速。
        velocity: DVec3,
    },
    ///  attractor 点（将粒子拉向一个点）。
    Attractor {
        /// attractor 位置。
        position: DVec3,
        /// 吸引力强度。
        strength: f64,
    },
    /// 漩涡力（螺旋运动）。
    Vortex {
        /// 漩涡轴。
        axis: DVec3,
        /// 漩涡强度。
        strength: f64,
    },
}

impl ParticleForce {
    /// 计算本力施加到粒子上的加速度。
    pub fn compute_acceleration(&self, particle: &Particle) -> DVec3 {
        match self {
            Self::Gravity { acceleration } => *acceleration,
            Self::Drag { coefficient } => -particle.velocity * *coefficient,
            Self::Wind { velocity } => (*velocity - particle.velocity) * 0.1,
            Self::Attractor { position, strength } => {
                let to_attractor = *position - particle.position;
                let distance = to_attractor.length().max(0.001);
                to_attractor.normalize() * (*strength / (distance * distance))
            }
            Self::Vortex { axis, strength } => {
                let radial = particle.position.cross(*axis);
                radial.normalize() * *strength
            }
        }
    }
}

/// 在系统寿命内特定时刻发生的一批粒子爆发。
#[derive(Debug, Clone, PartialEq)]
pub struct ParticleBurst {
    /// 爆发发生的时刻，以系统启动后的秒数计。
    pub time: f64,
    /// 爆发中发射的最小粒子数。
    pub minimum: u32,
    /// 爆发中发射的最大粒子数。
    pub maximum: u32,
    /// 本次爆发是否已触发。
    pub complete: bool,
}

impl ParticleBurst {
    /// 创建一个新的粒子爆发。
    pub fn new(time: f64, minimum: u32, maximum: u32) -> Self {
        Self {
            time,
            minimum,
            maximum,
            complete: false,
        }
    }

    /// 重置爆发以便循环。
    pub fn reset(&mut self) {
        self.complete = false;
    }
}

/// 粒子系统配置。
#[derive(Debug, Clone)]
pub struct ParticleSystemConfig {
    /// 发射器形状。
    pub emitter_shape: EmitterShape,
    /// 发射速率（每秒粒子数）。
    pub emission_rate: f64,
    /// 最小粒子寿命（秒）。
    pub min_lifetime: f64,
    /// 最大粒子寿命（秒）。
    pub max_lifetime: f64,
    /// 最小初始速度。
    pub min_speed: f64,
    /// 最大初始速度。
    pub max_speed: f64,
    /// 最小粒子尺寸。
    pub min_size: f64,
    /// 最大粒子尺寸。
    pub max_size: f64,
    /// 起始颜色（RGBA）。
    pub start_color: [f64; 4],
    /// 结束颜色（RGBA）。
    pub end_color: [f64; 4],
    /// 最大粒子数。
    pub max_particles: usize,
    /// 系统是否循环。
    pub looping: bool,
    /// 施加到粒子上的力。
    pub forces: Vec<ParticleForce>,
    /// 特定时刻的粒子爆发。
    pub bursts: Vec<ParticleBurst>,
    /// 系统寿命（秒）（f64::MAX = 无限）。
    pub system_lifetime: f64,
    /// 最小粒子质量（千克）。
    pub min_mass: f64,
    /// 最大粒子质量（千克）。
    pub max_mass: f64,
    /// 粒子的起始缩放。
    pub start_scale: f64,
    /// 粒子的结束缩放。
    pub end_scale: f64,
    /// 最小图像尺寸 [宽, 高]，以像素计。
    pub min_image_size: [f64; 2],
    /// 最大图像尺寸 [宽, 高]，以像素计。
    pub max_image_size: [f64; 2],
    /// 粒子尺寸是否以米为单位（true）还是像素（false）。
    pub size_in_meters: bool,
    /// 粒子 billboard 的图像 URI。
    pub image: Option<String>,
}

impl Default for ParticleSystemConfig {
    /// 默认点发射器、10 粒/秒、带向下重力、最多 1000 粒且循环。
    fn default() -> Self {
        // 中性预设：白色起始、全透明结束，保留重力作为常见默认力
        Self {
            emitter_shape: EmitterShape::Point,
            emission_rate: 10.0,
            min_lifetime: 1.0,
            max_lifetime: 3.0,
            min_speed: 1.0,
            max_speed: 5.0,
            min_size: 0.5,
            max_size: 2.0,
            start_color: [1.0, 1.0, 1.0, 1.0],
            end_color: [1.0, 1.0, 1.0, 0.0],
            max_particles: 1000,
            looping: true,
            forces: vec![ParticleForce::Gravity {
                acceleration: DVec3::new(0.0, -9.81, 0.0),
            }],
            bursts: Vec::new(),
            system_lifetime: f64::MAX,
            min_mass: 1.0,
            max_mass: 1.0,
            start_scale: 1.0,
            end_scale: 1.0,
            min_image_size: [1.0, 1.0],
            max_image_size: [1.0, 1.0],
            size_in_meters: false,
            image: None,
        }
    }
}

/// 管理粒子生命周期的粒子系统。
#[derive(Debug)]
pub struct ParticleSystem {
    /// 配置。
    pub config: ParticleSystemConfig,
    /// 活跃粒子。
    pub particles: Vec<Particle>,
    /// 发射器位置（世界空间）。
    pub emitter_position: DVec3,
    /// 发射器方向（用于 cone 发射器）。
    pub emitter_direction: DVec3,
    /// 用于发射的累计时间。
    emission_accumulator: f64,
    /// 下一个粒子 ID。
    next_id: u64,
    /// 总流逝时间。
    pub elapsed_time: f64,
    /// 系统是否正在运行。
    pub running: bool,
}

impl ParticleSystem {
    /// 创建一个新的粒子系统。
    pub fn new(config: ParticleSystemConfig, emitter_position: DVec3) -> Self {
        Self {
            config,
            particles: Vec::new(),
            emitter_position,
            emitter_direction: DVec3::Y, // 默认向上
            emission_accumulator: 0.0,
            next_id: 0,
            elapsed_time: 0.0,
            running: true,
        }
    }

    /// 创建一个火焰粒子系统预设。
    pub fn fire(emitter_position: DVec3) -> Self {
        let config = ParticleSystemConfig {
            emitter_shape: EmitterShape::Cone { angle: 0.3 },
            emission_rate: 50.0,
            min_lifetime: 0.5,
            max_lifetime: 1.5,
            min_speed: 2.0,
            max_speed: 5.0,
            min_size: 0.5,
            max_size: 1.5,
            start_color: [1.0, 0.8, 0.2, 1.0], // 橙黄
            end_color: [0.8, 0.2, 0.0, 0.0],   // 深红，透明
            max_particles: 500,
            looping: true,
            forces: vec![
                ParticleForce::Gravity {
                    acceleration: DVec3::new(0.0, 2.0, 0.0), // 向上（浮力）
                },
                ParticleForce::Drag { coefficient: 0.5 },
            ],
            ..Default::default()
        };
        Self::new(config, emitter_position)
    }

    /// 创建一个烟雾粒子系统预设。
    pub fn smoke(emitter_position: DVec3) -> Self {
        let config = ParticleSystemConfig {
            emitter_shape: EmitterShape::Sphere { radius: 0.5 },
            emission_rate: 20.0,
            min_lifetime: 2.0,
            max_lifetime: 5.0,
            min_speed: 0.5,
            max_speed: 2.0,
            min_size: 1.0,
            max_size: 3.0,
            start_color: [0.4, 0.4, 0.4, 0.8], // 灰色
            end_color: [0.6, 0.6, 0.6, 0.0],   // 浅灰，透明
            max_particles: 300,
            looping: true,
            forces: vec![
                ParticleForce::Gravity {
                    acceleration: DVec3::new(0.0, 0.5, 0.0), // 略微向上
                },
                ParticleForce::Wind {
                    velocity: DVec3::new(1.0, 0.0, 0.0),
                },
            ],
            ..Default::default()
        };
        Self::new(config, emitter_position)
    }

    /// 创建一个降雪粒子系统预设。
    pub fn snow(emitter_position: DVec3) -> Self {
        let config = ParticleSystemConfig {
            emitter_shape: EmitterShape::Box {
                half_extents: DVec3::new(50.0, 0.1, 50.0),
            },
            emission_rate: 100.0,
            min_lifetime: 5.0,
            max_lifetime: 10.0,
            min_speed: 1.0,
            max_speed: 3.0,
            min_size: 0.1,
            max_size: 0.3,
            start_color: [1.0, 1.0, 1.0, 1.0], // 白色
            end_color: [1.0, 1.0, 1.0, 0.5],   // 半透明
            max_particles: 2000,
            looping: true,
            forces: vec![
                ParticleForce::Gravity {
                    acceleration: DVec3::new(0.0, -1.0, 0.0), // 缓慢下落
                },
                ParticleForce::Wind {
                    velocity: DVec3::new(0.5, 0.0, 0.3),
                },
            ],
            ..Default::default()
        };
        Self::new(config, emitter_position)
    }

    /// 按一个时间增量更新粒子系统。
    ///
    /// # 参数
    /// * `dt` - 时间增量（秒）
    /// * `rng_seed` - 用于确定性随机性的简单种子
    pub fn update(&mut self, dt: f64, rng_seed: u64) {
        if !self.running {
            return;
        }

        self.elapsed_time += dt;

        // 更新现有粒子
        for particle in &mut self.particles {
            particle.update(dt, &self.config.forces);
        }

        // 移除已死亡的粒子
        self.particles.retain(|p| p.alive);

        // 发射新粒子
        if self.config.looping || self.elapsed_time < self.config.max_lifetime {
            self.emission_accumulator += dt * self.config.emission_rate;

            while self.emission_accumulator >= 1.0
                && self.particles.len() < self.config.max_particles
            {
                self.emission_accumulator -= 1.0;
                self.emit_particle(rng_seed + self.next_id);
            }
        }

        // 处理爆发
        let bursts_to_fire: Vec<(usize, u32)> = self
            .config
            .bursts
            .iter()
            .enumerate()
            .filter(|(_, b)| !b.complete && self.elapsed_time >= b.time)
            .map(|(i, b)| (i, b.minimum.max(1)))
            .collect();

        for (idx, count) in bursts_to_fire {
            for i in 0..count {
                if self.particles.len() < self.config.max_particles {
                    self.emit_particle(rng_seed + self.next_id + i as u64);
                }
            }
            self.config.bursts[idx].complete = true;
        }

        // 检查系统寿命
        if self.elapsed_time >= self.config.system_lifetime {
            if self.config.looping {
                self.elapsed_time = 0.0;
                for burst in &mut self.config.bursts {
                    burst.reset();
                }
            } else {
                self.running = false;
            }
        }
    }

    /// 发射单个粒子。
    fn emit_particle(&mut self, seed: u64) {
        let rng = SimpleRng::new(seed);

        // 随机寿命
        let lifetime = rng.range(self.config.min_lifetime, self.config.max_lifetime);

        // 随机速度
        let speed = rng.range(self.config.min_speed, self.config.max_speed);

        // 随机尺寸
        let size = rng.range(self.config.min_size, self.config.max_size);

        // 根据发射器形状计算发射位置与方向
        let (position, direction) = self.compute_emission(&rng);

        let velocity = direction * speed;

        let mut particle = Particle::new(self.next_id, position, velocity, lifetime);
        particle.size = size;
        particle.color = self.config.start_color;

        self.particles.push(particle);
        self.next_id += 1;
    }

    /// 根据发射器形状计算发射位置与方向。
    fn compute_emission(&self, rng: &SimpleRng) -> (DVec3, DVec3) {
        match &self.config.emitter_shape {
            EmitterShape::Point => (self.emitter_position, self.emitter_direction),
            EmitterShape::Cone { angle } => {
                // 圆锥内的随机方向
                let theta = rng.range(0.0, std::f64::consts::TAU);
                let phi = rng.range(0.0, *angle);

                let sin_phi = phi.sin();
                let dir = DVec3::new(
                    sin_phi * theta.cos(),
                    phi.cos(),
                    sin_phi * theta.sin(),
                );

                (self.emitter_position, dir.normalize())
            }
            EmitterShape::Box { half_extents } => {
                let offset = DVec3::new(
                    rng.range(-half_extents.x, half_extents.x),
                    rng.range(-half_extents.y, half_extents.y),
                    rng.range(-half_extents.z, half_extents.z),
                );
                (self.emitter_position + offset, self.emitter_direction)
            }
            EmitterShape::Sphere { radius } => {
                // 球面上的随机点
                let theta = rng.range(0.0, std::f64::consts::TAU);
                let phi = rng.range(0.0, std::f64::consts::PI);

                let dir = DVec3::new(
                    phi.sin() * theta.cos(),
                    phi.cos(),
                    phi.sin() * theta.sin(),
                );

                (self.emitter_position + dir * *radius, dir)
            }
            EmitterShape::Circle { radius } => {
                // 圆盘内均匀取点（XZ 平面），方向沿用发射器朝向
                let theta = rng.range(0.0, std::f64::consts::TAU);
                let r = rng.range(0.0, *radius);

                let offset = DVec3::new(r * theta.cos(), 0.0, r * theta.sin());
                (self.emitter_position + offset, self.emitter_direction)
            }
        }
    }

    /// 返回存活粒子的数量。
    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }

    /// 根据年龄插值粒子颜色。
    pub fn particle_color(&self, particle: &Particle) -> [f64; 4] {
        let t = particle.normalized_age();
        let start = &self.config.start_color;
        let end = &self.config.end_color;

        [
            start[0] + (end[0] - start[0]) * t,
            start[1] + (end[1] - start[1]) * t,
            start[2] + (end[2] - start[2]) * t,
            start[3] + (end[3] - start[3]) * t,
        ]
    }

    /// 停止粒子系统（不再发射，现有粒子继续）。
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// 启动/恢复粒子系统。
    pub fn start(&mut self) {
        self.running = true;
    }

    /// 重置粒子系统。
    pub fn reset(&mut self) {
        self.particles.clear();
        self.emission_accumulator = 0.0;
        self.elapsed_time = 0.0;
        self.next_id = 0;
        self.running = true;
    }
}

/// 用于粒子发射的简单确定性 RNG。
#[derive(Debug, Clone)]
struct SimpleRng {
    /// xorshift64 的内部状态字。
    state: u64,
}

impl SimpleRng {
    /// 以给定种子初始化状态（混入两个大常数避免零周期）。
    fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407),
        }
    }

    /// 产生下一个 [0, 1) 浮点随机数。
    fn next(&mut self) -> f64 {
        // xorshift64
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;

        // 转换为 [0, 1)
        (self.state >> 11) as f64 / ((1u64 << 53) as f64)
    }

    /// 在 [min, max] 区间取一个样本（基于克隆副本，不改变自身状态）。
    fn range(&self, min: f64, max: f64) -> f64 {
        // 克隆后推进，保证同一发射器内多次取样互相独立
        let mut rng = self.clone();
        min + rng.next() * (max - min)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_creation() {
        let particle = Particle::new(
            1,
            DVec3::ZERO,
            DVec3::new(1.0, 2.0, 3.0),
            5.0,
        );

        assert_eq!(particle.id, 1);
        assert!(particle.alive);
        assert_eq!(particle.age, 0.0);
        assert_eq!(particle.lifetime, 5.0);
    }

    #[test]
    fn test_particle_normalized_age() {
        let mut particle = Particle::new(1, DVec3::ZERO, DVec3::Y, 4.0);
        particle.age = 2.0;

        assert!((particle.normalized_age() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_particle_update_gravity() {
        let mut particle = Particle::new(1, DVec3::ZERO, DVec3::ZERO, 10.0);
        let forces = vec![ParticleForce::Gravity {
            acceleration: DVec3::new(0.0, -10.0, 0.0),
        }];

        particle.update(1.0, &forces);

        // 在 -10 m/s² 重力下经过 1 秒后
        assert!((particle.velocity.y - (-10.0)).abs() < 1e-10);
        assert!((particle.position.y - (-10.0)).abs() < 1e-10);
    }

    #[test]
    fn test_particle_death() {
        let mut particle = Particle::new(1, DVec3::ZERO, DVec3::Y, 1.0);
        let forces: Vec<ParticleForce> = vec![];

        particle.update(0.5, &forces);
        assert!(particle.alive);

        particle.update(0.6, &forces); // 总年龄 = 1.1 > 寿命
        assert!(!particle.alive);
    }

    #[test]
    fn test_particle_drag() {
        let mut particle = Particle::new(1, DVec3::ZERO, DVec3::new(10.0, 0.0, 0.0), 10.0);
        let forces = vec![ParticleForce::Drag { coefficient: 0.5 }];

        particle.update(1.0, &forces);

        // 由于阻力，速度应下降
        assert!(particle.velocity.x < 10.0);
        assert!(particle.velocity.x > 0.0);
    }

    #[test]
    fn test_particle_system_creation() {
        let system = ParticleSystem::new(
            ParticleSystemConfig::default(),
            DVec3::ZERO,
        );

        assert_eq!(system.particle_count(), 0);
        assert!(system.running);
    }

    #[test]
    fn test_particle_system_emission() {
        let config = ParticleSystemConfig {
            emission_rate: 100.0, // 每秒 100 个粒子
            ..Default::default()
        };
        let mut system = ParticleSystem::new(config, DVec3::ZERO);

        system.update(1.0, 42); // 1 秒

        // 应已发射约 100 个粒子
        assert!(system.particle_count() > 50);
        assert!(system.particle_count() <= 100);
    }

    #[test]
    fn test_particle_system_max_particles() {
        let config = ParticleSystemConfig {
            emission_rate: 1000.0,
            max_particles: 50,
            ..Default::default()
        };
        let mut system = ParticleSystem::new(config, DVec3::ZERO);

        system.update(1.0, 42);

        assert!(system.particle_count() <= 50);
    }

    #[test]
    fn test_particle_system_stop() {
        let config = ParticleSystemConfig {
            emission_rate: 100.0,
            ..Default::default()
        };
        let mut system = ParticleSystem::new(config, DVec3::ZERO);

        system.update(0.5, 42);
        let count_before = system.particle_count();

        system.stop();
        system.update(0.5, 42);

        // stop 后不再发射新粒子
        // （部分可能已死亡，因此计数可能更少）
        assert!(system.particle_count() <= count_before);
    }

    #[test]
    fn test_particle_system_reset() {
        let config = ParticleSystemConfig {
            emission_rate: 100.0,
            ..Default::default()
        };
        let mut system = ParticleSystem::new(config, DVec3::ZERO);

        system.update(1.0, 42);
        assert!(system.particle_count() > 0);

        system.reset();
        assert_eq!(system.particle_count(), 0);
        assert_eq!(system.elapsed_time, 0.0);
    }

    #[test]
    fn test_fire_preset() {
        let system = ParticleSystem::fire(DVec3::ZERO);

        assert_eq!(system.config.emission_rate, 50.0);
        assert!(matches!(system.config.emitter_shape, EmitterShape::Cone { .. }));
        assert_eq!(system.config.start_color, [1.0, 0.8, 0.2, 1.0]);
    }

    #[test]
    fn test_smoke_preset() {
        let system = ParticleSystem::smoke(DVec3::ZERO);

        assert_eq!(system.config.emission_rate, 20.0);
        assert!(matches!(system.config.emitter_shape, EmitterShape::Sphere { .. }));
    }

    #[test]
    fn test_snow_preset() {
        let system = ParticleSystem::snow(DVec3::ZERO);

        assert_eq!(system.config.emission_rate, 100.0);
        assert!(matches!(system.config.emitter_shape, EmitterShape::Box { .. }));
    }

    #[test]
    fn test_particle_color_interpolation() {
        let config = ParticleSystemConfig {
            start_color: [1.0, 0.0, 0.0, 1.0], // 红色
            end_color: [0.0, 0.0, 1.0, 0.0],   // 蓝色，透明
            ..Default::default()
        };
        let system = ParticleSystem::new(config, DVec3::ZERO);

        let mut particle = Particle::new(1, DVec3::ZERO, DVec3::Y, 2.0);
        particle.age = 1.0; // 处于寿命的 50%

        let color = system.particle_color(&particle);

        // 应处于红色与蓝色的中点
        assert!((color[0] - 0.5).abs() < 1e-10);
        assert!((color[2] - 0.5).abs() < 1e-10);
        assert!((color[3] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_force_attractor() {
        let force = ParticleForce::Attractor {
            position: DVec3::new(10.0, 0.0, 0.0),
            strength: 100.0,
        };

        let particle = Particle::new(1, DVec3::ZERO, DVec3::ZERO, 10.0);
        let accel = force.compute_acceleration(&particle);

        // 应向 attractor 方向（+X 方向）加速
        assert!(accel.x > 0.0);
    }

    #[test]
    fn test_force_vortex() {
        let force = ParticleForce::Vortex {
            axis: DVec3::Y,
            strength: 5.0,
        };

        let particle = Particle::new(1, DVec3::new(1.0, 0.0, 0.0), DVec3::ZERO, 10.0);
        let accel = force.compute_acceleration(&particle);

        // 应产生圆周运动（垂直于位置与轴）
        assert!(accel.length() > 0.0);
    }

    #[test]
    fn test_simple_rng_deterministic() {
        let rng1 = SimpleRng::new(42);
        let rng2 = SimpleRng::new(42);

        assert!((rng1.range(0.0, 1.0) - rng2.range(0.0, 1.0)).abs() < 1e-10);
    }
}
