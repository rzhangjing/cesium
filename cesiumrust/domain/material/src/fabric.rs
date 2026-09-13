//! The Fabric JSON schema: the declarative material description language.
//!
//! Maps to the `fabric` option of CesiumJS `Scene/Material.js`. A Fabric
//! template is a JSON object with up to five properties:
//!
//! - `type`: the material type name (existing or new)
//! - `uniforms`: map of uniform name → value
//! - `materials`: map of sub-material name → nested Fabric template
//! - `components`: the `czm_material` component expressions
//!   (`diffuse`/`specular`/`shininess`/`normal`/`emission`/`alpha`)
//! - `source`: a full custom `czm_getMaterial` GLSL definition
//!
//! `source` and `components` are mutually exclusive.

use crate::error::MaterialError;
use crate::uniform::{uniform_value_from_json, UniformValue};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;

/// The valid top-level properties of a Fabric template.
/// Maps to `templateProperties` in `Material.js`.
pub const TEMPLATE_PROPERTIES: [&str; 5] =
    ["type", "materials", "uniforms", "components", "source"];

/// The valid properties of a Fabric `components` object.
/// Maps to `componentProperties` in `Material.js`.
pub const COMPONENT_PROPERTIES: [&str; 6] = [
    "diffuse", "specular", "shininess", "normal", "emission", "alpha",
];

/// The `czm_material` component expressions of a Fabric template.
///
/// Maps to `template.components` in `Material.js`. Each entry is a GLSL
/// expression string assigned to the corresponding `czm_material` member in
/// the generated `czm_getMaterial` body. Iteration order for shader
/// generation is the canonical CesiumJS order: diffuse, specular, shininess,
/// normal, emission, alpha.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MaterialComponents {
    /// `material.diffuse = <expr>;` (gamma-corrected unless fused).
    pub diffuse: Option<String>,
    /// `material.specular = <expr>;`
    pub specular: Option<String>,
    /// `material.shininess = <expr>;`
    pub shininess: Option<String>,
    /// `material.normal = <expr>;`
    pub normal: Option<String>,
    /// `material.emission = <expr>;` (gamma-corrected unless fused).
    pub emission: Option<String>,
    /// `material.alpha = <expr>;`
    pub alpha: Option<String>,
}

impl MaterialComponents {
    /// Returns true when no component expression is set.
    pub fn is_empty(&self) -> bool {
        self.diffuse.is_none()
            && self.specular.is_none()
            && self.shininess.is_none()
            && self.normal.is_none()
            && self.emission.is_none()
            && self.alpha.is_none()
    }

    /// Iterates the components in canonical CesiumJS order.
    pub fn iter(&self) -> impl Iterator<Item = (&'static str, &str)> {
        [
            ("diffuse", self.diffuse.as_deref()),
            ("specular", self.specular.as_deref()),
            ("shininess", self.shininess.as_deref()),
            ("normal", self.normal.as_deref()),
            ("emission", self.emission.as_deref()),
            ("alpha", self.alpha.as_deref()),
        ]
        .into_iter()
        .filter_map(|(name, expr)| expr.map(|e| (name, e)))
    }

    fn parse(json: &JsonValue) -> Result<Option<Self>, MaterialError> {
        let map = match json {
            JsonValue::Null => return Ok(None),
            JsonValue::Object(map) => map,
            _ => {
                return Err(MaterialError::InvalidPropertyName {
                    property: "<components>".to_string(),
                    expected: COMPONENT_PROPERTIES.join(", "),
                })
            }
        };

        // Validate property names (maps to checkForValidProperties with
        // invalidNameError for components).
        for key in map.keys() {
            if !COMPONENT_PROPERTIES.contains(&key.as_str()) {
                return Err(MaterialError::InvalidPropertyName {
                    property: key.clone(),
                    expected: "'diffuse', 'specular', 'shininess', 'normal', 'emission', or 'alpha'"
                        .to_string(),
                });
            }
        }

        let expr = |key: &str| -> Option<String> {
            map.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
        };

        Ok(Some(MaterialComponents {
            diffuse: expr("diffuse"),
            specular: expr("specular"),
            shininess: expr("shininess"),
            normal: expr("normal"),
            emission: expr("emission"),
            alpha: expr("alpha"),
        }))
    }
}

/// A parsed Fabric material template.
///
/// Maps to the cloned `options.fabric` / `_template` object in
/// `Material.js`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FabricTemplate {
    /// The material type name (`template.type`); a GUID is generated during
    /// material construction when absent.
    pub type_name: Option<String>,
    /// Uniform name → value (`template.uniforms`).
    pub uniforms: BTreeMap<String, UniformValue>,
    /// Sub-material name → nested template (`template.materials`).
    pub materials: BTreeMap<String, FabricTemplate>,
    /// Component expressions (`template.components`).
    pub components: Option<MaterialComponents>,
    /// Custom `czm_getMaterial` GLSL source (`template.source`).
    pub source: Option<String>,
}

impl FabricTemplate {
    /// Parses a Fabric template from a JSON value.
    pub fn from_json(json: &JsonValue) -> Result<Self, MaterialError> {
        let map = match json {
            JsonValue::Object(map) => map,
            JsonValue::Null => return Ok(FabricTemplate::default()),
            _ => {
                return Err(MaterialError::Json(
                    "fabric must be a JSON object".to_string(),
                ))
            }
        };

        // Validate top-level property names (maps to checkForValidProperties
        // with invalidNameError for the template).
        for key in map.keys() {
            if !TEMPLATE_PROPERTIES.contains(&key.as_str()) {
                return Err(MaterialError::InvalidPropertyName {
                    property: key.clone(),
                    expected: "'type', 'materials', 'uniforms', 'components', or 'source'"
                        .to_string(),
                });
            }
        }

        let type_name = map
            .get("type")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut uniforms = BTreeMap::new();
        if let Some(JsonValue::Object(uniform_map)) = map.get("uniforms") {
            for (name, value) in uniform_map {
                uniforms.insert(
                    name.clone(),
                    uniform_value_from_json(value).map_err(|e| match e {
                        MaterialError::InvalidUniformValue { reason, .. } => {
                            MaterialError::InvalidUniformValue {
                                uniform: name.clone(),
                                reason,
                            }
                        }
                        other => other,
                    })?,
                );
            }
        }

        let mut materials = BTreeMap::new();
        if let Some(JsonValue::Object(material_map)) = map.get("materials") {
            for (name, sub_json) in material_map {
                materials.insert(name.clone(), FabricTemplate::from_json(sub_json)?);
            }
        }

        let components = match map.get("components") {
            Some(c) => MaterialComponents::parse(c)?,
            None => None,
        };

        let source = map
            .get("source")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(FabricTemplate {
            type_name,
            uniforms,
            materials,
            components,
            source,
        })
    }

    /// Parses a Fabric template from a JSON string.
    /// Maps to `parse_fabric` in the architecture plan.
    pub fn from_json_str(json: &str) -> Result<Self, MaterialError> {
        let value: JsonValue = serde_json::from_str(json)?;
        Self::from_json(&value)
    }

    /// Validates the template for structural errors.
    ///
    /// Maps to `checkForTemplateErrors` in `Material.js`:
    /// - `source` and `components` cannot coexist
    /// - uniforms and materials cannot share a name
    ///
    /// Property-name validation already happens during parsing.
    pub fn validate(&self) -> Result<(), MaterialError> {
        if self.components.is_some() && self.source.is_some() {
            return Err(MaterialError::SourceAndComponents);
        }
        for name in self.uniforms.keys() {
            if self.materials.contains_key(name) {
                return Err(MaterialError::DuplicateUniformMaterialName {
                    name: name.clone(),
                });
            }
        }
        for sub in self.materials.values() {
            sub.validate()?;
        }
        Ok(())
    }

    /// Deep-merges `base` into `self`, with `self` taking precedence.
    ///
    /// Maps to `combine(result._template, template, true)` in
    /// `initializeMaterial`: the user-provided template wins, and any keys
    /// missing from it are filled in from the cached (base) template.
    pub fn merge_over(&mut self, base: &FabricTemplate) {
        if self.type_name.is_none() {
            self.type_name = base.type_name.clone();
        }
        for (name, value) in &base.uniforms {
            self.uniforms.entry(name.clone()).or_insert_with(|| value.clone());
        }
        for (name, sub_base) in &base.materials {
            match self.materials.get_mut(name) {
                Some(sub) => sub.merge_over(sub_base),
                None => {
                    self.materials.insert(name.clone(), sub_base.clone());
                }
            }
        }
        if self.components.is_none() {
            self.components = base.components.clone();
        }
        if self.source.is_none() {
            self.source = base.source.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_minimal() {
        let t = FabricTemplate::from_json_str("{}").unwrap();
        assert!(t.type_name.is_none());
        assert!(t.uniforms.is_empty());
        assert!(t.materials.is_empty());
        assert!(t.components.is_none());
        assert!(t.source.is_none());
    }

    #[test]
    fn test_parse_color_fabric() {
        let t = FabricTemplate::from_json(&json!({
            "type": "Color",
            "uniforms": {
                "color": {"red": 1.0, "green": 1.0, "blue": 0.0, "alpha": 1.0}
            }
        }))
        .unwrap();
        assert_eq!(t.type_name.as_deref(), Some("Color"));
        assert_eq!(
            t.uniforms.get("color"),
            Some(&UniformValue::Vec4([1.0, 1.0, 0.0, 1.0]))
        );
    }

    #[test]
    fn test_parse_components() {
        let t = FabricTemplate::from_json(&json!({
            "components": {
                "diffuse": "color.rgb",
                "alpha": "color.a"
            }
        }))
        .unwrap();
        let c = t.components.as_ref().unwrap();
        assert_eq!(c.diffuse.as_deref(), Some("color.rgb"));
        assert_eq!(c.alpha.as_deref(), Some("color.a"));
        assert!(c.specular.is_none());
        let names: Vec<_> = c.iter().map(|(n, _)| n).collect();
        assert_eq!(names, vec!["diffuse", "alpha"]);
    }

    #[test]
    fn test_parse_nested_materials() {
        let t = FabricTemplate::from_json(&json!({
            "materials": {
                "diffuseMap": {
                    "type": "DiffuseMap"
                }
            },
            "components": {
                "diffuse": "diffuseMap.diffuse"
            }
        }))
        .unwrap();
        assert_eq!(t.materials.len(), 1);
        let sub = t.materials.get("diffuseMap").unwrap();
        assert_eq!(sub.type_name.as_deref(), Some("DiffuseMap"));
    }

    #[test]
    fn test_invalid_top_level_property() {
        let err = FabricTemplate::from_json(&json!({"bogus": 1})).unwrap_err();
        assert!(matches!(err, MaterialError::InvalidPropertyName { .. }));
    }

    #[test]
    fn test_invalid_component_property() {
        let err = FabricTemplate::from_json(&json!({
            "components": {"glossy": "1.0"}
        }))
        .unwrap_err();
        assert!(matches!(err, MaterialError::InvalidPropertyName { .. }));
    }

    #[test]
    fn test_source_and_components_conflict() {
        let t = FabricTemplate::from_json(&json!({
            "source": "czm_material czm_getMaterial(czm_materialInput materialInput) { }",
            "components": {"diffuse": "vec3(1.0)"}
        }))
        .unwrap();
        assert_eq!(t.validate(), Err(MaterialError::SourceAndComponents));
    }

    #[test]
    fn test_uniform_material_name_conflict() {
        let t = FabricTemplate::from_json(&json!({
            "uniforms": {"shared": 1.0},
            "materials": {"shared": {"type": "Color"}}
        }))
        .unwrap();
        assert_eq!(
            t.validate(),
            Err(MaterialError::DuplicateUniformMaterialName {
                name: "shared".to_string()
            })
        );
    }

    #[test]
    fn test_merge_over_precedence() {
        let mut user = FabricTemplate::from_json(&json!({
            "type": "Color",
            "uniforms": {
                "color": {"red": 0.0, "green": 1.0, "blue": 0.0, "alpha": 1.0}
            }
        }))
        .unwrap();
        let cached = FabricTemplate::from_json(&json!({
            "type": "Color",
            "uniforms": {
                "color": {"red": 1.0, "green": 0.0, "blue": 0.0, "alpha": 0.5},
                "extra": 2.0
            },
            "components": {"diffuse": "color.rgb", "alpha": "color.a"}
        }))
        .unwrap();

        user.merge_over(&cached);

        // User's color wins; extra uniform filled from cache; components
        // filled from cache.
        assert_eq!(
            user.uniforms.get("color"),
            Some(&UniformValue::Vec4([0.0, 1.0, 0.0, 1.0]))
        );
        assert_eq!(
            user.uniforms.get("extra"),
            Some(&UniformValue::Float(2.0))
        );
        assert!(user.components.is_some());
    }
}
