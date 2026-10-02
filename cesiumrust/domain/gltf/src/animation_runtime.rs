//! 用于 glTF 2.0 的骨骼动画运行时系统。
//!
//! 提供动画求值（样条插值）、蒙皮（关节矩阵计算）以及 morph target 混合。

use crate::gltf_model::{Animation, AnimationPath, Interpolation};
use glam::{DMat4, DQuat, DVec3};

/// 动画播放状态。
///
/// 映射到 CesiumJS `ModelAnimationState`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnimationState {
    /// 动画已停止。
    #[default]
    Stopped,
    /// 动画正在播放。
    Playing,
    /// 动画已暂停。
    Paused,
}

/// 动画循环模式。
///
/// 映射到 CesiumJS `ModelAnimationLoop`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnimationLoop {
    /// 播放一次后停止。
    #[default]
    None,
    /// 持续循环。
    Repeat,
    /// 往复（先正放后倒放）。
    MirroredRepeat,
}

/// 带播放控制的运行时动画实例。
///
/// 封装一个命名动画的当前播放位置、速度与循环模式。
#[derive(Debug, Clone)]
pub struct RuntimeAnimation {
    /// 动画名称。
    pub name: Option<String>,
    /// 当前播放状态。
    pub state: AnimationState,
    /// 循环模式。
    pub loop_mode: AnimationLoop,
    /// 播放速度乘数。
    pub multiplier: f64,
    /// 是否反向播放。
    pub reverse: bool,
    /// 当前本地时间（秒）。
    pub local_time: f64,
    /// 动画时长（秒）。
    pub duration: f64,
    /// 开始前的延迟（秒）。
    pub delay: f64,
    /// 停止时是否移除。
    pub remove_on_stop: bool,
    /// 是否将动画钳制到其时间范围。
    pub clamp_animations: bool,
}

impl RuntimeAnimation {
    /// 由一个 glTF animation 创建新的运行时动画。
    pub fn from_gltf(animation: &Animation, duration: f64) -> Self {
        // 初始为停止态、正放、单倍速
        Self {
            name: animation.name.clone(),
            state: AnimationState::Stopped,
            loop_mode: AnimationLoop::None,
            multiplier: 1.0,
            reverse: false,
            local_time: 0.0,
            duration,
            delay: 0.0,
            remove_on_stop: false,
            clamp_animations: true,
        }
    }

    /// 开始播放动画。
    pub fn play(&mut self) {
        self.state = AnimationState::Playing;
    }

    /// 暂停动画。
    pub fn pause(&mut self) {
        // 仅播放态可转为暂停
        if self.state == AnimationState::Playing {
            self.state = AnimationState::Paused;
        }
    }

    /// 停止动画并重置时间。
    pub fn stop(&mut self) {
        self.state = AnimationState::Stopped;
        self.local_time = 0.0;
    }

    /// 将动画推进 delta_time 秒。
    /// 若动画仍活跃则返回 true。
    pub fn advance(&mut self, delta_time: f64) -> bool {
        // 非播放态：直接返回是否仍活跃（暂停也算活跃）
        if self.state != AnimationState::Playing {
            return self.state != AnimationState::Stopped;
        }

        // 反向播放时有效增量为负
        let effective_delta = if self.reverse {
            -delta_time * self.multiplier
        } else {
            delta_time * self.multiplier
        };

        // 将带符号增量累加到本地时间
        self.local_time += effective_delta;

        // 处理循环
        if self.duration > 0.0 {
            match self.loop_mode {
                AnimationLoop::None => {
                    // 到达首尾边界则钳制并停止
                    if self.local_time >= self.duration || self.local_time < 0.0 {
                        self.local_time = self.local_time.clamp(0.0, self.duration);
                        self.state = AnimationState::Stopped;
                        return false;
                    }
                }
                AnimationLoop::Repeat => {
                    // 取模回绕到 [0, duration)
                    self.local_time = self.local_time.rem_euclid(self.duration);
                }
                AnimationLoop::MirroredRepeat => {
                    // 双倍周期内折返：后半段倒放
                    let cycle = self.duration * 2.0;
                    let t = self.local_time.rem_euclid(cycle);
                    self.local_time = if t > self.duration {
                        // 后半段折返为倒放
                        cycle - t
                    } else {
                        t
                    };
                }
            }
        }

        true
    }

    /// 获取生效时间（根据设置钳制或环绕）。
    pub fn effective_time(&self) -> f64 {
        // 钳制模式截断到 [0,duration]，否则按 duration 环绕
        if self.clamp_animations {
            self.local_time.clamp(0.0, self.duration)
        } else if self.duration > 0.0 {
            self.local_time.rem_euclid(self.duration)
        } else {
            self.local_time
        }
    }
}

/// 用于动画插值的关键帧样条。
///
/// 映射到 CesiumJS 样条类（LinearSpline、QuaternionSpline、HermiteSpline、SteppedSpline）
#[derive(Debug, Clone)]
pub enum AnimationSpline {
    /// 常量值（单个关键帧）。
    Constant(ConstantSpline),
    /// 阶跃插值（保持值直到下一个关键帧）。
    Step(StepSpline),
    /// 线性插值。
    Linear(LinearSpline),
    /// 四元数 slerp 插值。
    QuaternionSlerp(QuaternionSpline),
    /// 三次 Hermite 样条插值。
    CubicSpline(CubicSpline),
}

/// 常量样条（单个关键帧）。
#[derive(Debug, Clone)]
pub struct ConstantSpline {
    /// 常量值。
    pub value: Vec<f64>,
}

/// 阶跃样条（无插值，保持上一个值）。
#[derive(Debug, Clone)]
pub struct StepSpline {
    /// 关键帧时间。
    pub times: Vec<f64>,
    /// 关键帧值（展平）。
    pub values: Vec<f64>,
    /// 每个关键帧的分量数。
    pub components: usize,
}

/// 线性插值样条。
#[derive(Debug, Clone)]
pub struct LinearSpline {
    /// 关键帧时间。
    pub times: Vec<f64>,
    /// 关键帧值（展平）。
    pub values: Vec<f64>,
    /// 每个关键帧的分量数。
    pub components: usize,
}

/// 四元数 slerp 样条。
#[derive(Debug, Clone)]
pub struct QuaternionSpline {
    /// 关键帧时间。
    pub times: Vec<f64>,
    /// 每个关键帧的四元数值 [x, y, z, w]（展平）。
    pub values: Vec<f64>,
}

/// 三次 Hermite 样条。
///
/// 映射到 CesiumJS `HermiteSpline`
#[derive(Debug, Clone)]
pub struct CubicSpline {
    /// 关键帧时间。
    pub times: Vec<f64>,
    /// 关键帧值（展平）。
    pub values: Vec<f64>,
    /// 入切线（展平，比 values 少一个）。
    pub in_tangents: Vec<f64>,
    /// 出切线（展平，比 values 少一个）。
    pub out_tangents: Vec<f64>,
    /// 每个关键帧的分量数。
    pub components: usize,
}

impl AnimationSpline {
    /// 由关键帧数据创建样条。
    ///
    /// 映射到 CesiumJS `ModelAnimationChannel.createSpline`
    pub fn from_keyframes(
        times: Vec<f64>,
        values: Vec<f64>,
        interpolation: Interpolation,
        path: AnimationPath,
        components: usize,
    ) -> Self {
        // 常量分支：空值回退为全零分量
        if times.len() <= 1 {
            return Self::Constant(ConstantSpline {
                value: if values.is_empty() {
                    vec![0.0; components]
                } else {
                    values[..components.min(values.len())].to_vec()
                },
            });
        }

        // 按插值类型分派构造对应样条
        match interpolation {
            Interpolation::Step => Self::Step(StepSpline {
                times,
                values,
                components,
            }),
            Interpolation::Linear => {
                // 旋转通道走四元数 slerp，其余走普通线性
                if path == AnimationPath::Rotation {
                    Self::QuaternionSlerp(QuaternionSpline { times, values })
                } else {
                    Self::Linear(LinearSpline {
                        times,
                        values,
                        components,
                    })
                }
            }
            Interpolation::CubicSpline => {
                // CubicSpline 数据布局：每个关键帧 [inTangent, value, outTangent]
                let num_keys = times.len();
                let mut cubic_values = Vec::with_capacity(num_keys * components);
                let mut in_tangents = Vec::with_capacity((num_keys - 1) * components);
                let mut out_tangents = Vec::with_capacity((num_keys - 1) * components);

                // 每关键帧占 [inTangent, value, outTangent] 三段
                for i in 0..num_keys {
                    let base = i * 3 * components;
                    // 入切线
                    if i > 0 && base + components <= values.len() {
                        in_tangents
                            .extend_from_slice(&values[base..base + components]);
                    }
                    // 值
                    let val_base = base + components;
                    if val_base + components <= values.len() {
                        cubic_values
                            .extend_from_slice(&values[val_base..val_base + components]);
                    }
                    // 出切线
                    let out_base = base + 2 * components;
                    if i < num_keys - 1 && out_base + components <= values.len() {
                        out_tangents
                            .extend_from_slice(&values[out_base..out_base + components]);
                    }
                }

                Self::CubicSpline(CubicSpline {
                    times,
                    values: cubic_values,
                    in_tangents,
                    out_tangents,
                    components,
                })
            }
        }
    }

    /// 在时间 t 处求值样条。
    /// 将插值结果作为扁平向量返回。
    pub fn evaluate(&self, time: f64) -> Vec<f64> {
        // 常量样条直接返回其固定值，其余分派到对应实现
        match self {
            Self::Constant(s) => s.value.clone(),
            Self::Step(s) => s.evaluate(time),
            Self::Linear(s) => s.evaluate(time),
            Self::QuaternionSlerp(s) => s.evaluate(time),
            Self::CubicSpline(s) => s.evaluate(time),
        }
    }

    /// 将时间钳制到样条的范围。
    pub fn clamp_time(&self, time: f64) -> f64 {
        // 将 time 钳制到首末关键帧之间
        let times = self.times();
        if times.is_empty() {
            return 0.0;
        }
        time.clamp(times[0], *times.last().unwrap())
    }

    /// 将时间环绕到样条的范围（用于循环）。
    pub fn wrap_time(&self, time: f64) -> f64 {
        // 以 duration 为周期对 time 取模环绕
        let times = self.times();
        if times.len() < 2 {
            return 0.0;
        }
        // 首末帧构成环绕区间
        let start = times[0];
        let end = *times.last().unwrap();
        let duration = end - start;
        // 零时长退化为直接返回起点
        if duration <= 0.0 {
            return start;
        }
        start + (time - start).rem_euclid(duration)
    }

    /// 返回样条的关键帧时间序列（常量样条为空）。
    fn times(&self) -> &[f64] {
        match self {
            Self::Constant(_) => &[],
            Self::Step(s) => &s.times,
            Self::Linear(s) => &s.times,
            Self::QuaternionSlerp(s) => &s.times,
            Self::CubicSpline(s) => &s.times,
        }
    }
}

impl StepSpline {
    /// 阶跃插值：返回不超过 time 的最近关键帧的值。
    fn evaluate(&self, time: f64) -> Vec<f64> {
        let idx = self.find_keyframe(time);
        // 定位到该关键帧在展平 values 中的基址
        let base = idx * self.components;
        if base + self.components <= self.values.len() {
            self.values[base..base + self.components].to_vec()
        } else {
            // 越界回退全零分量
            vec![0.0; self.components]
        }
    }

    /// 返回 time 之前（含）最后一个关键帧的索引。
    fn find_keyframe(&self, time: f64) -> usize {
        // 找到 time <= 给定时间的最后一个关键帧
        let mut idx = 0;
        // times 升序：记录最后一个 <= time 的关键帧索引
        for (i, &t) in self.times.iter().enumerate() {
            if t <= time {
                idx = i;
            } else {
                break;
            }
        }
        idx
    }
}

impl LinearSpline {
    /// 线性插值：在相邻关键帧间按归一化参数 t 逐分量混合。
    fn evaluate(&self, time: f64) -> Vec<f64> {
        let (i, t) = self.find_interval(time);
        let base0 = i * self.components;
        let base1 = (i + 1) * self.components;

        // 末帧越界：直接返回起点关键帧
        if base1 + self.components > self.values.len() {
            return self.values[base0..base0 + self.components].to_vec();
        }

        let mut result = Vec::with_capacity(self.components);
        // 逐分量按局部参数 t 线性混合
        for c in 0..self.components {
            let v0 = self.values[base0 + c];
            let v1 = self.values[base1 + c];
            result.push(v0 + (v1 - v0) * t);
        }
        result
    }

    /// 返回 time 所在区间索引与该区间内的归一化参数 t。
    fn find_interval(&self, time: f64) -> (usize, f64) {
        // 早于首帧时钳到第一个区间
        if time <= self.times[0] {
            return (0, 0.0);
        }
        let last = self.times.len() - 1;
        if time >= self.times[last] {
            return (last.saturating_sub(1), 1.0);
        }

        // 扫描定位 time 所在的两个关键帧之间
        for i in 0..last {
            if time >= self.times[i] && time < self.times[i + 1] {
                let dt = self.times[i + 1] - self.times[i];
                // 区间内归一化进度 t（零时长区间回退 0）
                let t = if dt > 0.0 {
                    (time - self.times[i]) / dt
                } else {
                    0.0
                };
                return (i, t);
            }
        }
        (last.saturating_sub(1), 1.0)
    }
}

impl QuaternionSpline {
    /// 四元数 slerp 插值：在相邻旋转关键帧间球面线性插值。
    fn evaluate(&self, time: f64) -> Vec<f64> {
        let (i, t) = self.find_interval(time);
        let base0 = i * 4;
        let base1 = (i + 1) * 4;

        if base1 + 4 > self.values.len() {
            // 末帧越界：返回起点四元数
            return self.values[base0..base0 + 4].to_vec();
        }

        // 将展平值重建为起止两个四元数
        let q0 = DQuat::from_xyzw(
            self.values[base0],
            self.values[base0 + 1],
            self.values[base0 + 2],
            self.values[base0 + 3],
        );
        let q1 = DQuat::from_xyzw(
            self.values[base1],
            self.values[base1 + 1],
            self.values[base1 + 2],
            self.values[base1 + 3],
        );

        // 球面插值后展平为 [x, y, z, w]
        let result = q0.slerp(q1, t);
        vec![result.x, result.y, result.z, result.w]
    }

    /// 返回 time 所在区间索引与该区间内的归一化参数 t。
    fn find_interval(&self, time: f64) -> (usize, f64) {
        // 早于首帧时钳到第一个区间
        if time <= self.times[0] {
            return (0, 0.0);
        }
        let last = self.times.len() - 1;
        if time >= self.times[last] {
            return (last.saturating_sub(1), 1.0);
        }

        // 扫描定位 time 所在的两个关键帧之间
        for i in 0..last {
            if time >= self.times[i] && time < self.times[i + 1] {
                let dt = self.times[i + 1] - self.times[i];
                // 区间内归一化进度 t（零时长区间回退 0）
                let t = if dt > 0.0 {
                    (time - self.times[i]) / dt
                } else {
                    0.0
                };
                return (i, t);
            }
        }
        (last.saturating_sub(1), 1.0)
    }
}

impl CubicSpline {
    /// Hermite 三次样条插值：结合切线在关键帧间平滑求值。
    fn evaluate(&self, time: f64) -> Vec<f64> {
        let (i, t) = self.find_interval(time);
        let base0 = i * self.components;
        let base1 = (i + 1) * self.components;

        if base1 + self.components > self.values.len() {
            if base0 + self.components <= self.values.len() {
                return self.values[base0..base0 + self.components].to_vec();
            }
            return vec![0.0; self.components];
        }

        // Hermite 插值：
        // p(t) = (2t³ - 3t² + 1)p0 + (t³ - 2t² + t)m0 + (-2t³ + 3t²)p1 + (t³ - t²)m1
        let t2 = t * t;
        let t3 = t2 * t;

        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;

        // 用于切线缩放的关键帧间时间差
        let dt = if i + 1 < self.times.len() {
            self.times[i + 1] - self.times[i]
        } else {
            1.0
        };

        let mut result = Vec::with_capacity(self.components);
        for c in 0..self.components {
            let p0 = self.values[base0 + c];
            let p1 = self.values[base1 + c];

            // out_tangent[i] 与 in_tangent[i]（因第一个 in-tangent 未使用而错开一位）
            let out_base = i * self.components;
            let in_base = if i > 0 { (i - 1) * self.components } else { 0 };

            let m0 = if out_base + c < self.out_tangents.len() {
                self.out_tangents[out_base + c] * dt
            } else {
                0.0
            };
            let m1 = if in_base + c < self.in_tangents.len() {
                self.in_tangents[in_base + c] * dt
            } else {
                0.0
            };

            result.push(h00 * p0 + h10 * m0 + h01 * p1 + h11 * m1);
        }
        result
    }

    /// 返回 time 所在区间索引与该区间内的归一化参数 t。
    fn find_interval(&self, time: f64) -> (usize, f64) {
        // 早于首帧时钳到第一个区间
        if time <= self.times[0] {
            return (0, 0.0);
        }
        let last = self.times.len() - 1;
        if time >= self.times[last] {
            return (last.saturating_sub(1), 1.0);
        }

        // 扫描定位 time 所在的两个关键帧之间
        for i in 0..last {
            if time >= self.times[i] && time < self.times[i + 1] {
                let dt = self.times[i + 1] - self.times[i];
                // 区间内归一化进度 t（零时长区间回退 0）
                let t = if dt > 0.0 {
                    (time - self.times[i]) / dt
                } else {
                    0.0
                };
                return (i, t);
            }
        }
        (last.saturating_sub(1), 1.0)
    }
}

/// 用于骨骼动画的运行时 skin。
///
/// 封装关节骨架、逆绑定矩阵与逐帧计算出的关节矩阵。
#[derive(Debug, Clone)]
pub struct RuntimeSkin {
    /// joint node 索引。
    pub joints: Vec<usize>,
    /// 逆变换绑定矩阵（每关节一个，列主序 4x4）。
    pub inverse_bind_matrices: Vec<DMat4>,
    /// 已计算的关节矩阵（每帧更新）。
    pub joint_matrices: Vec<DMat4>,
}

impl RuntimeSkin {
    /// 由 joint 索引与逆变换绑定矩阵创建运行时 skin。
    pub fn new(joints: Vec<usize>, inverse_bind_matrices: Vec<DMat4>) -> Self {
        // 关节数决定初始矩阵数量
        let count = joints.len();
        Self {
            joints,
            inverse_bind_matrices,
            joint_matrices: vec![DMat4::IDENTITY; count],
        }
    }

    /// 由 node 世界变换更新关节矩阵。
    ///
    /// 映射到 CesiumJS `ModelSkin.updateJointMatrices`
    /// 公式：jointMatrix[i] = nodeWorldTransform[joint[i]] * inverseBindMatrix[i]
    pub fn update_joint_matrices(&mut self, node_world_transforms: &[DMat4]) {
        // 仅当关节索引与逆绑定矩阵都有效时才更新
        for (i, &joint_idx) in self.joints.iter().enumerate() {
            if joint_idx < node_world_transforms.len()
                && i < self.inverse_bind_matrices.len()
            {
                self.joint_matrices[i] = node_world_transforms[joint_idx]
                    * self.inverse_bind_matrices[i];
            }
        }
    }

    /// 给定一个顶点的关节权重，计算其蒙皮矩阵。
    ///
    /// 映射到 CesiumJS GPU 蒙皮：
    /// `skinningMatrix = sum(weight[i] * jointMatrix[joint[i]])`
    pub fn compute_skinning_matrix(
        &self,
        joints: [u16; 4],
        weights: [f32; 4],
    ) -> DMat4 {
        // 累加各影响关节矩阵的加权和
        let mut result = DMat4::ZERO;

        for i in 0..4 {
            let weight = weights[i] as f64;
            // 跳过零权重关节
            if weight > 0.0 {
                let joint_idx = joints[i] as usize;
                if joint_idx < self.joint_matrices.len() {
                    // 忽略越界的关节索引
                    result += self.joint_matrices[joint_idx] * weight;
                }
            }
        }

        result
    }
}

/// morph target 混合。
///
/// 映射到 CesiumJS 在 ModelRuntimePrimitive 中的 morph target 处理。
#[derive(Debug, Clone, Default)]
pub struct MorphTargetBlender {
    /// 当前的 morph 权重。
    pub weights: Vec<f64>,
}

impl MorphTargetBlender {
    /// 创建一个具有给定 target 数量的新 morph target 混合器。
    pub fn new(target_count: usize) -> Self {
        // 初始时所有 target 权重为 0
        Self {
            weights: vec![0.0; target_count],
        }
    }

    /// 设置一个 morph target 权重。
    pub fn set_weight(&mut self, index: usize, weight: f64) {
        // 越界忽略，权重钳到 [0,1]
        if index < self.weights.len() {
            self.weights[index] = weight.clamp(0.0, 1.0);
        }
    }

    /// 在各 morph target 之间混合一个顶点属性。
    ///
    /// result = base + sum(weight[i] * target_displacement[i])
    pub fn blend_attribute(
        &self,
        base: DVec3,
        target_displacements: &[DVec3],
    ) -> DVec3 {
        // 基值上逐 target 叠加位移的加权和
        let mut result = base;
        // 跳过零权重与越界 target
        for (i, &weight) in self.weights.iter().enumerate() {
            if weight > 0.0 && i < target_displacements.len() {
                result += target_displacements[i] * weight;
            }
        }
        result
    }
}

/// 一个针对特定 node 属性的 animation channel。
///
/// 映射到 CesiumJS `ModelAnimationChannel`
#[derive(Debug, Clone)]
pub struct RuntimeChannel {
    /// 目标 node 索引。
    pub target_node: usize,
    /// 目标属性路径。
    pub path: AnimationPath,
    /// 插值样条。
    pub spline: AnimationSpline,
}

impl RuntimeChannel {
    /// 在给定时间求值该 channel。
    /// 将动画值作为扁平向量返回。
    pub fn evaluate(&self, time: f64, clamp: bool) -> Vec<f64> {
        // 依钳制/环绕选择生效时间后交给样条求值
        let t = if clamp {
            self.spline.clamp_time(time)
        } else {
            self.spline.wrap_time(time)
        };
        self.spline.evaluate(t)
    }

    /// 作为平移向量求值。
    pub fn evaluate_translation(&self, time: f64, clamp: bool) -> DVec3 {
        // 取前三分量作平移，不足则零向量
        let v = self.evaluate(time, clamp);
        if v.len() >= 3 {
            DVec3::new(v[0], v[1], v[2])
        } else {
            // 分量不足则回退零平移
            DVec3::ZERO
        }
    }

    /// 作为旋转四元数求值。
    pub fn evaluate_rotation(&self, time: f64, clamp: bool) -> DQuat {
        // 取四分量作四元数，不足则单位四元数
        let v = self.evaluate(time, clamp);
        if v.len() >= 4 {
            DQuat::from_xyzw(v[0], v[1], v[2], v[3])
        } else {
            // 分量不足则回退恒等旋转
            DQuat::IDENTITY
        }
    }

    /// 作为缩放向量求值。
    pub fn evaluate_scale(&self, time: f64, clamp: bool) -> DVec3 {
        // 取前三分量作缩放，不足则单位向量
        let v = self.evaluate(time, clamp);
        if v.len() >= 3 {
            DVec3::new(v[0], v[1], v[2])
        } else {
            // 分量不足则回退单位缩放
            DVec3::ONE
        }
    }
}

/// 从关键帧时间计算动画时长。
pub fn compute_duration(times: &[f64]) -> f64 {
    // 时长 = 末帧时间 - 首帧时间；无关键帧时为 0
    if times.is_empty() {
        return 0.0;
    }
    times.last().unwrap() - times[0]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 播放/暂停/停止状态机转换。
    #[test]
    fn test_runtime_animation_play_stop() {
        let anim = Animation::default();
        let mut rt = RuntimeAnimation::from_gltf(&anim, 2.0);
        assert_eq!(rt.state, AnimationState::Stopped);

        rt.play();
        assert_eq!(rt.state, AnimationState::Playing);

        rt.pause();
        assert_eq!(rt.state, AnimationState::Paused);

        rt.play();
        rt.stop();
        assert_eq!(rt.state, AnimationState::Stopped);
        assert_eq!(rt.local_time, 0.0);
    }

    /// 推进时间与末帧自动停止。
    #[test]
    fn test_runtime_animation_advance() {
        let anim = Animation::default();
        let mut rt = RuntimeAnimation::from_gltf(&anim, 2.0);
        rt.play();

        assert!(rt.advance(0.5));
        assert!((rt.local_time - 0.5).abs() < 1e-10);

        assert!(rt.advance(1.0));
        assert!((rt.local_time - 1.5).abs() < 1e-10);

        // 应在末尾停止（无循环）
        assert!(!rt.advance(1.0));
        assert_eq!(rt.state, AnimationState::Stopped);
    }

    /// Repeat 循环将时间取模回绕。
    #[test]
    fn test_runtime_animation_loop() {
        let anim = Animation::default();
        let mut rt = RuntimeAnimation::from_gltf(&anim, 2.0);
        rt.loop_mode = AnimationLoop::Repeat;
        rt.play();

        rt.advance(1.5);
        assert!((rt.local_time - 1.5).abs() < 1e-10);

        rt.advance(1.0);
        // 2.5 % 2.0 = 0.5
        assert!((rt.local_time - 0.5).abs() < 1e-10);
    }

    /// 反向播放时本地时间递减。
    #[test]
    fn test_runtime_animation_reverse() {
        let anim = Animation::default();
        let mut rt = RuntimeAnimation::from_gltf(&anim, 2.0);
        rt.reverse = true;
        rt.local_time = 2.0;
        rt.play();

        rt.advance(0.5);
        assert!((rt.local_time - 1.5).abs() < 1e-10);
    }

    /// 速度乘数放大每帧增量。
    #[test]
    fn test_runtime_animation_multiplier() {
        let anim = Animation::default();
        let mut rt = RuntimeAnimation::from_gltf(&anim, 4.0);
        rt.multiplier = 2.0;
        rt.play();

        rt.advance(1.0);
        assert!((rt.local_time - 2.0).abs() < 1e-10);
    }

    /// 单关键帧退化为常量样条。
    #[test]
    fn test_constant_spline() {
        let spline = AnimationSpline::from_keyframes(
            vec![0.0],
            vec![1.0, 2.0, 3.0],
            Interpolation::Linear,
            AnimationPath::Translation,
            3,
        );

        let v = spline.evaluate(0.5);
        assert_eq!(v, vec![1.0, 2.0, 3.0]);
    }

    /// 两关键帧线性插值中点。
    #[test]
    fn test_linear_spline() {
        let spline = AnimationSpline::from_keyframes(
            vec![0.0, 1.0],
            vec![0.0, 0.0, 0.0, 10.0, 20.0, 30.0],
            Interpolation::Linear,
            AnimationPath::Translation,
            3,
        );

        let v = spline.evaluate(0.5);
        assert!((v[0] - 5.0).abs() < 1e-10);
        assert!((v[1] - 10.0).abs() < 1e-10);
        assert!((v[2] - 15.0).abs() < 1e-10);
    }

    /// 超出样条范围时时间被钳制。
    #[test]
    fn test_linear_spline_clamp() {
        let spline = AnimationSpline::from_keyframes(
            vec![0.0, 1.0],
            vec![0.0, 10.0],
            Interpolation::Linear,
            AnimationPath::Translation,
            1,
        );

        let v = spline.evaluate(2.0);
        assert!((v[0] - 10.0).abs() < 1e-10);

        let v = spline.evaluate(-1.0);
        assert!((v[0] - 0.0).abs() < 1e-10);
    }

    /// 阶跃样条取不超过时间的最近值。
    #[test]
    fn test_step_spline() {
        let spline = AnimationSpline::from_keyframes(
            vec![0.0, 1.0, 2.0],
            vec![0.0, 5.0, 10.0],
            Interpolation::Step,
            AnimationPath::Translation,
            1,
        );

        let v = spline.evaluate(0.5);
        assert!((v[0] - 0.0).abs() < 1e-10);

        let v = spline.evaluate(1.5);
        assert!((v[0] - 5.0).abs() < 1e-10);

        let v = spline.evaluate(2.0);
        assert!((v[0] - 10.0).abs() < 1e-10);
    }

    /// 四元数样条中点等于 slerp(0.5)。
    #[test]
    fn test_quaternion_spline() {
        // 从单四元数到绕 Z 轴旋转 90°
        let q0 = DQuat::IDENTITY;
        let q1 = DQuat::from_rotation_z(std::f64::consts::FRAC_PI_2);

        let spline = AnimationSpline::from_keyframes(
            vec![0.0, 1.0],
            vec![q0.x, q0.y, q0.z, q0.w, q1.x, q1.y, q1.z, q1.w],
            Interpolation::Linear,
            AnimationPath::Rotation,
            4,
        );

        let v = spline.evaluate(0.5);
        let result = DQuat::from_xyzw(v[0], v[1], v[2], v[3]);
        let expected = q0.slerp(q1, 0.5);

        assert!((result.x - expected.x).abs() < 1e-10);
        assert!((result.y - expected.y).abs() < 1e-10);
        assert!((result.z - expected.z).abs() < 1e-10);
        assert!((result.w - expected.w).abs() < 1e-10);
    }

    /// Hermite 三次样条端点回归到 value。
    #[test]
    fn test_cubic_spline() {
        // CubicSpline 布局：[inTangent0, value0, outTangent0, inTangent1, value1, outTangent1]
        let spline = AnimationSpline::from_keyframes(
            vec![0.0, 1.0],
            vec![
                0.0, 0.0, 0.0, // in-tangent[0]（未使用）
                0.0, 0.0, 0.0, // value[0]
                1.0, 1.0, 1.0, // out-tangent[0]
                1.0, 1.0, 1.0, // in-tangent[1]
                10.0, 10.0, 10.0, // value[1]
                0.0, 0.0, 0.0, // out-tangent[1]（未使用）
            ],
            Interpolation::CubicSpline,
            AnimationPath::Translation,
            3,
        );

        // 在 t=0 时，应为 value[0]
        let v = spline.evaluate(0.0);
        assert!(v[0].abs() < 1e-10);

        // 在 t=1 时，应为 value[1]
        let v = spline.evaluate(1.0);
        assert!((v[0] - 10.0).abs() < 1e-10);
    }

    /// 关节矩阵 = nodeWorld * inverseBind。
    #[test]
    fn test_runtime_skin() {
        let joints = vec![0, 1];
        let ibm = vec![DMat4::IDENTITY, DMat4::IDENTITY];
        let mut skin = RuntimeSkin::new(joints, ibm);

        let transforms = vec![
            DMat4::from_translation(DVec3::new(1.0, 0.0, 0.0)),
            DMat4::from_translation(DVec3::new(0.0, 2.0, 0.0)),
        ];

        skin.update_joint_matrices(&transforms);

        // 关节 0：translate(1,0,0) * identity = translate(1,0,0)
        let t0 = skin.joint_matrices[0].w_axis.truncate();
        assert!((t0.x - 1.0).abs() < 1e-10);

        // 关节 1：translate(0,2,0) * identity = translate(0,2,0)
        let t1 = skin.joint_matrices[1].w_axis.truncate();
        assert!((t1.y - 2.0).abs() < 1e-10);
    }

    /// 50/50 双关节蒙皮矩阵混合。
    #[test]
    fn test_skinning_matrix() {
        let joints = vec![0, 1];
        let ibm = vec![DMat4::IDENTITY, DMat4::IDENTITY];
        let mut skin = RuntimeSkin::new(joints, ibm);

        let transforms = vec![
            DMat4::from_translation(DVec3::new(2.0, 0.0, 0.0)),
            DMat4::from_translation(DVec3::new(0.0, 4.0, 0.0)),
        ];
        skin.update_joint_matrices(&transforms);

        // 关节 0 与关节 1 之间 50/50 混合
        let matrix = skin.compute_skinning_matrix([0, 1, 0, 0], [0.5, 0.5, 0.0, 0.0]);
        let t = matrix.w_axis.truncate();
        assert!((t.x - 1.0).abs() < 1e-10);
        assert!((t.y - 2.0).abs() < 1e-10);
    }

    /// morph 权重叠加基值得到混合结果。
    #[test]
    fn test_morph_target_blender() {
        let mut blender = MorphTargetBlender::new(2);
        blender.set_weight(0, 0.5);
        blender.set_weight(1, 1.0);

        let base = DVec3::new(0.0, 0.0, 0.0);
        let targets = vec![
            DVec3::new(2.0, 0.0, 0.0),
            DVec3::new(0.0, 3.0, 0.0),
        ];

        let result = blender.blend_attribute(base, &targets);
        assert!((result.x - 1.0).abs() < 1e-10); // 0 + 0.5 * 2
        assert!((result.y - 3.0).abs() < 1e-10); // 0 + 1.0 * 3
    }

    /// channel 平移求值取前三分量。
    #[test]
    fn test_runtime_channel_evaluate() {
        let channel = RuntimeChannel {
            target_node: 0,
            path: AnimationPath::Translation,
            spline: AnimationSpline::from_keyframes(
                vec![0.0, 1.0],
                vec![0.0, 0.0, 0.0, 5.0, 10.0, 15.0],
                Interpolation::Linear,
                AnimationPath::Translation,
                3,
            ),
        };

        let t = channel.evaluate_translation(0.5, true);
        assert!((t.x - 2.5).abs() < 1e-10);
        assert!((t.y - 5.0).abs() < 1e-10);
        assert!((t.z - 7.5).abs() < 1e-10);
    }

    /// wrap_time 将越界时间环绕回范围。
    #[test]
    fn test_spline_wrap_time() {
        let spline = AnimationSpline::from_keyframes(
            vec![0.0, 2.0],
            vec![0.0, 10.0],
            Interpolation::Linear,
            AnimationPath::Translation,
            1,
        );

        let wrapped = spline.wrap_time(3.0);
        assert!((wrapped - 1.0).abs() < 1e-10);
    }

    /// MirroredRepeat 后半段倒放。
    #[test]
    fn test_mirrored_repeat() {
        let anim = Animation::default();
        let mut rt = RuntimeAnimation::from_gltf(&anim, 2.0);
        rt.loop_mode = AnimationLoop::MirroredRepeat;
        rt.play();

        // 推进到 3.0 → cycle=4，t=3.0 > 2.0 → 4.0 - 3.0 = 1.0
        rt.advance(3.0);
        assert!((rt.local_time - 1.0).abs() < 1e-10);
    }

    /// 时长 = 末帧减首帧，空为 0。
    #[test]
    fn test_compute_duration() {
        assert!((compute_duration(&[0.0, 1.5, 3.0]) - 3.0).abs() < 1e-10);
        assert!((compute_duration(&[]) - 0.0).abs() < 1e-10);
    }
}
