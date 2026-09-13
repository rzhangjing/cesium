//! Skeletal animation runtime system for glTF 2.0.
//!
//! Maps to CesiumJS:
//! - `Scene/Model/ModelAnimation.js`
//! - `Scene/Model/ModelAnimationChannel.js`
//! - `Scene/Model/ModelAnimationCollection.js`
//! - `Scene/Model/ModelSkin.js`
//! - `Scene/Model/ModelRuntimeNode.js`
//!
//! Provides animation evaluation (spline interpolation), skinning (joint matrix
//! computation), and morph target blending.

use crate::gltf_model::{Animation, AnimationPath, Interpolation};
use glam::{DMat4, DQuat, DVec3};

/// Animation playback state.
///
/// Maps to CesiumJS `ModelAnimationState`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnimationState {
    /// Animation is stopped.
    #[default]
    Stopped,
    /// Animation is playing.
    Playing,
    /// Animation is paused.
    Paused,
}

/// Animation loop mode.
///
/// Maps to CesiumJS `ModelAnimationLoop`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnimationLoop {
    /// Play once and stop.
    #[default]
    None,
    /// Loop continuously.
    Repeat,
    /// Ping-pong (play forward then backward).
    MirroredRepeat,
}

/// A runtime animation instance with playback control.
///
/// Maps to CesiumJS `Scene/Model/ModelAnimation.js`
#[derive(Debug, Clone)]
pub struct RuntimeAnimation {
    /// Animation name.
    pub name: Option<String>,
    /// Current playback state.
    pub state: AnimationState,
    /// Loop mode.
    pub loop_mode: AnimationLoop,
    /// Playback speed multiplier.
    pub multiplier: f64,
    /// Whether to reverse playback.
    pub reverse: bool,
    /// Current local time in seconds.
    pub local_time: f64,
    /// Duration of the animation in seconds.
    pub duration: f64,
    /// Delay before starting in seconds.
    pub delay: f64,
    /// Whether to remove when stopped.
    pub remove_on_stop: bool,
    /// Whether to clamp animations to their time range.
    pub clamp_animations: bool,
}

impl RuntimeAnimation {
    /// Creates a new runtime animation from a glTF animation.
    pub fn from_gltf(animation: &Animation, duration: f64) -> Self {
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

    /// Starts playing the animation.
    pub fn play(&mut self) {
        self.state = AnimationState::Playing;
    }

    /// Pauses the animation.
    pub fn pause(&mut self) {
        if self.state == AnimationState::Playing {
            self.state = AnimationState::Paused;
        }
    }

    /// Stops the animation and resets time.
    pub fn stop(&mut self) {
        self.state = AnimationState::Stopped;
        self.local_time = 0.0;
    }

    /// Advances the animation by delta_time seconds.
    /// Returns true if the animation is still active.
    pub fn advance(&mut self, delta_time: f64) -> bool {
        if self.state != AnimationState::Playing {
            return self.state != AnimationState::Stopped;
        }

        let effective_delta = if self.reverse {
            -delta_time * self.multiplier
        } else {
            delta_time * self.multiplier
        };

        self.local_time += effective_delta;

        // Handle looping
        if self.duration > 0.0 {
            match self.loop_mode {
                AnimationLoop::None => {
                    if self.local_time >= self.duration || self.local_time < 0.0 {
                        self.local_time = self.local_time.clamp(0.0, self.duration);
                        self.state = AnimationState::Stopped;
                        return false;
                    }
                }
                AnimationLoop::Repeat => {
                    self.local_time = self.local_time.rem_euclid(self.duration);
                }
                AnimationLoop::MirroredRepeat => {
                    let cycle = self.duration * 2.0;
                    let t = self.local_time.rem_euclid(cycle);
                    self.local_time = if t > self.duration {
                        cycle - t
                    } else {
                        t
                    };
                }
            }
        }

        true
    }

    /// Gets the effective time (clamped or wrapped based on settings).
    pub fn effective_time(&self) -> f64 {
        if self.clamp_animations {
            self.local_time.clamp(0.0, self.duration)
        } else if self.duration > 0.0 {
            self.local_time.rem_euclid(self.duration)
        } else {
            self.local_time
        }
    }
}

/// A keyframe spline for animation interpolation.
///
/// Maps to CesiumJS spline classes (LinearSpline, QuaternionSpline, HermiteSpline, SteppedSpline)
#[derive(Debug, Clone)]
pub enum AnimationSpline {
    /// Constant value (single keyframe).
    Constant(ConstantSpline),
    /// Step interpolation (hold value until next keyframe).
    Step(StepSpline),
    /// Linear interpolation.
    Linear(LinearSpline),
    /// Quaternion slerp interpolation.
    QuaternionSlerp(QuaternionSpline),
    /// Cubic Hermite spline interpolation.
    CubicSpline(CubicSpline),
}

/// Constant spline (single keyframe).
#[derive(Debug, Clone)]
pub struct ConstantSpline {
    /// The constant value.
    pub value: Vec<f64>,
}

/// Step spline (no interpolation, holds previous value).
#[derive(Debug, Clone)]
pub struct StepSpline {
    /// Keyframe times.
    pub times: Vec<f64>,
    /// Keyframe values (flattened).
    pub values: Vec<f64>,
    /// Components per keyframe.
    pub components: usize,
}

/// Linear interpolation spline.
#[derive(Debug, Clone)]
pub struct LinearSpline {
    /// Keyframe times.
    pub times: Vec<f64>,
    /// Keyframe values (flattened).
    pub values: Vec<f64>,
    /// Components per keyframe.
    pub components: usize,
}

/// Quaternion slerp spline.
#[derive(Debug, Clone)]
pub struct QuaternionSpline {
    /// Keyframe times.
    pub times: Vec<f64>,
    /// Quaternion values [x, y, z, w] per keyframe (flattened).
    pub values: Vec<f64>,
}

/// Cubic Hermite spline.
///
/// Maps to CesiumJS `HermiteSpline`
#[derive(Debug, Clone)]
pub struct CubicSpline {
    /// Keyframe times.
    pub times: Vec<f64>,
    /// Keyframe values (flattened).
    pub values: Vec<f64>,
    /// In-tangents (flattened, one fewer than values).
    pub in_tangents: Vec<f64>,
    /// Out-tangents (flattened, one fewer than values).
    pub out_tangents: Vec<f64>,
    /// Components per keyframe.
    pub components: usize,
}

impl AnimationSpline {
    /// Creates a spline from keyframe data.
    ///
    /// Maps to CesiumJS `ModelAnimationChannel.createSpline`
    pub fn from_keyframes(
        times: Vec<f64>,
        values: Vec<f64>,
        interpolation: Interpolation,
        path: AnimationPath,
        components: usize,
    ) -> Self {
        if times.len() <= 1 {
            return Self::Constant(ConstantSpline {
                value: if values.is_empty() {
                    vec![0.0; components]
                } else {
                    values[..components.min(values.len())].to_vec()
                },
            });
        }

        match interpolation {
            Interpolation::Step => Self::Step(StepSpline {
                times,
                values,
                components,
            }),
            Interpolation::Linear => {
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
                // CubicSpline data layout: [inTangent, value, outTangent] per keyframe
                let num_keys = times.len();
                let mut cubic_values = Vec::with_capacity(num_keys * components);
                let mut in_tangents = Vec::with_capacity((num_keys - 1) * components);
                let mut out_tangents = Vec::with_capacity((num_keys - 1) * components);

                for i in 0..num_keys {
                    let base = i * 3 * components;
                    // in-tangent
                    if i > 0 && base + components <= values.len() {
                        in_tangents
                            .extend_from_slice(&values[base..base + components]);
                    }
                    // value
                    let val_base = base + components;
                    if val_base + components <= values.len() {
                        cubic_values
                            .extend_from_slice(&values[val_base..val_base + components]);
                    }
                    // out-tangent
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

    /// Evaluates the spline at time t.
    /// Returns the interpolated value as a flat vector.
    pub fn evaluate(&self, time: f64) -> Vec<f64> {
        match self {
            Self::Constant(s) => s.value.clone(),
            Self::Step(s) => s.evaluate(time),
            Self::Linear(s) => s.evaluate(time),
            Self::QuaternionSlerp(s) => s.evaluate(time),
            Self::CubicSpline(s) => s.evaluate(time),
        }
    }

    /// Clamps time to the spline's range.
    pub fn clamp_time(&self, time: f64) -> f64 {
        let times = self.times();
        if times.is_empty() {
            return 0.0;
        }
        time.clamp(times[0], *times.last().unwrap())
    }

    /// Wraps time to the spline's range (for looping).
    pub fn wrap_time(&self, time: f64) -> f64 {
        let times = self.times();
        if times.len() < 2 {
            return 0.0;
        }
        let start = times[0];
        let end = *times.last().unwrap();
        let duration = end - start;
        if duration <= 0.0 {
            return start;
        }
        start + (time - start).rem_euclid(duration)
    }

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
    fn evaluate(&self, time: f64) -> Vec<f64> {
        let idx = self.find_keyframe(time);
        let base = idx * self.components;
        if base + self.components <= self.values.len() {
            self.values[base..base + self.components].to_vec()
        } else {
            vec![0.0; self.components]
        }
    }

    fn find_keyframe(&self, time: f64) -> usize {
        // Find the last keyframe with time <= given time
        let mut idx = 0;
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
    fn evaluate(&self, time: f64) -> Vec<f64> {
        let (i, t) = self.find_interval(time);
        let base0 = i * self.components;
        let base1 = (i + 1) * self.components;

        if base1 + self.components > self.values.len() {
            return self.values[base0..base0 + self.components].to_vec();
        }

        let mut result = Vec::with_capacity(self.components);
        for c in 0..self.components {
            let v0 = self.values[base0 + c];
            let v1 = self.values[base1 + c];
            result.push(v0 + (v1 - v0) * t);
        }
        result
    }

    fn find_interval(&self, time: f64) -> (usize, f64) {
        if time <= self.times[0] {
            return (0, 0.0);
        }
        let last = self.times.len() - 1;
        if time >= self.times[last] {
            return (last.saturating_sub(1), 1.0);
        }

        for i in 0..last {
            if time >= self.times[i] && time < self.times[i + 1] {
                let dt = self.times[i + 1] - self.times[i];
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
    fn evaluate(&self, time: f64) -> Vec<f64> {
        let (i, t) = self.find_interval(time);
        let base0 = i * 4;
        let base1 = (i + 1) * 4;

        if base1 + 4 > self.values.len() {
            return self.values[base0..base0 + 4].to_vec();
        }

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

        let result = q0.slerp(q1, t);
        vec![result.x, result.y, result.z, result.w]
    }

    fn find_interval(&self, time: f64) -> (usize, f64) {
        if time <= self.times[0] {
            return (0, 0.0);
        }
        let last = self.times.len() - 1;
        if time >= self.times[last] {
            return (last.saturating_sub(1), 1.0);
        }

        for i in 0..last {
            if time >= self.times[i] && time < self.times[i + 1] {
                let dt = self.times[i + 1] - self.times[i];
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

        // Hermite interpolation:
        // p(t) = (2t³ - 3t² + 1)p0 + (t³ - 2t² + t)m0 + (-2t³ + 3t²)p1 + (t³ - t²)m1
        let t2 = t * t;
        let t3 = t2 * t;

        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;

        // Delta time between keyframes for tangent scaling
        let dt = if i + 1 < self.times.len() {
            self.times[i + 1] - self.times[i]
        } else {
            1.0
        };

        let mut result = Vec::with_capacity(self.components);
        for c in 0..self.components {
            let p0 = self.values[base0 + c];
            let p1 = self.values[base1 + c];

            // out_tangent[i] and in_tangent[i] (offset by one since first in-tangent is unused)
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

    fn find_interval(&self, time: f64) -> (usize, f64) {
        if time <= self.times[0] {
            return (0, 0.0);
        }
        let last = self.times.len() - 1;
        if time >= self.times[last] {
            return (last.saturating_sub(1), 1.0);
        }

        for i in 0..last {
            if time >= self.times[i] && time < self.times[i + 1] {
                let dt = self.times[i + 1] - self.times[i];
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

/// Runtime skin for skeletal animation.
///
/// Maps to CesiumJS `Scene/Model/ModelSkin.js`
#[derive(Debug, Clone)]
pub struct RuntimeSkin {
    /// Joint node indices.
    pub joints: Vec<usize>,
    /// Inverse bind matrices (one per joint, column-major 4x4).
    pub inverse_bind_matrices: Vec<DMat4>,
    /// Computed joint matrices (updated each frame).
    pub joint_matrices: Vec<DMat4>,
}

impl RuntimeSkin {
    /// Creates a runtime skin from joint indices and inverse bind matrices.
    pub fn new(joints: Vec<usize>, inverse_bind_matrices: Vec<DMat4>) -> Self {
        let count = joints.len();
        Self {
            joints,
            inverse_bind_matrices,
            joint_matrices: vec![DMat4::IDENTITY; count],
        }
    }

    /// Updates joint matrices from node world transforms.
    ///
    /// Maps to CesiumJS `ModelSkin.updateJointMatrices`
    /// Formula: jointMatrix[i] = nodeWorldTransform[joint[i]] * inverseBindMatrix[i]
    pub fn update_joint_matrices(&mut self, node_world_transforms: &[DMat4]) {
        for (i, &joint_idx) in self.joints.iter().enumerate() {
            if joint_idx < node_world_transforms.len()
                && i < self.inverse_bind_matrices.len()
            {
                self.joint_matrices[i] = node_world_transforms[joint_idx]
                    * self.inverse_bind_matrices[i];
            }
        }
    }

    /// Computes the skinning matrix for a vertex given its joint weights.
    ///
    /// Maps to CesiumJS GPU skinning:
    /// `skinningMatrix = sum(weight[i] * jointMatrix[joint[i]])`
    pub fn compute_skinning_matrix(
        &self,
        joints: [u16; 4],
        weights: [f32; 4],
    ) -> DMat4 {
        let mut result = DMat4::ZERO;

        for i in 0..4 {
            let weight = weights[i] as f64;
            if weight > 0.0 {
                let joint_idx = joints[i] as usize;
                if joint_idx < self.joint_matrices.len() {
                    result += self.joint_matrices[joint_idx] * weight;
                }
            }
        }

        result
    }
}

/// Morph target blending.
///
/// Maps to CesiumJS morph target handling in ModelRuntimePrimitive.
#[derive(Debug, Clone, Default)]
pub struct MorphTargetBlender {
    /// Current morph weights.
    pub weights: Vec<f64>,
}

impl MorphTargetBlender {
    /// Creates a new morph target blender with the given number of targets.
    pub fn new(target_count: usize) -> Self {
        Self {
            weights: vec![0.0; target_count],
        }
    }

    /// Sets a morph target weight.
    pub fn set_weight(&mut self, index: usize, weight: f64) {
        if index < self.weights.len() {
            self.weights[index] = weight.clamp(0.0, 1.0);
        }
    }

    /// Blends a vertex attribute across morph targets.
    ///
    /// result = base + sum(weight[i] * target_displacement[i])
    pub fn blend_attribute(
        &self,
        base: DVec3,
        target_displacements: &[DVec3],
    ) -> DVec3 {
        let mut result = base;
        for (i, &weight) in self.weights.iter().enumerate() {
            if weight > 0.0 && i < target_displacements.len() {
                result += target_displacements[i] * weight;
            }
        }
        result
    }
}

/// An animation channel targeting a specific node property.
///
/// Maps to CesiumJS `ModelAnimationChannel`
#[derive(Debug, Clone)]
pub struct RuntimeChannel {
    /// Target node index.
    pub target_node: usize,
    /// Target property path.
    pub path: AnimationPath,
    /// The interpolation spline.
    pub spline: AnimationSpline,
}

impl RuntimeChannel {
    /// Evaluates the channel at the given time.
    /// Returns the animated value as a flat vector.
    pub fn evaluate(&self, time: f64, clamp: bool) -> Vec<f64> {
        let t = if clamp {
            self.spline.clamp_time(time)
        } else {
            self.spline.wrap_time(time)
        };
        self.spline.evaluate(t)
    }

    /// Evaluates as a translation vector.
    pub fn evaluate_translation(&self, time: f64, clamp: bool) -> DVec3 {
        let v = self.evaluate(time, clamp);
        if v.len() >= 3 {
            DVec3::new(v[0], v[1], v[2])
        } else {
            DVec3::ZERO
        }
    }

    /// Evaluates as a rotation quaternion.
    pub fn evaluate_rotation(&self, time: f64, clamp: bool) -> DQuat {
        let v = self.evaluate(time, clamp);
        if v.len() >= 4 {
            DQuat::from_xyzw(v[0], v[1], v[2], v[3])
        } else {
            DQuat::IDENTITY
        }
    }

    /// Evaluates as a scale vector.
    pub fn evaluate_scale(&self, time: f64, clamp: bool) -> DVec3 {
        let v = self.evaluate(time, clamp);
        if v.len() >= 3 {
            DVec3::new(v[0], v[1], v[2])
        } else {
            DVec3::ONE
        }
    }
}

/// Computes animation duration from keyframe times.
pub fn compute_duration(times: &[f64]) -> f64 {
    if times.is_empty() {
        return 0.0;
    }
    times.last().unwrap() - times[0]
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_runtime_animation_advance() {
        let anim = Animation::default();
        let mut rt = RuntimeAnimation::from_gltf(&anim, 2.0);
        rt.play();

        assert!(rt.advance(0.5));
        assert!((rt.local_time - 0.5).abs() < 1e-10);

        assert!(rt.advance(1.0));
        assert!((rt.local_time - 1.5).abs() < 1e-10);

        // Should stop at end (no loop)
        assert!(!rt.advance(1.0));
        assert_eq!(rt.state, AnimationState::Stopped);
    }

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

    #[test]
    fn test_runtime_animation_multiplier() {
        let anim = Animation::default();
        let mut rt = RuntimeAnimation::from_gltf(&anim, 4.0);
        rt.multiplier = 2.0;
        rt.play();

        rt.advance(1.0);
        assert!((rt.local_time - 2.0).abs() < 1e-10);
    }

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

    #[test]
    fn test_quaternion_spline() {
        // Identity to 90° rotation around Z
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

    #[test]
    fn test_cubic_spline() {
        // CubicSpline layout: [inTangent0, value0, outTangent0, inTangent1, value1, outTangent1]
        let spline = AnimationSpline::from_keyframes(
            vec![0.0, 1.0],
            vec![
                0.0, 0.0, 0.0, // in-tangent[0] (unused)
                0.0, 0.0, 0.0, // value[0]
                1.0, 1.0, 1.0, // out-tangent[0]
                1.0, 1.0, 1.0, // in-tangent[1]
                10.0, 10.0, 10.0, // value[1]
                0.0, 0.0, 0.0, // out-tangent[1] (unused)
            ],
            Interpolation::CubicSpline,
            AnimationPath::Translation,
            3,
        );

        // At t=0, should be value[0]
        let v = spline.evaluate(0.0);
        assert!(v[0].abs() < 1e-10);

        // At t=1, should be value[1]
        let v = spline.evaluate(1.0);
        assert!((v[0] - 10.0).abs() < 1e-10);
    }

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

        // Joint 0: translate(1,0,0) * identity = translate(1,0,0)
        let t0 = skin.joint_matrices[0].w_axis.truncate();
        assert!((t0.x - 1.0).abs() < 1e-10);

        // Joint 1: translate(0,2,0) * identity = translate(0,2,0)
        let t1 = skin.joint_matrices[1].w_axis.truncate();
        assert!((t1.y - 2.0).abs() < 1e-10);
    }

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

        // 50/50 blend between joint 0 and joint 1
        let matrix = skin.compute_skinning_matrix([0, 1, 0, 0], [0.5, 0.5, 0.0, 0.0]);
        let t = matrix.w_axis.truncate();
        assert!((t.x - 1.0).abs() < 1e-10);
        assert!((t.y - 2.0).abs() < 1e-10);
    }

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

    #[test]
    fn test_mirrored_repeat() {
        let anim = Animation::default();
        let mut rt = RuntimeAnimation::from_gltf(&anim, 2.0);
        rt.loop_mode = AnimationLoop::MirroredRepeat;
        rt.play();

        // Advance to 3.0 → cycle=4, t=3.0 > 2.0 → 4.0 - 3.0 = 1.0
        rt.advance(3.0);
        assert!((rt.local_time - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_compute_duration() {
        assert!((compute_duration(&[0.0, 1.5, 3.0]) - 3.0).abs() < 1e-10);
        assert!((compute_duration(&[]) - 0.0).abs() < 1e-10);
    }
}
