//! Material assembly: Fabric template -> GLSL shader source + uniforms.
//!
//! Faithful port of the construction pipeline in CesiumJS
//! `Scene/Material.js`: `initializeMaterial`, `createMethodDefinition`,
//! `createUniforms`/`createUniform`, `createSubMaterials`, `replaceToken`,
//! `getNumberOfTokens` and `isTranslucent`.
//!
//! The domain layer produces the same GLSL `czm_getMaterial` shader source and
//! uniform bookkeeping that CesiumJS generates; the render adapter is then
//! responsible for translating that source to the target shading language.

use crate::cache::CachedMaterial;
use crate::error::MaterialError;
use crate::fabric::FabricTemplate;
use crate::translucent::TranslucentSpec;
use crate::uniform::UniformValue;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

/// Counter used to generate GUIDs for anonymous material types.
/// Maps to `createGuid()` in `Material.js`.
static GUID_COUNTER: AtomicU64 = AtomicU64::new(0);

fn create_guid() -> String {
    let n = GUID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("material-{:016x}", n)
}

/// Options for constructing a [`Material`].
///
/// Maps to the `options` object of the CesiumJS `Material` constructor
/// (`{ strict, translucent, fabric, count }`). The `count` member is internal
/// (a shared rename counter) and is threaded through the build functions as a
/// `&mut usize` instead.
#[derive(Debug, Clone, Default)]
pub struct MaterialOptions {
    /// When `true`, unused uniforms / channels / sub-materials are errors.
    /// Maps to `options.strict`.
    pub strict: bool,
    /// Explicit translucency override. Maps to `options.translucent`.
    ///
    /// The domain layer supports the boolean form only; the function form used
    /// by the built-in materials is captured by [`TranslucentSpec`].
    pub translucent: Option<bool>,
    /// The Fabric template. Maps to `options.fabric`.
    pub fabric: FabricTemplate,
}

/// A constructed Fabric material.
///
/// Maps to a CesiumJS `Material` instance. The [`Material::shader_source`] is
/// the fully assembled GLSL (sub-material functions prepended, uniforms
/// renamed to unique ids). Sub-materials are kept as nested [`Material`]s so
/// their uniform values remain individually addressable, mirroring
/// `material.materials`.
#[derive(Debug, Clone)]
pub struct Material {
    type_name: String,
    shader_source: String,
    /// Public uniform values keyed by their original (Fabric) names.
    /// Maps to `material.uniforms`.
    uniforms: BTreeMap<String, UniformValue>,
    /// Sub-materials keyed by their Fabric names. Maps to `material.materials`.
    materials: BTreeMap<String, Material>,
    /// The resolved translucency for this material itself (the value pushed
    /// onto `_translucentFunctions` at the end of `initializeMaterial`).
    own_translucent: Option<TranslucentSpec>,
    /// Renamed (shader) uniform id -> original (Fabric) uniform id.
    /// Maps to the keys/getters of `material._uniforms`.
    uniform_bindings: BTreeMap<String, String>,
}

impl Material {
    /// The material type name (a GUID when the Fabric had no `type`).
    pub fn type_name(&self) -> &str {
        &self.type_name
    }

    /// The assembled GLSL shader source. Maps to `material.shaderSource`.
    pub fn shader_source(&self) -> &str {
        &self.shader_source
    }

    /// Public uniform values keyed by original Fabric names.
    pub fn uniforms(&self) -> &BTreeMap<String, UniformValue> {
        &self.uniforms
    }

    /// Mutable access to the public uniform values.
    pub fn uniforms_mut(&mut self) -> &mut BTreeMap<String, UniformValue> {
        &mut self.uniforms
    }

    /// Sub-materials keyed by Fabric name.
    pub fn materials(&self) -> &BTreeMap<String, Material> {
        &self.materials
    }

    /// Renamed (shader) uniform id -> original Fabric uniform id.
    pub fn uniform_bindings(&self) -> &BTreeMap<String, String> {
        &self.uniform_bindings
    }

    /// Whether this material (and all of its sub-materials) is translucent.
    ///
    /// Maps to `Material.prototype.isTranslucent`: CesiumJS ANDs together
    /// every function on `_translucentFunctions`, which is the flattened set
    /// of this material's own spec plus every descendant's spec.
    pub fn is_translucent(&self) -> bool {
        let own = self
            .own_translucent
            .as_ref()
            .map(|spec| spec.evaluate(&self.uniforms))
            .unwrap_or(true);
        own && self.materials.values().all(Material::is_translucent)
    }

    /// Flattened map of shader (renamed) uniform id -> current value, covering
    /// this material and all sub-materials. Convenient for a render adapter
    /// that binds uniforms by their shader names.
    pub fn shader_uniforms(&self) -> BTreeMap<String, UniformValue> {
        let mut out = BTreeMap::new();
        self.collect_shader_uniforms(&mut out);
        out
    }

    fn collect_shader_uniforms(&self, out: &mut BTreeMap<String, UniformValue>) {
        for (renamed, original) in &self.uniform_bindings {
            if let Some(value) = self.uniforms.get(original) {
                out.insert(renamed.clone(), value.clone());
            }
        }
        for sub in self.materials.values() {
            sub.collect_shader_uniforms(out);
        }
    }

    /// The names of all texture (`sampler2D` / `samplerCube`) uniforms across
    /// this material and its sub-materials, keyed by original Fabric name.
    pub fn texture_uniforms(&self) -> BTreeMap<String, UniformValue> {
        let mut out = BTreeMap::new();
        self.collect_texture_uniforms(&mut out);
        out
    }

    fn collect_texture_uniforms(&self, out: &mut BTreeMap<String, UniformValue>) {
        for (name, value) in &self.uniforms {
            if matches!(
                value,
                UniformValue::Sampler2D(_) | UniformValue::SamplerCube(_)
            ) {
                out.insert(name.clone(), value.clone());
            }
        }
        for sub in self.materials.values() {
            sub.collect_texture_uniforms(out);
        }
    }
}

/// Builds a [`Material`] from a Fabric template.
///
/// This is the top-level entry used by the material cache. It returns the
/// material together with the number of translucent functions collected on its
/// `_translucentFunctions` (own + descendants), which a parent needs in order
/// to compute its `defaultTranslucent`.
pub(crate) fn build_material(
    fabric: FabricTemplate,
    strict: bool,
    options_translucent: Option<bool>,
    cache: &HashMap<String, CachedMaterial>,
    count: &mut usize,
) -> Result<(Material, usize), MaterialError> {
    // `result._template = clone(options.fabric)` -- we already own the clone.
    let mut template = fabric;

    // `result.type = template.type ?? createGuid()`
    let type_name = template.type_name.clone().unwrap_or_else(create_guid);

    // Cache merge: build the template off the stored one (user wins).
    let cached = cache.get(&type_name);
    let cached_translucent: Option<TranslucentSpec> = if let Some(cached_material) = cached {
        template.merge_over(&cached_material.fabric);
        cached_material.translucent.clone()
    } else {
        None
    };

    // `checkForTemplateErrors`
    template.validate()?;

    // `createMethodDefinition`
    let mut shader_source = String::new();
    create_method_definition(&template, &mut shader_source);

    // `createUniforms`
    let mut uniforms = BTreeMap::new();
    let mut uniform_bindings = BTreeMap::new();
    create_uniforms(
        &template,
        strict,
        &mut shader_source,
        &mut uniforms,
        &mut uniform_bindings,
        count,
    )?;

    // `createSubMaterials`
    let mut materials = BTreeMap::new();
    let mut sub_translucent_count = 0usize;
    create_sub_materials(
        &template,
        strict,
        cache,
        count,
        &mut shader_source,
        &mut materials,
        &mut sub_translucent_count,
    )?;

    // Resolve translucency:
    //   defaultTranslucent = _translucentFunctions.length === 0 ? true : undefined
    //   translucent = cached ?? defaultTranslucent
    //   translucent = options.translucent ?? translucent
    let default_translucent = if sub_translucent_count == 0 {
        Some(TranslucentSpec::Always)
    } else {
        None
    };
    let resolved = options_translucent
        .map(|b| {
            if b {
                TranslucentSpec::Always
            } else {
                TranslucentSpec::Never
            }
        })
        .or(cached_translucent)
        .or(default_translucent);
    let own_count = usize::from(resolved.is_some());

    Ok((
        Material {
            type_name,
            shader_source,
            uniforms,
            materials,
            own_translucent: resolved,
            uniform_bindings,
        },
        sub_translucent_count + own_count,
    ))
}

/// `isMaterialFused`: does a component expression reference any sub-material?
fn is_material_fused(component_expr: &str, materials: &BTreeMap<String, FabricTemplate>) -> bool {
    materials
        .keys()
        .any(|id| component_expr.contains(id.as_str()))
}

/// `createMethodDefinition`: build the `czm_getMaterial` body from `source`
/// or `components`.
fn create_method_definition(template: &FabricTemplate, shader_source: &mut String) {
    if let Some(source) = &template.source {
        shader_source.push_str(source);
        shader_source.push('\n');
        return;
    }

    shader_source
        .push_str("czm_material czm_getMaterial(czm_materialInput materialInput)\n{\n");
    shader_source.push_str("czm_material material = czm_getDefaultMaterial(materialInput);\n");

    if let Some(components) = &template.components {
        let is_multi_material = !template.materials.is_empty();
        for (component, expr) in components.iter() {
            if component == "diffuse" || component == "emission" {
                let is_fusion = is_multi_material && is_material_fused(expr, &template.materials);
                let component_source = if is_fusion {
                    expr.to_string()
                } else {
                    format!("czm_gammaCorrect({})", expr)
                };
                // Note the trailing space before the newline (CesiumJS emits
                // `material.<c> = <src>; \n` for diffuse/emission/alpha).
                shader_source.push_str(&format!("material.{} = {}; \n", component, component_source));
            } else if component == "alpha" {
                shader_source.push_str(&format!("material.alpha = {}; \n", expr));
            } else {
                shader_source.push_str(&format!("material.{} = {};\n", component, expr));
            }
        }
    }

    shader_source.push_str("return material;\n}\n");
}

/// `createUniforms`: process every uniform declared in the template.
fn create_uniforms(
    template: &FabricTemplate,
    strict: bool,
    shader_source: &mut String,
    uniforms: &mut BTreeMap<String, UniformValue>,
    bindings: &mut BTreeMap<String, String>,
    count: &mut usize,
) -> Result<(), MaterialError> {
    // Work on a growable copy so we can add `<image>Dimensions` uniforms
    // dynamically (as CesiumJS mutates `material._template.uniforms`).
    let mut all_uniforms = template.uniforms.clone();
    let ids: Vec<String> = template.uniforms.keys().cloned().collect();
    let mut processed = HashSet::new();
    for id in ids {
        create_uniform(
            &id,
            strict,
            shader_source,
            &mut all_uniforms,
            uniforms,
            bindings,
            count,
            &mut processed,
        )?;
    }
    Ok(())
}

/// `createUniform`: declare, rename and bind a single uniform.
#[allow(clippy::too_many_arguments)]
fn create_uniform(
    uniform_id: &str,
    strict: bool,
    shader_source: &mut String,
    all_uniforms: &mut BTreeMap<String, UniformValue>,
    uniforms: &mut BTreeMap<String, UniformValue>,
    bindings: &mut BTreeMap<String, String>,
    count: &mut usize,
    processed: &mut HashSet<String>,
) -> Result<(), MaterialError> {
    if !processed.insert(uniform_id.to_string()) {
        return Ok(());
    }

    let uniform_value = all_uniforms
        .get(uniform_id)
        .cloned()
        .ok_or_else(|| MaterialError::InvalidUniformType {
            uniform: uniform_id.to_string(),
        })?;
    let uniform_type = uniform_value.glsl_type();

    if uniform_type == "channels" {
        // Channels are a textual substitution, not a real uniform.
        let channels_str = match &uniform_value {
            UniformValue::Channels(s) => s.clone(),
            _ => unreachable!("glsl_type() reported channels for a non-Channels value"),
        };
        let replaced = replace_token(shader_source, uniform_id, &channels_str, false);
        if replaced == 0 && strict {
            return Err(MaterialError::StrictUnusedChannels {
                uniform: uniform_id.to_string(),
            });
        }
        return Ok(());
    }

    // WebGL cannot query texture dimensions in GLSL, so CesiumJS creates a
    // companion `<image>Dimensions` ivec3 uniform when the source uses it.
    if uniform_type == "sampler2D" {
        let dims_name = format!("{}Dimensions", uniform_id);
        if get_number_of_tokens(shader_source, &dims_name) > 0 {
            all_uniforms.insert(dims_name.clone(), UniformValue::IVec3([1, 1, 0]));
            create_uniform(
                &dims_name,
                strict,
                shader_source,
                all_uniforms,
                uniforms,
                bindings,
                count,
                processed,
            )?;
        }
    }

    // Prepend the declaration if the source does not already declare it.
    if !has_uniform_declaration(shader_source, uniform_type, uniform_id) {
        let declaration = format!("uniform {} {};", uniform_type, uniform_id);
        shader_source.insert_str(0, &declaration);
    }

    // Rename to a unique id: `<id>_<count++>`.
    let new_id = format!("{}_{}", uniform_id, *count);
    *count += 1;
    let replaced = replace_token(shader_source, uniform_id, &new_id, true);
    // A count of exactly one means only the declaration was renamed, i.e. the
    // uniform is declared but never used in the body.
    if replaced == 1 && strict {
        return Err(MaterialError::StrictUnusedUniform {
            uniform: uniform_id.to_string(),
        });
    }

    uniforms.insert(uniform_id.to_string(), uniform_value);
    bindings.insert(new_id, uniform_id.to_string());
    Ok(())
}

/// `createSubMaterials`: recursively build sub-materials and splice their
/// shader source and method calls into the parent.
#[allow(clippy::too_many_arguments)]
fn create_sub_materials(
    template: &FabricTemplate,
    strict: bool,
    cache: &HashMap<String, CachedMaterial>,
    count: &mut usize,
    shader_source: &mut String,
    materials: &mut BTreeMap<String, Material>,
    sub_translucent_count: &mut usize,
) -> Result<(), MaterialError> {
    for (sub_id, sub_template) in &template.materials {
        // Construct the sub-material (no options.translucent for sub-materials).
        let (mut sub_material, sub_count) =
            build_material(sub_template.clone(), strict, None, cache, count)?;
        *sub_translucent_count += sub_count;

        // Make the sub-material's czm_getMaterial unique.
        let new_method_name = format!("czm_getMaterial_{}", *count);
        *count += 1;
        replace_token(
            &mut sub_material.shader_source,
            "czm_getMaterial",
            &new_method_name,
            true,
        );

        // Prepend the sub-material's source.
        let sub_source = std::mem::take(&mut sub_material.shader_source);
        let parent_source = std::mem::take(shader_source);
        *shader_source = sub_source + &parent_source;

        // Replace each material id with a czm_getMaterial method call.
        let method_call = format!("{}(materialInput)", new_method_name);
        let replaced = replace_token(shader_source, sub_id, &method_call, true);
        if replaced == 0 && strict {
            return Err(MaterialError::StrictUnusedMaterial { id: sub_id.clone() });
        }

        materials.insert(sub_id.clone(), sub_material);
    }
    Ok(())
}

/// `replaceToken`: replace standalone occurrences of `token` in `source` with
/// `new_token`, returning the number of replacements.
///
/// A standalone occurrence is one not preceded by a word character (and not by
/// a period when `exclude_period` is true) and not followed by a word
/// character. Faithful byte-level port of the CesiumJS regex
/// `([\w.])?token([\w])?` (with the `.` dropped from the prefix class when
/// `exclude_period` is false).
pub(crate) fn replace_token(
    source: &mut String,
    token: &str,
    new_token: &str,
    exclude_period: bool,
) -> usize {
    let bytes = source.as_bytes();
    let token_bytes = token.as_bytes();
    let tlen = token_bytes.len();
    let n = bytes.len();
    if tlen == 0 || tlen > n {
        return 0;
    }

    let mut result: Vec<u8> = Vec::with_capacity(n);
    let mut count = 0usize;
    let mut i = 0usize;
    while i < n {
        if i + tlen <= n && &bytes[i..i + tlen] == token_bytes {
            let prefix_ok = if i == 0 {
                true
            } else {
                let prev = bytes[i - 1];
                if exclude_period {
                    !is_word_byte(prev) && prev != b'.'
                } else {
                    !is_word_byte(prev)
                }
            };
            let suffix_ok = i + tlen >= n || !is_word_byte(bytes[i + tlen]);
            if prefix_ok && suffix_ok {
                result.extend_from_slice(new_token.as_bytes());
                count += 1;
                i += tlen;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }

    // SAFETY: the source was valid UTF-8; we copied every byte verbatim except
    // ASCII token ranges, which were replaced with ASCII `new_token`.
    *source = String::from_utf8(result).expect("replace_token preserves UTF-8 validity");
    count
}

/// `getNumberOfTokens`: count standalone occurrences without modifying source.
/// CesiumJS implements this as `replaceToken(material, token, token, ...)`.
fn get_number_of_tokens(source: &mut String, token: &str) -> usize {
    replace_token(source, token, token, true)
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Tests whether the source already contains `uniform <type> <id> ;` with
/// flexible whitespace. Faithful to the CesiumJS substring regex
/// `uniform\s+<type>\s+<id>\s*;`.
fn has_uniform_declaration(source: &str, uniform_type: &str, uniform_id: &str) -> bool {
    let bytes = source.as_bytes();
    let n = bytes.len();
    let kw = b"uniform";
    let type_bytes = uniform_type.as_bytes();
    let id_bytes = uniform_id.as_bytes();

    let mut i = 0usize;
    while i + kw.len() <= n {
        if &bytes[i..i + kw.len()] == kw {
            let mut j = i + kw.len();
            let s1 = j;
            while j < n && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j > s1 && j + type_bytes.len() <= n && &bytes[j..j + type_bytes.len()] == type_bytes
            {
                let mut k = j + type_bytes.len();
                let s2 = k;
                while k < n && bytes[k].is_ascii_whitespace() {
                    k += 1;
                }
                if k > s2 && k + id_bytes.len() <= n && &bytes[k..k + id_bytes.len()] == id_bytes {
                    let mut m = k + id_bytes.len();
                    while m < n && bytes[m].is_ascii_whitespace() {
                        m += 1;
                    }
                    if m < n && bytes[m] == b';' {
                        return true;
                    }
                }
            }
        }
        i += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::MaterialSystem;

    fn build(fabric: &str) -> Material {
        let template = FabricTemplate::from_json_str(fabric).unwrap();
        let system = MaterialSystem::with_builtin_materials();
        system
            .build(template, false, None)
            .expect("material should build")
    }

    #[test]
    fn test_color_material_shader() {
        let m = build(r#"{"type": "Color"}"#);
        assert_eq!(m.type_name(), "Color");
        let src = m.shader_source();
        // The color uniform is declared and renamed with a unique suffix.
        assert!(src.contains("uniform vec4 color_"));
        // diffuse is gamma-corrected (Color is not a multi-material fusion).
        assert!(src.contains("material.diffuse = czm_gammaCorrect(color_"));
        assert!(src.contains(".rgb); \n"));
        // alpha assignment carries the trailing space.
        assert!(src.contains("material.alpha = color_"));
        assert!(src.contains(".a; \n"));
        assert!(src.contains("return material;"));
        // The uniform value is the built-in default.
        assert_eq!(
            m.uniforms().get("color"),
            Some(&UniformValue::Vec4([1.0, 0.0, 0.0, 0.5]))
        );
    }

    #[test]
    fn test_color_material_translucency() {
        // Default Color has alpha 0.5 -> translucent.
        let m = build(r#"{"type": "Color"}"#);
        assert!(m.is_translucent());
    }

    #[test]
    fn test_color_material_opaque_override() {
        let m = build(r#"{"type": "Color", "uniforms": {"color": {"red": 1.0, "green": 0.0, "blue": 0.0, "alpha": 1.0}}}"#);
        assert!(!m.is_translucent());
    }

    #[test]
    fn test_checkerboard_renames_both_colors() {
        let m = build(r#"{"type": "Checkerboard"}"#);
        let src = m.shader_source();
        assert!(src.contains("uniform vec4 lightColor_"));
        assert!(src.contains("uniform vec4 darkColor_"));
        // The source uses the renamed uniforms.
        assert!(src.contains("lightColor_"));
        assert!(src.contains("darkColor_"));
        // Checkerboard's GLSL source is embedded.
        assert!(src.contains("czm_getMaterial"));
        // Default light/dark colors have alpha 0.5 -> translucent.
        assert!(m.is_translucent());
    }

    #[test]
    fn test_image_material_texture_and_repeat() {
        let m = build(r#"{"type": "Image"}"#);
        let src = m.shader_source();
        assert!(src.contains("uniform sampler2D image_"));
        assert!(src.contains("uniform vec2 repeat_"));
        assert!(src.contains("texture(image_"));
        assert!(src.contains("fract(repeat_"));
        // The default image uniform is the sentinel id.
        assert_eq!(
            m.uniforms().get("image"),
            Some(&UniformValue::Sampler2D(
                crate::uniform::DEFAULT_IMAGE_ID.to_string()
            ))
        );
        assert!(m.texture_uniforms().contains_key("image"));
    }

    #[test]
    fn test_diffuse_map_channels_substitution() {
        let m = build(r#"{"type": "DiffuseMap"}"#);
        let src = m.shader_source();
        // The `channels` token is textually replaced with `rgb`.
        assert!(src.contains(".rgb"));
        assert!(!src.contains(".channels"));
        // channels is not a real uniform.
        assert!(!m.uniforms().contains_key("channels"));
        assert!(m.uniforms().contains_key("image"));
        assert!(m.uniforms().contains_key("repeat"));
        // DiffuseMap is never translucent.
        assert!(!m.is_translucent());
    }

    #[test]
    fn test_bump_map_auto_dimensions_uniform() {
        let m = build(r#"{"type": "BumpMap"}"#);
        // BumpMap's GLSL uses imageDimensions, so the companion uniform exists.
        let dims = m.uniforms().get("imageDimensions");
        assert_eq!(dims, Some(&UniformValue::IVec3([1, 1, 0])));
        assert!(m.shader_source().contains("uniform ivec3 imageDimensions_"));
        assert!(!m.is_translucent());
    }

    #[test]
    fn test_custom_components_material_gets_guid_type() {
        let m = build(r#"{"components": {"diffuse": "vec3(1.0)", "alpha": "0.5"}}"#);
        assert!(m.type_name().starts_with("material-"));
        let src = m.shader_source();
        assert!(src.contains("material.diffuse = czm_gammaCorrect(vec3(1.0)); \n"));
        assert!(src.contains("material.alpha = 0.5; \n"));
    }

    #[test]
    fn test_custom_source_material() {
        let src = "czm_material czm_getMaterial(czm_materialInput materialInput)\n{\n  czm_material m = czm_getDefaultMaterial(materialInput);\n  m.diffuse = vec3(0.5);\n  return m;\n}";
        let json = format!(r#"{{"source": {:?}}}"#, src);
        let m = build(&json);
        // Source is emitted verbatim with a trailing newline.
        assert!(m.shader_source().starts_with(src));
        assert!(m.shader_source().ends_with('\n'));
    }

    #[test]
    fn test_sub_material_composition() {
        // A parent that fuses a Color sub-material into its diffuse.
        let m = build(
            r#"{
                "materials": {
                    "base": {"type": "Color"}
                },
                "components": {
                    "diffuse": "base.diffuse",
                    "alpha": "base.alpha"
                }
            }"#,
        );
        let src = m.shader_source();
        // The sub-material's czm_getMaterial was renamed.
        assert!(src.contains("czm_getMaterial_"));
        // The sub-material id was replaced with a method call.
        assert!(src.contains("(materialInput).diffuse"));
        assert!(!src.contains("base.diffuse"));
        // The sub-material is present.
        assert!(m.materials().contains_key("base"));
        // Fused diffuse is NOT wrapped in czm_gammaCorrect.
        assert!(src.contains("material.diffuse = czm_getMaterial_"));
    }

    #[test]
    fn test_sub_material_translucency_propagates() {
        // Parent is opaque by default, but the Color sub-material (alpha 0.5)
        // makes the whole thing translucent.
        let m = build(
            r#"{
                "materials": {"base": {"type": "Color"}},
                "components": {"diffuse": "base.diffuse", "alpha": "base.alpha"}
            }"#,
        );
        assert!(m.is_translucent());
    }

    #[test]
    fn test_strict_unused_uniform_errors() {
        let template = FabricTemplate::from_json_str(
            r#"{"uniforms": {"unused": 1.0}, "components": {"diffuse": "vec3(1.0)"}}"#,
        )
        .unwrap();
        let system = MaterialSystem::with_builtin_materials();
        let err = system
            .build(template, true, None)
            .expect_err("strict build should fail");
        assert!(matches!(
            err,
            MaterialError::StrictUnusedUniform { ref uniform } if uniform == "unused"
        ));
    }

    #[test]
    fn test_strict_unused_channels_errors() {
        let template = FabricTemplate::from_json_str(
            r#"{"uniforms": {"channels": "rgb"}, "components": {"diffuse": "vec3(1.0)"}}"#,
        )
        .unwrap();
        let system = MaterialSystem::with_builtin_materials();
        let err = system
            .build(template, true, None)
            .expect_err("strict build should fail");
        assert!(matches!(
            err,
            MaterialError::StrictUnusedChannels { ref uniform } if uniform == "channels"
        ));
    }

    #[test]
    fn test_replace_token_boundaries() {
        // `lightColor` must not be affected by replacing `color` (case), and
        // `material.diffuse` must not be affected by replacing `diffuse`
        // (period prefix with exclude_period = true).
        let mut s = "material.diffuse = lightColor.rgb;".to_string();
        let n = replace_token(&mut s, "diffuse", "diffuse_0", true);
        assert_eq!(n, 0);
        assert_eq!(s, "material.diffuse = lightColor.rgb;");

        let mut s2 = "vec3 color = color.rgb;".to_string();
        let n2 = replace_token(&mut s2, "color", "color_0", true);
        // Both standalone `color` occurrences are replaced; `color.rgb`'s
        // `color` is standalone (suffix `.`), the `color` in `vec3 color` too.
        assert_eq!(n2, 2);
        assert_eq!(s2, "vec3 color_0 = color_0.rgb;");
    }

    #[test]
    fn test_replace_token_period_allowed_for_channels() {
        let mut s = "texture(image, st).channels".to_string();
        let n = replace_token(&mut s, "channels", "rgb", false);
        assert_eq!(n, 1);
        assert_eq!(s, "texture(image, st).rgb");
    }

    #[test]
    fn test_shader_uniforms_flattened() {
        let m = build(
            r#"{
                "materials": {"base": {"type": "Color"}},
                "components": {"diffuse": "base.diffuse", "alpha": "base.alpha"}
            }"#,
        );
        let flat = m.shader_uniforms();
        // Includes the sub-material's renamed color uniform.
        assert!(flat.values().any(|v| matches!(v, UniformValue::Vec4(_))));
    }
}
