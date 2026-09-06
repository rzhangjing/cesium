//! Ported from `packages/engine/Source/Scene/Model/ModelArticulationStage.js`.
//!
//! An in-memory representation of an articulation stage belonging to a
//! [`ModelArticulation`](super::model_articulation::ModelArticulation).

use cesium_core::articulation_stage_type::ArticulationStageType;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::math::CesiumMath;
use cesium_core::matrix3::Matrix3;
use cesium_core::matrix4::Matrix4;

/// An in-memory representation of a stage within a
/// [`ModelArticulation`](super::model_articulation::ModelArticulation).
///
/// Mirrors CesiumJS `ModelArticulationStage` (262 lines).
pub struct ModelArticulationStage {
    /// The name of this articulation stage.
    pub name: String,
    /// The type of transformation this stage applies.
    pub stage_type: ArticulationStageType,
    /// The minimum allowed value.
    pub minimum_value: f64,
    /// The maximum allowed value.
    pub maximum_value: f64,
    current_value: f64,
    /// Whether the owning articulation should be marked dirty on value change.
    pub dirty_on_change: bool,
}

impl ModelArticulationStage {
    /// Creates a new `ModelArticulationStage`.
    pub fn new(
        name: &str,
        stage_type: ArticulationStageType,
        minimum_value: f64,
        maximum_value: f64,
        initial_value: f64,
    ) -> Self {
        Self {
            name: name.to_string(),
            stage_type,
            minimum_value,
            maximum_value,
            current_value: CesiumMath::clamp(initial_value, minimum_value, maximum_value),
            dirty_on_change: false,
        }
    }

    /// Returns the current value of this stage.
    pub fn current_value(&self) -> f64 {
        self.current_value
    }

    /// Sets the current value, clamping to `[minimum_value, maximum_value]`.
    ///
    /// Returns `true` if the value actually changed (exceeds `EPSILON16`).
    pub fn set_current_value(&mut self, value: f64) -> bool {
        let clamped = CesiumMath::clamp(value, self.minimum_value, self.maximum_value);
        if !CesiumMath::equals_epsilon(self.current_value, clamped, Some(CesiumMath::EPSILON16), None) {
            self.current_value = clamped;
            return true;
        }
        false
    }

    /// Applies this stage's transformation to `result` in-place.
    ///
    /// Mirrors CesiumJS `ModelArticulationStage.prototype.applyStageToMatrix`.
    pub fn apply_stage_to_matrix(&self, result: &mut Matrix4) {
        let value = self.current_value;
        let mut out = Matrix4::IDENTITY;
        match self.stage_type {
            ArticulationStageType::XRotate => {
                let r = Matrix3::from_rotation_x_new(CesiumMath::to_radians(value));
                Matrix4::multiply_by_matrix3(result, &r, &mut out);
            }
            ArticulationStageType::YRotate => {
                let r = Matrix3::from_rotation_y_new(CesiumMath::to_radians(value));
                Matrix4::multiply_by_matrix3(result, &r, &mut out);
            }
            ArticulationStageType::ZRotate => {
                let r = Matrix3::from_rotation_z_new(CesiumMath::to_radians(value));
                Matrix4::multiply_by_matrix3(result, &r, &mut out);
            }
            ArticulationStageType::XTranslate => {
                let t = Cartesian3::new(value, 0.0, 0.0);
                Matrix4::multiply_by_translation(result, &t, &mut out);
            }
            ArticulationStageType::YTranslate => {
                let t = Cartesian3::new(0.0, value, 0.0);
                Matrix4::multiply_by_translation(result, &t, &mut out);
            }
            ArticulationStageType::ZTranslate => {
                let t = Cartesian3::new(0.0, 0.0, value);
                Matrix4::multiply_by_translation(result, &t, &mut out);
            }
            ArticulationStageType::XScale => {
                let s = Cartesian3::new(value, 1.0, 1.0);
                Matrix4::multiply_by_scale(result, &s, &mut out);
            }
            ArticulationStageType::YScale => {
                let s = Cartesian3::new(1.0, value, 1.0);
                Matrix4::multiply_by_scale(result, &s, &mut out);
            }
            ArticulationStageType::ZScale => {
                let s = Cartesian3::new(1.0, 1.0, value);
                Matrix4::multiply_by_scale(result, &s, &mut out);
            }
            ArticulationStageType::UniformScale => {
                Matrix4::multiply_by_uniform_scale(result, value, &mut out);
            }
        }
        *result = out;
    }

    /// Applies this stage's transformation and returns a new matrix.
    pub fn apply_stage_to_matrix_new(&self, matrix: &Matrix4) -> Matrix4 {
        let mut result = *matrix;
        self.apply_stage_to_matrix(&mut result);
        result
    }
}

impl Default for ModelArticulationStage {
    fn default() -> Self {
        Self {
            name: String::new(),
            stage_type: ArticulationStageType::XRotate,
            minimum_value: 0.0,
            maximum_value: 0.0,
            current_value: 0.0,
            dirty_on_change: false,
        }
    }
}
