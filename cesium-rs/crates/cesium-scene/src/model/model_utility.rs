//! Ported from `packages/engine/Source/Scene/Model/ModelUtility.js`.
//!
//! Utility functions for model processing.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::matrix3::Matrix3;
use cesium_core::matrix4::Matrix4;
use cesium_core::primitive_type::PrimitiveType;
use cesium_core::quaternion::Quaternion;
use cesium_core::runtime_error::RuntimeError;
use serde_json::Value;

use crate::axis::Axis;
use crate::cull_face::CullFace;
use crate::vertex_attribute_semantic::VertexAttributeSemantic;

/// A lightweight representation of a glTF node attribute used by the
/// utility functions.
///
/// This is a Rust analogue of the `ModelComponents.Attribute` shape
/// consumed by `ModelUtility` helpers.
#[derive(Debug, Clone, Default)]
pub struct ModelAttribute {
    /// The semantic (e.g. "POSITION").
    pub semantic: Option<VertexAttributeSemantic>,
    /// The set index.
    pub set_index: Option<u32>,
    /// The attribute name as it appears in the model file.
    pub name: String,
    /// The attribute type string (e.g. "VEC3").
    pub gl_type: String,
    /// Whether the attribute is quantized.
    pub quantization: Option<ModelAttributeQuantization>,
    /// Minimum component values.
    pub min: Option<Vec<f64>>,
    /// Maximum component values.
    pub max: Option<Vec<f64>>,
}

/// Quantization metadata for a model attribute.
#[derive(Debug, Clone, Default)]
pub struct ModelAttributeQuantization {
    /// The quantized type string.
    pub gl_type: String,
}

/// A lightweight representation of a glTF node used by the utility
/// functions.
///
/// This is a Rust analogue of the `ModelComponents.Node` shape.
#[derive(Debug, Clone, Default)]
pub struct ModelNode {
    /// The full transformation matrix, if present.
    pub matrix: Option<Matrix4>,
    /// The translation component.
    pub translation: Option<Cartesian3>,
    /// The rotation component (quaternion).
    pub rotation: Option<Quaternion>,
    /// The scale component.
    pub scale: Option<Cartesian3>,
}

/// Information about a model attribute, returned by
/// [`ModelUtility::get_attribute_info`].
#[derive(Debug, Clone)]
pub struct AttributeInfo {
    /// Whether the attribute is quantized.
    pub is_quantized: bool,
    /// The GLSL variable name.
    pub variable_name: String,
    /// Whether the attribute has a semantic.
    pub has_semantic: bool,
    /// The GLSL type string.
    pub glsl_type: String,
    /// The quantized GLSL type, if quantized.
    pub quantized_glsl_type: Option<String>,
}

/// Utility functions for model processing.
///
/// All methods are static; the struct exists only as a namespace.
pub struct ModelUtility;

impl ModelUtility {
    /// Creates a new ModelUtility (unit struct; always returns `Self`).
    pub fn new() -> Self { Self }

    /// The glTF extensions supported by the model loader.
    ///
    /// Mirrors `ModelUtility.supportedExtensions`.
    pub const SUPPORTED_EXTENSIONS: &'static [&'static str] = &[
        "AGI_articulations",
        "CESIUM_mesh_vector",
        "CESIUM_primitive_outline",
        "CESIUM_RTC",
        "EXT_feature_metadata",
        "EXT_implicit_cylinder_region",
        "EXT_implicit_ellipsoid_region",
        "EXT_instance_features",
        "EXT_mesh_features",
        "EXT_mesh_gpu_instancing",
        "EXT_mesh_polygon",
        "EXT_mesh_primitive_edge_visibility",
        "EXT_meshopt_compression",
        "EXT_primitive_voxels",
        "EXT_structural_metadata",
        "EXT_texture_webp",
        "KHR_blend",
        "KHR_draco_mesh_compression",
        "KHR_implicit_shapes",
        "KHR_materials_common",
        "KHR_materials_pbrSpecularGlossiness",
        "KHR_materials_specular",
        "KHR_materials_anisotropy",
        "KHR_materials_clearcoat",
        "KHR_materials_unlit",
        "KHR_mesh_quantization",
        "KHR_mesh_primitive_restart",
        "KHR_meshopt_compression",
        "KHR_techniques_webgl",
        "KHR_texture_basisu",
        "KHR_texture_transform",
        "KHR_gaussian_splatting",
        "KHR_gaussian_splatting_compression_spz_2",
        "WEB3D_quantized_attributes",
    ];

    /// Checks whether or not the extensions required by the glTF are
    /// supported.
    ///
    /// Mirrors `ModelUtility.checkSupportedExtensions(extensionsRequired)`.
    ///
    /// # Errors
    /// Returns `RuntimeError("Unsupported glTF Extension: <name>")` for the
    /// first unsupported extension encountered.
    pub fn check_supported_extensions(
        extensions_required: &[String],
    ) -> Result<(), RuntimeError> {
        for extension in extensions_required {
            if !Self::SUPPORTED_EXTENSIONS.contains(&extension.as_str()) {
                return Err(RuntimeError::new(Some(&format!(
                    "Unsupported glTF Extension: {extension}"
                ))));
            }
        }
        Ok(())
    }

    /// Creates a RuntimeError for a failed model load.
    ///
    /// Mirrors `ModelUtility.getError(type, path, error)`.
    pub fn get_error(type_name: &str, path: &str, error: Option<&str>) -> RuntimeError {
        let mut message = format!("Failed to load {type_name}: {path}");
        if let Some(err_msg) = error {
            message.push('\n');
            message.push_str(err_msg);
        }
        RuntimeError::new(Some(&message))
    }

    /// Gets a transformation matrix from a node in the model.
    ///
    /// Mirrors `ModelUtility.getNodeTransform(node)`.
    pub fn get_node_transform(node: &ModelNode) -> Matrix4 {
        if let Some(ref matrix) = node.matrix {
            return *matrix;
        }

        let translation = node
            .translation
            .as_ref()
            .unwrap_or(&Cartesian3::ZERO);
        let rotation = node
            .rotation
            .as_ref()
            .unwrap_or(&Quaternion::IDENTITY);
        let scale = node.scale.as_ref().unwrap_or(&Cartesian3::ONE);

        Matrix4::from_translation_quaternion_rotation_scale_new(translation, rotation, scale)
    }

    /// Finds an attribute by semantic from a list of attributes.
    ///
    /// Mirrors `ModelUtility.getAttributeBySemantic(object, semantic, setIndex)`.
    /// The `attributes` slice contains the attributes to search.
    pub fn get_attribute_by_semantic(
        attributes: &[ModelAttribute],
        semantic: VertexAttributeSemantic,
        set_index: Option<u32>,
    ) -> Option<&ModelAttribute> {
        for attribute in attributes {
            if attribute.semantic == Some(semantic) {
                let matches_set_index = match set_index {
                    Some(idx) => attribute.set_index == Some(idx),
                    None => true,
                };
                if matches_set_index {
                    return Some(attribute);
                }
            }
        }
        None
    }

    /// Finds an attribute by name from a list of attributes.
    ///
    /// Mirrors `ModelUtility.getAttributeByName(object, name)`.
    pub fn get_attribute_by_name<'a>(
        attributes: &'a [ModelAttribute],
        name: &str,
    ) -> Option<&'a ModelAttribute> {
        attributes.iter().find(|a| a.name == name)
    }

    /// Finds a feature ID from a JSON array with `label` or
    /// `positionalLabel` matching the given label.
    ///
    /// Mirrors `ModelUtility.getFeatureIdsByLabel(featureIds, label)`.
    pub fn get_feature_ids_by_label<'a>(
        feature_ids: &'a [Value],
        label: &str,
    ) -> Option<&'a Value> {
        for feature_id_set in feature_ids {
            let matches_positional = feature_id_set
                .get("positionalLabel")
                .and_then(|v| v.as_str())
                == Some(label);
            let matches_label = feature_id_set
                .get("label")
                .and_then(|v| v.as_str())
                == Some(label);
            if matches_positional || matches_label {
                return Some(feature_id_set);
            }
        }
        None
    }

    /// Returns whether any attribute in the slice has quantization.
    ///
    /// Mirrors `ModelUtility.hasQuantizedAttributes(attributes)`.
    pub fn has_quantized_attributes(attributes: &[ModelAttribute]) -> bool {
        attributes.iter().any(|a| a.quantization.is_some())
    }

    /// Gets attribute info (variable name, GLSL type, quantization) for
    /// a model attribute.
    ///
    /// Mirrors `ModelUtility.getAttributeInfo(attribute)`.
    pub fn get_attribute_info(attribute: &ModelAttribute) -> AttributeInfo {
        let (variable_name, has_semantic) = if let Some(semantic) = &attribute.semantic {
            (
                semantic.get_variable_name(attribute.set_index),
                true,
            )
        } else {
            // Custom attributes: strip leading underscore, lowercase.
            let name = attribute.name.strip_prefix('_').unwrap_or(&attribute.name);
            (name.to_lowercase(), false)
        };

        let is_vertex_color = variable_name.starts_with("color_")
            && variable_name["color_".len()..]
                .chars()
                .all(|c| c.is_ascii_digit());

        let glsl_type = if is_vertex_color {
            "vec4".to_string()
        } else {
            attribute.gl_type.clone()
        };

        let quantized_glsl_type = attribute.quantization.as_ref().map(|q| {
            if is_vertex_color {
                "vec4".to_string()
            } else {
                q.gl_type.clone()
            }
        });

        AttributeInfo {
            is_quantized: attribute.quantization.is_some(),
            variable_name,
            has_semantic,
            glsl_type,
            quantized_glsl_type,
        }
    }

    /// Gets the minimum and maximum POSITION values for a primitive.
    ///
    /// Mirrors `ModelUtility.getPositionMinMax(primitive, instancingTranslationMin,
    /// instancingTranslationMax)`.
    pub fn get_position_min_max(
        attributes: &[ModelAttribute],
        instancing_translation_min: Option<&Cartesian3>,
        instancing_translation_max: Option<&Cartesian3>,
    ) -> Option<(Cartesian3, Cartesian3)> {
        let position = Self::get_attribute_by_semantic(
            attributes,
            VertexAttributeSemantic::Position,
            None,
        )?;

        let min_vals = position.min.as_ref()?;
        let max_vals = position.max.as_ref()?;

        if min_vals.len() < 3 || max_vals.len() < 3 {
            return None;
        }

        let mut pos_min =
            Cartesian3::from_elements_new(min_vals[0], min_vals[1], min_vals[2]);
        let mut pos_max =
            Cartesian3::from_elements_new(max_vals[0], max_vals[1], max_vals[2]);

        if let (Some(inst_min), Some(inst_max)) =
            (instancing_translation_min, instancing_translation_max)
        {
            pos_min = Cartesian3::add_new(&pos_min, inst_min);
            pos_max = Cartesian3::add_new(&pos_max, inst_max);
        }

        Some((pos_min, pos_max))
    }

    /// Returns a matrix that corrects the coordinate system so that z is
    /// up and x is forward.
    ///
    /// Mirrors `ModelUtility.getAxisCorrectionMatrix(upAxis, forwardAxis, result)`.
    pub fn get_axis_correction_matrix(up_axis: Axis, forward_axis: Axis) -> Matrix4 {
        let mut result = Matrix4::IDENTITY;

        if up_axis == Axis::Y {
            result = Axis::y_up_to_z_up();
        } else if up_axis == Axis::X {
            result = Axis::x_up_to_z_up();
        }

        if forward_axis == Axis::Z {
            // glTF 2.0 has Z-forward; adapt to X-forward.
            result = Matrix4::multiply_new(&result, &Axis::z_up_to_x_up());
        }

        result
    }

    /// Gets the cull face based on the model matrix determinant and
    /// primitive type.
    ///
    /// Mirrors `ModelUtility.getCullFace(modelMatrix, primitiveType)`.
    pub fn get_cull_face(model_matrix: &Matrix4, primitive_type: PrimitiveType) -> CullFace {
        if !primitive_type.is_triangles() {
            return CullFace::Back;
        }

        let matrix3 = Matrix4::get_matrix3_new(model_matrix);
        if Matrix3::determinant(&matrix3) < 0.0 {
            CullFace::Front
        } else {
            CullFace::Back
        }
    }

    /// Sanitizes an identifier for use in a GLSL shader.
    ///
    /// - Replaces non-alphanumeric sequences with `_`.
    /// - Removes `gl_` prefix (reserved in GLSL).
    /// - Prefixes with `_` if the first character is a digit.
    ///
    /// Mirrors `ModelUtility.sanitizeGlslIdentifier(identifier)`.
    pub fn sanitize_glsl_identifier(identifier: &str) -> String {
        // Replace non-alphanumeric sequences with a single underscore.
        let mut result = String::with_capacity(identifier.len());
        let mut last_was_replacement = false;
        for ch in identifier.chars() {
            if ch.is_ascii_alphanumeric() {
                result.push(ch);
                last_was_replacement = false;
            } else if !last_was_replacement {
                result.push('_');
                last_was_replacement = true;
            }
        }

        // Remove gl_ prefix.
        if let Some(stripped) = result.strip_prefix("gl_") {
            result = stripped.to_string();
        }

        // Prefix with _ if first character is a digit.
        if result.starts_with(|c: char| c.is_ascii_digit()) {
            result.insert(0, '_');
        }

        result
    }
}

impl Default for ModelUtility {
    fn default() -> Self { Self }
}
