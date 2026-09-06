//! Ported from `packages/engine/Source/Scene/Model/ModelArticulation.js`.
//!
//! An in-memory representation of an articulation that affects nodes in the
//! model scene graph.

use cesium_core::matrix4::Matrix4;

use super::model_articulation_stage::ModelArticulationStage;

/// An in-memory representation of an articulation (hierarchical transform)
/// within a model, as defined by the `AGI_articulations` extension.
///
/// Mirrors CesiumJS `ModelArticulation` (215 lines).
pub struct ModelArticulation {
    /// The name of this articulation.
    pub name: String,
    /// The runtime stages that belong to this articulation.
    pub runtime_stages: Vec<ModelArticulationStage>,
    /// Lookup from stage name to index in `runtime_stages`.
    runtime_stages_by_name: std::collections::HashMap<String, usize>,
    /// The indices of runtime nodes affected by this articulation.
    pub runtime_node_indices: Vec<usize>,
    /// Whether any stage value has changed since the last [`apply`](Self::apply).
    dirty: bool,
}

impl ModelArticulation {
    /// Creates a new `ModelArticulation`.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            runtime_stages: Vec::new(),
            runtime_stages_by_name: std::collections::HashMap::new(),
            runtime_node_indices: Vec::new(),
            dirty: true,
        }
    }

    /// Creates a `ModelArticulation` from a list of stages.
    pub fn from_stages(name: &str, stages: Vec<ModelArticulationStage>) -> Self {
        let mut stages_by_name = std::collections::HashMap::new();
        for (i, stage) in stages.iter().enumerate() {
            stages_by_name.insert(stage.name.clone(), i);
        }
        Self {
            name: name.to_string(),
            runtime_stages: stages,
            runtime_stages_by_name: stages_by_name,
            runtime_node_indices: Vec::new(),
            dirty: true,
        }
    }

    /// Returns whether this articulation has un-applied changes.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Sets the value of a named articulation stage.
    ///
    /// Returns `true` if the stage exists and its value changed.
    pub fn set_articulation_stage_value(&mut self, stage_name: &str, value: f64) -> bool {
        if let Some(&idx) = self.runtime_stages_by_name.get(stage_name) {
            let changed = self.runtime_stages[idx].set_current_value(value);
            if changed {
                self.dirty = true;
            }
            changed
        } else {
            false
        }
    }

    /// Returns a reference to a stage by name.
    pub fn get_stage(&self, stage_name: &str) -> Option<&ModelArticulationStage> {
        self.runtime_stages_by_name
            .get(stage_name)
            .map(|&idx| &self.runtime_stages[idx])
    }

    /// Returns a mutable reference to a stage by name.
    pub fn get_stage_mut(&mut self, stage_name: &str) -> Option<&mut ModelArticulationStage> {
        if let Some(&idx) = self.runtime_stages_by_name.get(stage_name) {
            Some(&mut self.runtime_stages[idx])
        } else {
            None
        }
    }

    /// Applies the chain of articulation stages to produce a composite transform.
    ///
    /// Returns the composite `Matrix4` resulting from multiplying all stage
    /// transforms in order. Returns `None` if not dirty (no changes since last apply).
    ///
    /// Mirrors CesiumJS `ModelArticulation.prototype.apply`.
    pub fn apply(&mut self) -> Option<Matrix4> {
        if !self.dirty {
            return None;
        }
        self.dirty = false;

        let mut articulation_matrix = Matrix4::IDENTITY;

        for stage in &self.runtime_stages {
            stage.apply_stage_to_matrix(&mut articulation_matrix);
        }

        Some(articulation_matrix)
    }

    /// Applies the articulation to a node's original transform, returning the
    /// new local transform.
    pub fn apply_to_transform(&mut self, original_transform: &Matrix4) -> Option<Matrix4> {
        self.apply().map(|articulation_matrix| {
            Matrix4::multiply_transformation_new(original_transform, &articulation_matrix)
        })
    }
}

impl Default for ModelArticulation {
    fn default() -> Self {
        Self::new("")
    }
}
