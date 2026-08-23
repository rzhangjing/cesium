//! glTF pipeline subsystem 鈥?glTF processing and optimization.
//! Ported from `packages/engine/Source/Scene/GltfPipeline/`.

pub mod add_buffer;
pub mod add_defaults;
pub mod add_extensions_required;
pub mod add_extensions_used;
pub mod add_pipeline_extras;
pub mod add_to_array;
pub mod find_accessor_min_max;
pub mod for_each;
pub mod for_each_texture_in_material;
pub mod get_accessor_byte_stride;
pub mod get_component_reader;
pub mod move_technique_render_states;
pub mod move_techniques_to_extension;
pub mod number_of_components_for_type;
pub mod parse_glb;
pub mod read_accessor_packed;
pub mod remove_extension;
pub mod remove_extensions_required;
pub mod remove_extensions_used;
pub mod remove_pipeline_extras;
pub mod remove_unused_elements;
pub mod update_accessor_component_types;
pub mod update_version;
pub mod uses_extension;

