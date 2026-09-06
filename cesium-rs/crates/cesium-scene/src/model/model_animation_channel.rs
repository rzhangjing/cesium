//! Ported from `packages/engine/Source/Scene/Model/ModelAnimationChannel.js`.
//!
//! A runtime animation channel for a ModelAnimation.

/// The animated property type of a glTF animation channel target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnimatedPropertyType {
    /// Node translation.
    Translation,
    /// Node rotation.
    Rotation,
    /// Node scale.
    Scale,
    /// Morph target weights.
    Weights,
}

impl AnimatedPropertyType {
    /// Returns the glTF string name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Translation => "translation",
            Self::Rotation => "rotation",
            Self::Scale => "scale",
            Self::Weights => "weights",
        }
    }

    /// Parses from a glTF string name.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "translation" => Some(Self::Translation),
            "rotation" => Some(Self::Rotation),
            "scale" => Some(Self::Scale),
            "weights" => Some(Self::Weights),
            _ => None,
        }
    }
}

/// The interpolation mode for animation sampler keyframes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InterpolationType {
    /// Step (discrete) interpolation.
    Step,
    /// Linear interpolation.
    Linear,
    /// Cubic spline interpolation.
    CubicSpline,
}

impl InterpolationType {
    /// Returns the glTF string name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Step => "STEP",
            Self::Linear => "LINEAR",
            Self::CubicSpline => "CUBICSPLINE",
        }
    }

    /// Parses from a glTF string name.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "STEP" => Some(Self::Step),
            "LINEAR" => Some(Self::Linear),
            "CUBICSPLINE" => Some(Self::CubicSpline),
            _ => None,
        }
    }
}

/// A runtime animation channel that interpolates between keyframe values
/// and applies the change to a target node property.
///
/// Mirrors CesiumJS `ModelAnimationChannel` (295 lines).
pub struct ModelAnimationChannel {
    /// The target property path being animated.
    pub target_path: AnimatedPropertyType,
    /// The index of the sampler in the animation.
    pub sampler_index: u32,
    /// The interpolation mode.
    pub interpolation: InterpolationType,
    /// The keyframe input times.
    pub times: Vec<f64>,
    /// The keyframe output values (flat array; layout depends on property type).
    pub points: Vec<f64>,
    /// The index of the target node.
    pub target_node_index: usize,
}

impl ModelAnimationChannel {
    /// Creates a new `ModelAnimationChannel`.
    pub fn new() -> Self {
        Self {
            target_path: AnimatedPropertyType::Translation,
            sampler_index: 0,
            interpolation: InterpolationType::Linear,
            times: Vec::new(),
            points: Vec::new(),
            target_node_index: 0,
        }
    }

    /// Returns the number of keyframes.
    pub fn keyframe_count(&self) -> usize {
        self.times.len()
    }

    /// Returns the duration of this channel's keyframes.
    pub fn duration(&self) -> f64 {
        if let (Some(&first), Some(&last)) = (self.times.first(), self.times.last()) {
            if last > first {
                last - first
            } else {
                0.0
            }
        } else {
            0.0
        }
    }
}

impl Default for ModelAnimationChannel {
    fn default() -> Self {
        Self::new()
    }
}
