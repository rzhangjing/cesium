//! Ported from `packages/engine/Source/Scene/EllipsoidSurfaceAppearance.js`.

use cesium_core::vertex_format::VertexFormat;

use crate::appearance::RenderState;
use crate::material::Material;

/// Options for creating an [`EllipsoidSurfaceAppearance`].
pub struct EllipsoidSurfaceAppearanceOptions {
    /// When `true`, flat shading is used (no lighting). Default: `false`.
    pub flat: Option<bool>,
    /// When `true`, the fragment shader flips the surface normal as
    /// needed to face the viewer. Default: `above_ground`.
    pub face_forward: Option<bool>,
    /// When `true`, the geometry appears translucent. Default: `true`.
    pub translucent: Option<bool>,
    /// When `true`, the geometry is on the ellipsoid surface (backface
    /// culling enabled). Default: `false`.
    pub above_ground: Option<bool>,
    /// The material name used to determine the fragment color.
    pub material: Option<Material>,
    /// Optional GLSL vertex shader source override.
    pub vertex_shader_source: Option<String>,
    /// Optional GLSL fragment shader source override.
    pub fragment_shader_source: Option<String>,
}

impl Default for EllipsoidSurfaceAppearanceOptions {
    fn default() -> Self {
        Self {
            flat: None,
            face_forward: None,
            translucent: None,
            above_ground: None,
            material: None,
            vertex_shader_source: None,
            fragment_shader_source: None,
        }
    }
}

/// An appearance for geometry on the surface of the ellipsoid like
/// `PolygonGeometry` and `RectangleGeometry`.
///
/// This appearance supports all materials and requires fewer vertex
/// attributes since the fragment shader can procedurally compute
/// `normal`, `tangent`, and `bitangent`.
///
/// Port of `EllipsoidSurfaceAppearance`.
pub struct EllipsoidSurfaceAppearance {
    /// The material used to determine the fragment color.
    pub material: Material,
    /// When `true`, the geometry is expected to appear translucent.
    pub translucent: bool,
    /// When `true`, flat shading is used.
    pub flat: bool,
    /// When `true`, the fragment shader flips the surface normal.
    pub face_forward: bool,
    /// When `true`, the geometry is on the ellipsoid's surface.
    pub above_ground: bool,
    /// The GLSL vertex shader source.
    pub vertex_shader_source: String,
    /// The GLSL fragment shader source.
    pub fragment_shader_source: String,
    /// The render state for this appearance.
    pub render_state: RenderState,
    /// Always `false` for ellipsoid surface appearances.
    pub closed: bool,
}

/// The default vertex shader source (placeholder — the real GLSL is
/// embedded via the shader pipeline).
const DEFAULT_VERTEX_SHADER_SOURCE: &str =
    "ellipsoidSurfaceAppearanceVS";

/// The default fragment shader source (placeholder — the real GLSL is
/// embedded via the shader pipeline).
const DEFAULT_FRAGMENT_SHADER_SOURCE: &str =
    "ellipsoidSurfaceAppearanceFS";

impl EllipsoidSurfaceAppearance {
    /// The [`VertexFormat`] that all `EllipsoidSurfaceAppearance`
    /// instances are compatible with, requiring only `position` and
    /// `st` attributes.
    ///
    /// Port of `EllipsoidSurfaceAppearance.VERTEX_FORMAT`.
    pub const VERTEX_FORMAT: VertexFormat = VertexFormat {
        position: true,
        normal: false,
        st: true,
        bitangent: false,
        tangent: false,
        color: false,
    };

    /// Creates a new `EllipsoidSurfaceAppearance`.
    pub fn new(options: Option<EllipsoidSurfaceAppearanceOptions>) -> Self {
        let opts = options.unwrap_or_default();
        let translucent = opts.translucent.unwrap_or(true);
        let above_ground = opts.above_ground.unwrap_or(false);

        let material = opts.material.unwrap_or_else(|| {
            Material::new("Color")
        });

        // Mirror `Appearance.getDefaultRenderState(translucent, !aboveGround)`.
        let render_state = RenderState {
            depth_test: true,
            depth_mask: true,
            blending: translucent,
        };

        Self {
            material,
            translucent,
            flat: opts.flat.unwrap_or(false),
            face_forward: opts.face_forward.unwrap_or(above_ground),
            above_ground,
            vertex_shader_source: opts
                .vertex_shader_source
                .unwrap_or_else(|| DEFAULT_VERTEX_SHADER_SOURCE.to_string()),
            fragment_shader_source: opts
                .fragment_shader_source
                .unwrap_or_else(|| DEFAULT_FRAGMENT_SHADER_SOURCE.to_string()),
            render_state,
            closed: false,
        }
    }

    /// Returns the [`VertexFormat`] that this appearance is compatible
    /// with.
    pub fn vertex_format(&self) -> VertexFormat {
        Self::VERTEX_FORMAT.clone()
    }

    /// Returns `true` if the appearance is translucent.
    pub fn is_translucent(&self) -> bool {
        self.translucent
    }
}

impl Default for EllipsoidSurfaceAppearance {
    fn default() -> Self {
        Self::new(None)
    }
}
