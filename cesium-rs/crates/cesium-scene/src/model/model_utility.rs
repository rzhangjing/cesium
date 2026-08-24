//! Ported from `packages/engine/Source/Scene/Model/ModelUtility.js`.
//!
//! Utility functions for model processing.

use cesium_core::runtime_error::RuntimeError;

/// Utility functions for model processing.
pub struct ModelUtility {
    _private: (),
}

impl ModelUtility {
    /// Creates a new ModelUtility.
    pub fn new() -> Self { Self { _private: () } }

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
    /// supported. If an unsupported extension is found, this returns a
    /// [`RuntimeError`] with the extension name.
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
}

impl Default for ModelUtility {
    fn default() -> Self { Self::new() }
}
