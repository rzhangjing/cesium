//! Ported from `packages/engine/Source/Scene/Model/ModelSkin.js`.
//!
//! An in-memory representation of a skin that affects nodes in the model scene graph.

use cesium_core::matrix4::Matrix4;

/// An in-memory representation of a skin for vertex deformation in a model.
///
/// Skins should only be initialized after all runtime nodes have been
/// instantiated by the scene graph.
///
/// Mirrors CesiumJS `ModelSkin` (181 lines).
pub struct ModelSkin {
    /// The inverse bind matrices of the skin.
    pub inverse_bind_matrices: Vec<Matrix4>,
    /// The joint indices (into the scene graph's runtime node array).
    pub joint_indices: Vec<usize>,
    /// The computed joint matrices: `jointMatrix = jointWorldTransform * inverseBindMatrix`.
    pub joint_matrices: Vec<Matrix4>,
}

impl ModelSkin {
    /// Creates a new `ModelSkin` from inverse bind matrices and joint indices.
    pub fn new(inverse_bind_matrices: Vec<Matrix4>, joint_indices: Vec<usize>) -> Self {
        let joint_matrices = vec![Matrix4::IDENTITY; joint_indices.len()];
        Self {
            inverse_bind_matrices,
            joint_indices,
            joint_matrices,
        }
    }

    /// Returns the number of joints in this skin.
    pub fn joints_length(&self) -> usize {
        self.joint_indices.len()
    }

    /// Computes the joint matrix for a single joint.
    ///
    /// `joint_matrix = joint_world_transform * inverse_bind_matrix`
    ///
    /// Mirrors CesiumJS `computeJointMatrix`.
    pub fn compute_joint_matrix(
        joint_world_transform: &Matrix4,
        inverse_bind_matrix: &Matrix4,
        result: &mut Matrix4,
    ) {
        Matrix4::multiply_transformation(joint_world_transform, inverse_bind_matrix, result);
    }

    /// Computes the joint matrix and returns a new matrix.
    pub fn compute_joint_matrix_new(
        joint_world_transform: &Matrix4,
        inverse_bind_matrix: &Matrix4,
    ) -> Matrix4 {
        Matrix4::multiply_transformation_new(joint_world_transform, inverse_bind_matrix)
    }

    /// Updates all joint matrices given a slice of joint world transforms.
    ///
    /// Each entry in `joint_world_transforms` corresponds to a joint in
    /// [`joint_indices`](Self::joint_indices). The transform at index `i`
    /// is `joint.transformToRoot * joint.transform` for runtime node `i`.
    ///
    /// Mirrors CesiumJS `ModelSkin.prototype.updateJointMatrices`.
    pub fn update_joint_matrices(&mut self, joint_world_transforms: &[Matrix4]) {
        let length = self.joint_matrices.len();
        for i in 0..length {
            if i < joint_world_transforms.len() && i < self.inverse_bind_matrices.len() {
                Self::compute_joint_matrix(
                    &joint_world_transforms[i],
                    &self.inverse_bind_matrices[i],
                    &mut self.joint_matrices[i],
                );
            }
        }
    }

    /// Initializes joint matrices from joint world transforms.
    ///
    /// This is the Rust equivalent of the JS `initialize()` function called
    /// in the constructor.
    pub fn initialize(&mut self, joint_world_transforms: &[Matrix4]) {
        let length = self.joint_indices.len();
        self.joint_matrices = vec![Matrix4::IDENTITY; length];
        for i in 0..length {
            if i < joint_world_transforms.len() && i < self.inverse_bind_matrices.len() {
                let joint_matrix = Self::compute_joint_matrix_new(
                    &joint_world_transforms[i],
                    &self.inverse_bind_matrices[i],
                );
                self.joint_matrices[i] = joint_matrix;
            }
        }
    }
}

impl Default for ModelSkin {
    fn default() -> Self {
        Self {
            inverse_bind_matrices: Vec::new(),
            joint_indices: Vec::new(),
            joint_matrices: Vec::new(),
        }
    }
}
