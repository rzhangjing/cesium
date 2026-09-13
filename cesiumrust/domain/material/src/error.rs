//! Errors produced by the Fabric material system.
//!
//! Maps to the `DeveloperError` throws in CesiumJS `Scene/Material.js`
//! (`checkForTemplateErrors`, `createUniform`, `createSubMaterials`,
//! `Material.fromType`).

use thiserror::Error;

/// Errors raised while parsing Fabric JSON or assembling a material.
#[derive(Debug, Error, PartialEq)]
pub enum MaterialError {
    /// `fabric: cannot have source and components in the same template.`
    #[error("fabric: cannot have source and components in the same template.")]
    SourceAndComponents,

    /// `fabric: property name '<property>' is not valid. It should be ...`
    #[error(
        "fabric: property name '{property}' is not valid. It should be {expected}."
    )]
    InvalidPropertyName {
        /// The offending property name.
        property: String,
        /// Comma separated list of valid property names.
        expected: String,
    },

    /// `fabric: uniforms and materials cannot share the same property '<name>'`
    #[error("fabric: uniforms and materials cannot share the same property '{name}'")]
    DuplicateUniformMaterialName {
        /// The shared property name.
        name: String,
    },

    /// `fabric: uniform '<uniform>' has invalid type.`
    #[error("fabric: uniform '{uniform}' has invalid type.")]
    InvalidUniformType {
        /// The uniform name whose value could not be typed.
        uniform: String,
    },

    /// A uniform value could not be parsed from JSON.
    #[error("fabric: uniform '{uniform}' has an invalid value: {reason}")]
    InvalidUniformValue {
        /// The uniform name (or a placeholder for anonymous values).
        uniform: String,
        /// Human readable reason.
        reason: String,
    },

    /// `strict: shader source does not use uniform '<uniform>'.`
    #[error("strict: shader source does not use uniform '{uniform}'.")]
    StrictUnusedUniform {
        /// The unused uniform name.
        uniform: String,
    },

    /// `strict: shader source does not use channels '<uniform>'.`
    #[error("strict: shader source does not use channels '{uniform}'.")]
    StrictUnusedChannels {
        /// The unused channels uniform name.
        uniform: String,
    },

    /// `strict: shader source does not use material '<id>'.`
    #[error("strict: shader source does not use material '{id}'.")]
    StrictUnusedMaterial {
        /// The unused sub-material id.
        id: String,
    },

    /// `material with type '<type>' does not exist.`
    #[error("material with type '{type_name}' does not exist.")]
    UnknownMaterialType {
        /// The requested material type.
        type_name: String,
    },

    /// Invalid Fabric JSON document.
    #[error("invalid fabric JSON: {0}")]
    Json(String),
}

impl From<serde_json::Error> for MaterialError {
    fn from(e: serde_json::Error) -> Self {
        MaterialError::Json(e.to_string())
    }
}
