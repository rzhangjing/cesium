//! Ported from `packages/engine/Source/Scene/Model/GltfLoaderUtil.js`.
//!
//! glTF loader utilities: accessor component/type helpers shared by the
//! vertex/index buffer loaders and the GPU resource creation path.

use crate::gltf_loader::GltfAccessor;

/// glTF accessor component types (WebGL constants used by the schema).
pub mod component_type {
    /// `BYTE` (signed 8-bit integer).
    pub const BYTE: u32 = 5120;
    /// `UNSIGNED_BYTE` (unsigned 8-bit integer).
    pub const UNSIGNED_BYTE: u32 = 5121;
    /// `SHORT` (signed 16-bit integer).
    pub const SHORT: u32 = 5122;
    /// `UNSIGNED_SHORT` (unsigned 16-bit integer).
    pub const UNSIGNED_SHORT: u32 = 5123;
    /// `UNSIGNED_INT` (unsigned 32-bit integer).
    pub const UNSIGNED_INT: u32 = 5125;
    /// `FLOAT` (32-bit floating point).
    pub const FLOAT: u32 = 5126;
}

/// glTF loader utilities.
pub struct GltfLoaderUtil {
    _private: (),
}

impl GltfLoaderUtil {
    /// Creates a new GltfLoaderUtil.
    pub fn new() -> Self { Self { _private: () } }

    /// The number of components of a glTF accessor type string
    /// (`SCALAR`=1, `VEC2`=2, `VEC3`=3, `VEC4`=4, `MAT2`=4, `MAT3`=9,
    /// `MAT4`=16). Returns 0 for unknown types.
    ///
    /// Mirrors `numberOfComponentsForType`.
    pub fn number_of_components_for_type(gl_type: &str) -> u32 {
        match gl_type {
            "SCALAR" => 1,
            "VEC2" => 2,
            "VEC3" => 3,
            "VEC4" => 4,
            "MAT2" => 4,
            "MAT3" => 9,
            "MAT4" => 16,
            _ => 0,
        }
    }

    /// The size in bytes of one component of the given glTF component
    /// type. Returns 0 for unknown types.
    ///
    /// Mirrors `sizeOfComponentType`.
    pub fn size_of_component_type(component_type_value: u32) -> u32 {
        match component_type_value {
            component_type::BYTE | component_type::UNSIGNED_BYTE => 1,
            component_type::SHORT | component_type::UNSIGNED_SHORT => 2,
            component_type::UNSIGNED_INT | component_type::FLOAT => 4,
            _ => 0,
        }
    }

    /// The byte stride of one element of the accessor (components ×
    /// component size), i.e. the tightly-packed stride used when the
    /// buffer view does not declare `byteStride`.
    pub fn accessor_element_stride(accessor: &GltfAccessor) -> u32 {
        Self::number_of_components_for_type(&accessor.gl_type)
            * Self::size_of_component_type(accessor.component_type)
    }

    /// Maps a glTF accessor (component type × element type × normalized)
    /// to the `wgpu::VertexFormat` consumed by the vertex layout.
    ///
    /// Rust analogue of the attribute-format decisions spread across the
    /// CesiumJS `GeometryPipeline`/`createAttributes` paths. Matrix types
    /// and unrepresentable packed combinations (e.g. 3-component
    /// normalized 8-bit) return `None`; the caller treats them as
    /// unsupported for GPU upload (a logged deviation at the call site).
    pub fn vertex_format(accessor: &GltfAccessor) -> Option<wgpu::VertexFormat> {
        use component_type as ct;
        use wgpu::VertexFormat as Vf;
        let normalized = accessor.normalized;
        match (accessor.component_type, accessor.gl_type.as_str()) {
            (ct::FLOAT, "SCALAR") => Some(Vf::Float32),
            (ct::FLOAT, "VEC2") => Some(Vf::Float32x2),
            (ct::FLOAT, "VEC3") => Some(Vf::Float32x3),
            (ct::FLOAT, "VEC4") => Some(Vf::Float32x4),
            (ct::BYTE, "SCALAR") => Some(if normalized { Vf::Snorm8 } else { Vf::Sint8 }),
            (ct::BYTE, "VEC2") => Some(if normalized { Vf::Snorm8x2 } else { Vf::Sint8x2 }),
            (ct::BYTE, "VEC4") => Some(if normalized { Vf::Snorm8x4 } else { Vf::Sint8x4 }),
            (ct::UNSIGNED_BYTE, "SCALAR") => Some(if normalized { Vf::Unorm8 } else { Vf::Uint8 }),
            (ct::UNSIGNED_BYTE, "VEC2") => {
                Some(if normalized { Vf::Unorm8x2 } else { Vf::Uint8x2 })
            }
            (ct::UNSIGNED_BYTE, "VEC4") => {
                Some(if normalized { Vf::Unorm8x4 } else { Vf::Uint8x4 })
            }
            (ct::SHORT, "SCALAR") => Some(if normalized { Vf::Snorm16 } else { Vf::Sint16 }),
            (ct::SHORT, "VEC2") => Some(if normalized { Vf::Snorm16x2 } else { Vf::Sint16x2 }),
            (ct::SHORT, "VEC4") => Some(if normalized { Vf::Snorm16x4 } else { Vf::Sint16x4 }),
            (ct::UNSIGNED_SHORT, "SCALAR") => {
                Some(if normalized { Vf::Unorm16 } else { Vf::Uint16 })
            }
            (ct::UNSIGNED_SHORT, "VEC2") => {
                Some(if normalized { Vf::Unorm16x2 } else { Vf::Uint16x2 })
            }
            (ct::UNSIGNED_SHORT, "VEC4") => {
                Some(if normalized { Vf::Unorm16x4 } else { Vf::Uint16x4 })
            }
            (ct::UNSIGNED_INT, "SCALAR") => Some(Vf::Uint32),
            (ct::UNSIGNED_INT, "VEC2") => Some(Vf::Uint32x2),
            (ct::UNSIGNED_INT, "VEC4") => Some(Vf::Uint32x4),
            // MAT types and packed-x3 variants have no wgpu equivalent
            // in a single attribute slot.
            _ => None,
        }
    }
}

impl Default for GltfLoaderUtil {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gltf_loader::GltfAccessor;

    fn accessor(component_type_value: u32, gl_type: &str, normalized: bool) -> GltfAccessor {
        GltfAccessor {
            component_type: component_type_value,
            gl_type: gl_type.to_string(),
            normalized,
            count: 1,
            ..Default::default()
        }
    }

    #[test]
    fn number_of_components_for_type_matches_schema() {
        assert_eq!(GltfLoaderUtil::number_of_components_for_type("SCALAR"), 1);
        assert_eq!(GltfLoaderUtil::number_of_components_for_type("VEC2"), 2);
        assert_eq!(GltfLoaderUtil::number_of_components_for_type("VEC3"), 3);
        assert_eq!(GltfLoaderUtil::number_of_components_for_type("VEC4"), 4);
        assert_eq!(GltfLoaderUtil::number_of_components_for_type("MAT2"), 4);
        assert_eq!(GltfLoaderUtil::number_of_components_for_type("MAT3"), 9);
        assert_eq!(GltfLoaderUtil::number_of_components_for_type("MAT4"), 16);
        assert_eq!(GltfLoaderUtil::number_of_components_for_type("BOGUS"), 0);
    }

    #[test]
    fn size_of_component_type_matches_webgl_constants() {
        assert_eq!(GltfLoaderUtil::size_of_component_type(5120), 1);
        assert_eq!(GltfLoaderUtil::size_of_component_type(5121), 1);
        assert_eq!(GltfLoaderUtil::size_of_component_type(5122), 2);
        assert_eq!(GltfLoaderUtil::size_of_component_type(5123), 2);
        assert_eq!(GltfLoaderUtil::size_of_component_type(5125), 4);
        assert_eq!(GltfLoaderUtil::size_of_component_type(5126), 4);
        assert_eq!(GltfLoaderUtil::size_of_component_type(0), 0);
    }

    #[test]
    fn accessor_element_stride_is_components_times_size() {
        // VEC3 FLOAT position: 3 * 4 = 12 bytes.
        assert_eq!(
            GltfLoaderUtil::accessor_element_stride(&accessor(5126, "VEC3", false)),
            12
        );
        // SCALAR UNSIGNED_SHORT index: 2 bytes.
        assert_eq!(
            GltfLoaderUtil::accessor_element_stride(&accessor(5123, "SCALAR", false)),
            2
        );
    }

    #[test]
    fn vertex_format_maps_common_attribute_types() {
        assert_eq!(
            GltfLoaderUtil::vertex_format(&accessor(5126, "VEC3", false)),
            Some(wgpu::VertexFormat::Float32x3)
        );
        assert_eq!(
            GltfLoaderUtil::vertex_format(&accessor(5126, "VEC2", false)),
            Some(wgpu::VertexFormat::Float32x2)
        );
        assert_eq!(
            GltfLoaderUtil::vertex_format(&accessor(5121, "VEC4", true)),
            Some(wgpu::VertexFormat::Unorm8x4)
        );
        assert_eq!(
            GltfLoaderUtil::vertex_format(&accessor(5121, "VEC4", false)),
            Some(wgpu::VertexFormat::Uint8x4)
        );
        assert_eq!(
            GltfLoaderUtil::vertex_format(&accessor(5122, "VEC2", true)),
            Some(wgpu::VertexFormat::Snorm16x2)
        );
    }

    #[test]
    fn vertex_format_is_none_for_unrepresentable_types() {
        // MAT4 needs four attribute slots.
        assert_eq!(GltfLoaderUtil::vertex_format(&accessor(5126, "MAT4", false)), None);
        // 3-component packed normalized types have no wgpu equivalent.
        assert_eq!(GltfLoaderUtil::vertex_format(&accessor(5121, "VEC3", true)), None);
    }
}
