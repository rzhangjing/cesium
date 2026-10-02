//! 粒子系统：发射器、粒子、爆发与系统生命周期。
//!
//! 支持盒/圆/球/圆锥四类发射器的确定性伪随机播种发射，
//! 逐粒子推进位置、寿命、颜色与缩放插值，并统一管理爆发触发、
//! 循环寿命与系统重置。

use glam::DVec3;
use std::f64::consts::PI;

const TWO_PI: f64 = 2.0 * PI;

// ============================================================================
// 发射器
// ============================================================================

/// 粒子发射器类型。
#[derive(Debug, Clone, PartialEq)]
pub enum ParticleEmitter {
    /// 在立方体内发射。速度从中心向外辐射。
    Box {
        /// 宽、高、深尺寸（米）。
        dimensions: DVec3,
    },
    /// 从圆发射。速度沿 +Z。
    Circle {
        /// 半径（米）。
        radius: f64,
    },
    /// 在球内发射。速度从中心向外辐射。
    Sphere {
        /// 半径（米）。
        radius: f64,
    },
    /// 从圆锥顶点发射。速度指向底面。
    Cone {
        /// 圆锥半角（弧度）。
        angle: f64,
    },
}

impl Default for ParticleEmitter {
    /// 默认发射器为半径 0.5 的圆形发射器。
    fn default() -> Self {
        // 圆形发射器最通用，作为系统默认形状
        Self::Circle { radius: 0.5 }
    }
}

impl ParticleEmitter {
    /// 计算新粒子的初始位置与速度。
    ///
    /// 使用基于种子的简单确定性伪随机以保证可重现性。
    pub fn emit(&self, seed: f64) -> (DVec3, DVec3) {
        match self {
            Self::Box { dimensions } => {
                // 盒内均匀取点，速度沿位置方向向外辐射
                let half = *dimensions * 0.5;
                let x = lerp_signed(-half.x, half.x, frac(seed * 7.13));
                let y = lerp_signed(-half.y, half.y, frac(seed * 3.77));
                let z = lerp_signed(-half.z, half.z, frac(seed * 5.91));
                let pos = DVec3::new(x, y, z);
                let vel = if pos.length() > 1e-10 {
                    pos.normalize()
                } else {
                    DVec3::Z
                };
                (pos, vel)
            }
            Self::Circle { radius } => {
                // 圆盘内极坐标取点，速度沿 +Z
                let theta = frac(seed * 6.28) * TWO_PI;
                let rad = frac(seed * 2.17) * radius;
                let x = rad * theta.cos();
                let y = rad * theta.sin();
                (DVec3::new(x, y, 0.0), DVec3::Z)
            }
            Self::Sphere { radius } => {
                // 球内球坐标取点，速度沿位置向外
                let theta = frac(seed * 4.31) * TWO_PI;
                let phi = frac(seed * 2.79) * PI;
                let rad = frac(seed * 1.53) * radius;
                let x = rad * theta.cos() * phi.sin();
                let y = rad * theta.sin() * phi.sin();
                let z = rad * phi.cos();
                let pos = DVec3::new(x, y, z);
                let vel = if pos.length() > 1e-10 {
                    pos.normalize()
                } else {
                    DVec3::Z
                };
                (pos, vel)
            }
            Self::Cone { angle } => {
                // 圆锥底面取点，速度由顶点指向底面
                let cone_radius = angle.tan();
                let theta = frac(seed * 5.47) * TWO_PI;
                let rad = frac(seed * 3.23) * cone_radius;
                let x = rad * theta.cos();
                let y = rad * theta.sin();
                let vel = DVec3::new(x, y, 1.0).normalize();
                (DVec3::ZERO, vel)
            }
        }
    }
}

// ============================================================================
// 粒子
// ============================================================================

/// 系统中的单个粒子。
#[derive(Debug, Clone)]
pub struct Particle {
    /// 质量（千克）。
    pub mass: f64,
    /// 世界坐标中的位置。
    pub position: DVec3,
    /// 世界坐标中的速度（m/s）。
    pub velocity: DVec3,
    /// 总寿命（秒）。
    pub life: f64,
    /// 出生时的颜色 [R, G, B, A]。
    pub start_color: [f64; 4],
    /// 死亡时的颜色 [R, G, B, A]。
    pub end_color: [f64; 4],
    /// 出生时的缩放。
    pub start_scale: f64,
    /// 死亡时的缩放。
    pub end_scale: f64,
    /// 图像尺寸 [宽, 高]，以像素计。
    pub image_size: [f64; 2],
    /// 当前年龄（秒）。
    pub age: f64,
}

impl Particle {
    /// 创建一个新粒子。
    pub fn new(position: DVec3, velocity: DVec3, life: f64) -> Self {
        Self {
            mass: 1.0,
            position,
            velocity,
            life,
            start_color: [1.0, 1.0, 1.0, 1.0],
            end_color: [1.0, 1.0, 1.0, 1.0],
            start_scale: 1.0,
            end_scale: 1.0,
            image_size: [1.0, 1.0],
            age: 0.0,
        }
    }

    /// 获取归一化年龄 [0, 1]。
    pub fn normalized_age(&self) -> f64 {
        // 零/负寿命视为已到期；否则 age/life 限到 [0,1]
        if self.life <= 0.0 {
            return 1.0;
        }
        (self.age / self.life).clamp(0.0, 1.0)
    }

    /// 粒子是否仍存活。
    pub fn is_alive(&self) -> bool {
        self.age < self.life
    }

    /// 按 dt 秒更新粒子。若仍存活则返回 true。
    pub fn update(&mut self, dt: f64) -> bool {
        // 施加速度
        self.position += self.velocity * dt;
        // 年龄
        self.age += dt;
        self.age < self.life
    }

    /// 获取当前年龄处的插值颜色。
    pub fn current_color(&self) -> [f64; 4] {
        let t = self.normalized_age();
        [
            lerp_signed(self.start_color[0], self.end_color[0], t),
            lerp_signed(self.start_color[1], self.end_color[1], t),
            lerp_signed(self.start_color[2], self.end_color[2], t),
            lerp_signed(self.start_color[3], self.end_color[3], t),
        ]
    }

    /// 获取当前年龄处的插值缩放。
    pub fn current_scale(&self) -> f64 {
        let t = self.normalized_age();
        lerp_signed(self.start_scale, self.end_scale, t)
    }
}

// ============================================================================
// ParticleBurst（粒子爆发）
// ============================================================================

/// 在特定时刻发生的一批粒子爆发。
#[derive(Debug, Clone, PartialEq)]
pub struct ParticleBurst {
    /// 系统启动后的秒数。
    pub time: f64,
    /// 最小粒子数。
    pub minimum: u32,
    /// 最大粒子数。
    pub maximum: u32,
    /// 本次爆发是否已触发。
    pub complete: bool,
}

impl ParticleBurst {
    /// 创建一个新的爆发。
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

// ============================================================================
// ParticleSystem（粒子系统）
// ============================================================================

/// 粒子系统的配置与状态。
#[derive(Debug, Clone)]
pub struct ParticleSystem {
    /// 系统是否可见。
    pub show: bool,
    /// 是否循环爆发。
    pub loop: bool,
    /// 发射器类型。
    pub emitter: ParticleEmitter,
    /// 发射速率（每秒粒子数）。
    pub emission_rate: f64,
    /// 爆发配置。
    pub bursts: Vec<ParticleBurst>,
    /// 起始颜色 [R, G, B, A]。
    pub start_color: [f64; 4],
    /// 结束颜色 [R, G, B, A]。
    pub end_color: [f64; 4],
    /// 起始缩放。
    pub start_scale: f64,
    /// 结束缩放。
    pub end_scale: f64,
    /// 最小速度（m/s）。
    pub minimum_speed: f64,
    /// 最大速度（m/s）。
    pub maximum_speed: f64,
    /// 最小粒子寿命（秒）。
    pub minimum_particle_life: f64,
    /// 最大粒子寿命（秒）。
    pub maximum_particle_life: f64,
    /// 最小质量（kg）。
    pub minimum_mass: f64,
    /// 最大质量（kg）。
    pub maximum_mass: f64,
    /// 最小图像尺寸 [w, h]。
    pub minimum_image_size: [f64; 2],
    /// 最大图像尺寸 [w, h]。
    pub maximum_image_size: [f64; 2],
    /// 尺寸是否以米为单位（而非像素）。
    pub size_in_meters: bool,
    /// 系统寿命（秒）。
    pub lifetime: f64,
    /// 模型矩阵（4x4 列主序）。
    pub model_matrix: [f64; 16],
    /// 发射器模型矩阵。
    pub emitter_model_matrix: [f64; 16],
    /// 图像 URI。
    pub image: Option<String>,

    // 运行时状态
    /// 活跃粒子。
    particles: Vec<Particle>,
    /// 当前系统时间。
    current_time: f64,
    /// 累积的小数部分粒子。
    carry_over: f64,
    /// 系统是否已完成。
    is_complete: bool,
    /// 用于确定性发射的种子计数器。
    seed_counter: f64,
}

impl Default for ParticleSystem {
    /// 默认可见、循环、每秒 5 粒、寿命近乎无限。
    fn default() -> Self {
        // 各颜色/缩放/速度/寿命取中性默认值，变换矩阵为单位阵
        Self {
            show: true,
            loop: true,
            emitter: ParticleEmitter::default(),
            emission_rate: 5.0,
            bursts: Vec::new(),
            start_color: [1.0, 1.0, 1.0, 1.0],
            end_color: [1.0, 1.0, 1.0, 1.0],
            start_scale: 1.0,
            end_scale: 1.0,
            minimum_speed: 1.0,
            maximum_speed: 1.0,
            minimum_particle_life: 5.0,
            maximum_particle_life: 5.0,
            minimum_mass: 1.0,
            maximum_mass: 1.0,
            minimum_image_size: [1.0, 1.0],
            maximum_image_size: [1.0, 1.0],
            size_in_meters: false,
            lifetime: f64::MAX,
            model_matrix: identity_matrix(),
            emitter_model_matrix: identity_matrix(),
            image: None,
            particles: Vec::new(),
            current_time: 0.0,
            carry_over: 0.0,
            is_complete: false,
            seed_counter: 0.0,
        }
    }
}

impl ParticleSystem {
    /// 创建一个新的粒子系统。
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取活跃粒子。
    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }

    /// 获取活跃粒子数量。
    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }

    /// 系统是否已完成其寿命。
    pub fn is_complete(&self) -> bool {
        self.is_complete
    }

    /// 获取当前系统时间。
    pub fn current_time(&self) -> f64 {
        self.current_time
    }

    /// 按 dt 秒更新粒子系统。
    pub fn update(&mut self, dt: f64) {
        if !self.show || self.is_complete {
            return;
        }

        self.current_time += dt;

        // 检查寿命
        if self.current_time >= self.lifetime {
            if self.loop {
                self.current_time = 0.0;
                for burst in &mut self.bursts {
                    burst.reset();
                }
            } else {
                self.is_complete = true;
                return;
            }
        }

        // 基于速率发射新粒子
        let to_emit = self.emission_rate * dt + self.carry_over;
        let count = to_emit.floor() as usize;
        self.carry_over = to_emit - count as f64;

        for _ in 0..count {
            self.emit_particle();
        }

        // 处理爆发
        for burst in &mut self.bursts {
            if !burst.complete && self.current_time >= burst.time {
                let burst_count = if burst.maximum > burst.minimum {
                    burst.minimum
                        + (frac(self.seed_counter * 1.37) * (burst.maximum - burst.minimum) as f64)
                            .floor() as u32
                } else {
                    burst.minimum
                };
                for _ in 0..burst_count {
                    self.emit_particle();
                }
                burst.complete = true;
            }
        }

        // 更新现有粒子
        self.particles.retain_mut(|p| p.update(dt));
    }

    /// 发射单个粒子。
    fn emit_particle(&mut self) {
        self.seed_counter += 1.0;
        let seed = self.seed_counter;

        let (mut pos, mut vel) = self.emitter.emit(seed);

        // 施加速度
        let speed = lerp_signed(
            self.minimum_speed,
            self.maximum_speed,
            frac(seed * 1.71),
        );
        vel *= speed;

        // 施加拉命
        let life = lerp_signed(
            self.minimum_particle_life,
            self.maximum_particle_life,
            frac(seed * 2.31),
        );

        let mut particle = Particle::new(pos, vel, life);
        particle.mass = lerp_signed(self.minimum_mass, self.maximum_mass, frac(seed * 3.11));
        particle.start_color = self.start_color;
        particle.end_color = self.end_color;
        particle.start_scale = self.start_scale;
        particle.end_scale = self.end_scale;
        particle.image_size = [
            lerp_signed(self.minimum_image_size[0], self.maximum_image_size[0], frac(seed * 4.13)),
            lerp_signed(self.minimum_image_size[1], self.maximum_image_size[1], frac(seed * 5.17)),
        ];

        // 将发射器模型矩阵的平移施加到位置
        let em = &self.emitter_model_matrix;
        pos = DVec3::new(
            pos.x + em[12],
            pos.y + em[13],
            pos.z + em[14],
        );
        particle.position = pos;

        self.particles.push(particle);
    }

    /// 移除所有粒子。
    pub fn clear(&mut self) {
        self.particles.clear();
    }

    /// 将系统重置为初始状态。
    pub fn reset(&mut self) {
        self.particles.clear();
        self.current_time = 0.0;
        self.carry_over = 0.0;
        self.is_complete = false;
        self.seed_counter = 0.0;
        for burst in &mut self.bursts {
            burst.reset();
        }
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 在 [a, b] 间按 t∈[0,1] 线性插值（允许越界外推）。
fn lerp_signed(a: f64, b: f64, t: f64) -> f64 {
    // 支持 t 超界时的有符号外推，用于确定性伪随机采样
    a + (b - a) * t
}

/// 取 x 的小数部分 [0, 1)。
fn frac(x: f64) -> f64 {
    // 用 floor 提取小数位，作为伪随机归一化手段
    x - x.floor()
}

/// 构造 4x4 列主序单位矩阵。
fn identity_matrix() -> [f64; 16] {
    // 对角为 1、其余为 0 的中性变换
    [
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ]
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_lifecycle() {
        let mut p = Particle::new(DVec3::ZERO, DVec3::new(1.0, 0.0, 0.0), 2.0);
        assert!(p.is_alive());
        assert_eq!(p.age, 0.0);

        assert!(p.update(1.0));
        assert!((p.age - 1.0).abs() < 1e-10);
        assert!((p.position.x - 1.0).abs() < 1e-10);
        assert!((p.normalized_age() - 0.5).abs() < 1e-10);

        assert!(!p.update(1.5));
        assert!(!p.is_alive());
    }

    #[test]
    fn test_particle_color_interpolation() {
        let mut p = Particle::new(DVec3::ZERO, DVec3::ZERO, 4.0);
        p.start_color = [1.0, 0.0, 0.0, 1.0];
        p.end_color = [0.0, 0.0, 1.0, 0.0];
        p.age = 2.0; // 50%

        let color = p.current_color();
        assert!((color[0] - 0.5).abs() < 1e-10);
        assert!((color[2] - 0.5).abs() < 1e-10);
        assert!((color[3] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_particle_scale_interpolation() {
        let mut p = Particle::new(DVec3::ZERO, DVec3::ZERO, 10.0);
        p.start_scale = 2.0;
        p.end_scale = 4.0;
        p.age = 5.0;

        assert!((p.current_scale() - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_box_emitter() {
        let emitter = ParticleEmitter::Box {
            dimensions: DVec3::new(2.0, 2.0, 2.0),
        };
        let (pos, vel) = emitter.emit(0.5);
        // 位置应处于 [-1, 1]^3 内
        assert!(pos.x.abs() <= 1.0);
        assert!(pos.y.abs() <= 1.0);
        assert!(pos.z.abs() <= 1.0);
        // 速度应已归一化
        assert!((vel.length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_circle_emitter() {
        let emitter = ParticleEmitter::Circle { radius: 2.0 };
        let (pos, vel) = emitter.emit(0.3);
        // 位置在 XY 平面
        assert!((pos.z).abs() < 1e-10);
        // 在半径内
        assert!((pos.x * pos.x + pos.y * pos.y).sqrt() <= 2.0 + 1e-10);
        // 速度沿 Z
        assert!((vel - DVec3::Z).length() < 1e-10);
    }

    #[test]
    fn test_sphere_emitter() {
        let emitter = ParticleEmitter::Sphere { radius: 3.0 };
        let (pos, vel) = emitter.emit(0.7);
        assert!(pos.length() <= 3.0 + 1e-10);
        assert!((vel.length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cone_emitter() {
        let emitter = ParticleEmitter::Cone {
            angle: std::f64::consts::FRAC_PI_4,
        };
        let (pos, vel) = emitter.emit(0.9);
        // 位置在原点
        assert!(pos.length() < 1e-10);
        // 速度的 Z 为正
        assert!(vel.z > 0.0);
        assert!((vel.length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_particle_burst() {
        let mut burst = ParticleBurst::new(2.0, 10, 20);
        assert!(!burst.complete);
        burst.complete = true;
        burst.reset();
        assert!(!burst.complete);
    }

    #[test]
    fn test_particle_system_emission() {
        let mut sys = ParticleSystem::new();
        sys.emission_rate = 100.0;
        sys.minimum_particle_life = 1.0;
        sys.maximum_particle_life = 1.0;

        sys.update(0.1); // 应发射约 10 个粒子
        assert!(sys.particle_count() >= 9);
        assert!(sys.particle_count() <= 11);
    }

    #[test]
    fn test_particle_system_lifetime() {
        let mut sys = ParticleSystem::new();
        sys.lifetime = 1.0;
        sys.loop = false;
        sys.emission_rate = 10.0;

        sys.update(0.5);
        assert!(!sys.is_complete());

        sys.update(0.6);
        assert!(sys.is_complete());
    }

    #[test]
    fn test_particle_system_loop() {
        let mut sys = ParticleSystem::new();
        sys.lifetime = 1.0;
        sys.loop = true;
        sys.emission_rate = 10.0;

        sys.update(1.5); // 超过寿命，应循环
        assert!(!sys.is_complete());
        assert!(sys.current_time() < 1.0);
    }

    #[test]
    fn test_particle_system_burst() {
        let mut sys = ParticleSystem::new();
        sys.emission_rate = 0.0; // 无持续发射
        sys.bursts.push(ParticleBurst::new(0.5, 20, 20));
        sys.minimum_particle_life = 10.0;
        sys.maximum_particle_life = 10.0;

        sys.update(0.3);
        assert_eq!(sys.particle_count(), 0);

        sys.update(0.3); // 现在为 0.6，位于 0.5 的爆发应触发
        assert_eq!(sys.particle_count(), 20);
    }

    #[test]
    fn test_particle_system_reset() {
        let mut sys = ParticleSystem::new();
        sys.emission_rate = 50.0;
        sys.update(1.0);
        assert!(sys.particle_count() > 0);

        sys.reset();
        assert_eq!(sys.particle_count(), 0);
        assert_eq!(sys.current_time(), 0.0);
        assert!(!sys.is_complete());
    }

    #[test]
    fn test_particle_system_clear() {
        let mut sys = ParticleSystem::new();
        sys.emission_rate = 50.0;
        sys.update(1.0);
        sys.clear();
        assert_eq!(sys.particle_count(), 0);
    }

    #[test]
    fn test_particle_system_hidden() {
        let mut sys = ParticleSystem::new();
        sys.show = false;
        sys.emission_rate = 100.0;
        sys.update(1.0);
        assert_eq!(sys.particle_count(), 0);
    }

    #[test]
    fn test_particles_die_over_time() {
        let mut sys = ParticleSystem::new();
        sys.emission_rate = 10.0;
        sys.minimum_particle_life = 0.5;
        sys.maximum_particle_life = 0.5;

        sys.update(0.1); // 发射约 1 个
        let count_after_emit = sys.particle_count();
        assert!(count_after_emit > 0);

        // 等待粒子死亡
        for _ in 0..10 {
            sys.update(0.1);
        }
        // 旧粒子应已死亡，新粒子已发射
        // 在 life=0.5 且 10 次 0.1 更新下，首帧的粒子已死亡
        assert!(sys.particle_count() < count_after_emit + 10);
    }
}
