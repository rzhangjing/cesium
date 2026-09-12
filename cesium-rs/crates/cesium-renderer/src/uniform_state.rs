//! Ported from `packages/engine/Source/Renderer/UniformState.js`.
//!
//! Manages per-frame uniform buffer updates. In CesiumJS, this is a large (1948 line)
//! file that maintains all the `czm_*` automatic uniforms, computing derived matrices
//! (modelView, modelViewProjection, normal, etc.) from camera/frustum/model state.
//!
//! In the Rust port, this captures the core uniform state with lazy computation
//! of derived matrices via dirty flags.

use cesium_core::bounding_rectangle::BoundingRectangle;
use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::matrix3::Matrix3;
use cesium_core::matrix4::Matrix4;

/// DEVIATION (B3.4): CesiumJS derives the world-space sun direction every frame
/// from the Simon1994PlanetaryPositions ephemeris (`setSunAndMoonDirections`,
/// UniformState.js L1322). That ephemeris is not yet ported (`Sun::update` is a
/// stub), so the uniform state is seeded with this fixed default direction. It
/// keeps the `czm_sunDirectionWC` supply chain (UniformState →
/// AutomaticUniforms → WGSL) live and the globe terminator lit; once the
/// ephemeris lands, [`UniformState::update_sun_direction`] overwrites it each
/// frame. The value matches the direction the hand-written `globe_fs.wgsl`
/// previously hardcoded, so the rendered result is unchanged.
const DEFAULT_SUN_DIRECTION: Cartesian3 = Cartesian3::new(0.55, 0.35, 0.5);

/// The normalized [`DEFAULT_SUN_DIRECTION`].
fn default_sun_direction() -> Cartesian3 {
    Cartesian3::normalize_new(&DEFAULT_SUN_DIRECTION)
}

/// Manages per-frame uniform values and buffer updates.
///
/// Mirrors the CesiumJS `UniformState` which maintains all `czm_*` automatic
/// uniforms. Derived matrices are computed lazily via dirty flags.
pub struct UniformState {
    /// Current frame number.
    frame_number: u64,
    /// The viewport rectangle.
    viewport: BoundingRectangle,
    /// Whether the viewport has changed.
    viewport_dirty: bool,

    // ---- Core matrices ----
    /// Model matrix (object to world).
    model: Matrix4,
    /// View matrix (world to eye).
    view: Matrix4,
    /// Inverse view matrix.
    inverse_view: Matrix4,
    /// Projection matrix.
    projection: Matrix4,
    /// Infinite projection matrix (for skybox, etc.).
    infinite_projection: Matrix4,

    // ---- Derived matrices (lazy computation) ----
    /// Model-view matrix.
    model_view: Matrix4,
    model_view_dirty: bool,
    /// Model-view-projection matrix.
    model_view_projection: Matrix4,
    model_view_projection_dirty: bool,
    /// Inverse model-view-projection matrix.
    inverse_model_view_projection: Matrix4,
    inverse_model_view_projection_dirty: bool,
    /// Inverse model matrix.
    inverse_model: Matrix4,
    inverse_model_dirty: bool,
    /// Inverse transpose model matrix (for normal transformation).
    inverse_transpose_model: Matrix3,
    inverse_transpose_model_dirty: bool,
    /// Inverse projection matrix.
    inverse_projection: Matrix4,
    inverse_projection_dirty: bool,
    /// Normal matrix (inverse transpose of model-view).
    normal: Matrix3,
    normal_dirty: bool,

    // ---- Camera state ----
    /// Camera position in world coordinates.
    camera_position: Cartesian3,
    /// Camera direction.
    camera_direction: Cartesian3,
    /// Camera right vector.
    camera_right: Cartesian3,
    /// Camera up vector.
    camera_up: Cartesian3,

    // ---- Frustum state ----
    /// Near and far frustum distances.
    entire_frustum: Cartesian2,
    /// Current split frustum near/far.
    current_frustum: Cartesian2,

    // ---- Lighting state ----
    /// Sun position in world coordinates.
    sun_position_wc: Cartesian3,
    /// Sun direction in world coordinates.
    sun_direction_wc: Cartesian3,
    /// Sun direction in eye coordinates.
    sun_direction_ec: Cartesian3,
    /// Moon direction in eye coordinates.
    moon_direction_ec: Cartesian3,
    /// Light direction in world coordinates.
    light_direction_wc: Cartesian3,
    /// Light direction in eye coordinates.
    light_direction_ec: Cartesian3,
    /// Light color (RGB).
    light_color: Cartesian3,
    /// Light color HDR (RGB).
    light_color_hdr: Cartesian3,

    // ---- Environment state ----
    /// Fog density.
    fog_density: f32,
    /// Fog minimum brightness.
    fog_minimum_brightness: f32,
    /// Background color.
    background_color: Color,

    // ---- Rendering state ----
    /// Pixel ratio for HiDPI displays.
    pixel_ratio: f32,
    /// Gamma correction value.
    gamma: f32,
    /// Current render pass.
    pass: Option<u32>,
    /// Current scene mode.
    mode: Option<u32>,
    /// The ellipsoid.
    ellipsoid: Ellipsoid,

    // ---- Viewport matrices ----
    /// Viewport orthographic matrix.
    viewport_orthographic_matrix: Matrix4,
    /// Viewport transformation matrix.
    viewport_transformation: Matrix4,
}

impl UniformState {
    /// Creates a new uniform state.
    pub fn new() -> Self {
        Self {
            frame_number: 0,
            viewport: BoundingRectangle::default(),
            viewport_dirty: false,
            model: Matrix4::IDENTITY,
            view: Matrix4::IDENTITY,
            inverse_view: Matrix4::IDENTITY,
            projection: Matrix4::IDENTITY,
            infinite_projection: Matrix4::IDENTITY,
            model_view: Matrix4::IDENTITY,
            model_view_dirty: true,
            model_view_projection: Matrix4::IDENTITY,
            model_view_projection_dirty: true,
            inverse_model_view_projection: Matrix4::IDENTITY,
            inverse_model_view_projection_dirty: true,
            inverse_model: Matrix4::IDENTITY,
            inverse_model_dirty: true,
            inverse_transpose_model: Matrix3::IDENTITY,
            inverse_transpose_model_dirty: true,
            inverse_projection: Matrix4::IDENTITY,
            inverse_projection_dirty: true,
            normal: Matrix3::IDENTITY,
            normal_dirty: true,
            camera_position: Cartesian3::ZERO,
            camera_direction: Cartesian3::ZERO,
            camera_right: Cartesian3::ZERO,
            camera_up: Cartesian3::ZERO,
            entire_frustum: Cartesian2::ZERO,
            current_frustum: Cartesian2::ZERO,
            sun_position_wc: Cartesian3::ZERO,
            // DEVIATION (B3.4): seeded with a fixed default sun direction (see
            // `DEFAULT_SUN_DIRECTION`) because the ephemeris CesiumJS uses to
            // derive it per frame is not yet ported. The view is identity here,
            // so the eye-space direction equals the world-space one.
            sun_direction_wc: default_sun_direction(),
            sun_direction_ec: default_sun_direction(),
            moon_direction_ec: Cartesian3::ZERO,
            light_direction_wc: default_sun_direction(),
            light_direction_ec: default_sun_direction(),
            // The default scene light is a SunLight (color WHITE, intensity
            // 2.0): czm_lightColor is clamped to a maximum luminance of 1.0 →
            // white; czm_lightColorHdr is color × intensity → (2, 2, 2).
            light_color: Cartesian3::new(1.0, 1.0, 1.0),
            light_color_hdr: Cartesian3::new(2.0, 2.0, 2.0),
            fog_density: 0.0,
            fog_minimum_brightness: 0.25,
            background_color: Color::default(),
            pixel_ratio: 1.0,
            gamma: 2.2,
            pass: None,
            mode: None,
            ellipsoid: Ellipsoid::WGS84,
            viewport_orthographic_matrix: Matrix4::IDENTITY,
            viewport_transformation: Matrix4::IDENTITY,
        }
    }

    /// Returns the current frame number.
    pub fn frame_number(&self) -> u64 {
        self.frame_number
    }

    /// Advances to the next frame.
    pub fn next_frame(&mut self) {
        self.frame_number += 1;
    }

    // ---- Update methods ----

    /// Updates the model matrix.
    pub fn update_model(&mut self, model: Matrix4) {
        self.model = model;
        self.model_view_dirty = true;
        self.model_view_projection_dirty = true;
        self.inverse_model_dirty = true;
        self.inverse_transpose_model_dirty = true;
        self.normal_dirty = true;
    }

    /// Updates the view matrix.
    pub fn update_view(&mut self, view: Matrix4) {
        self.view = view;
        self.model_view_dirty = true;
        self.model_view_projection_dirty = true;
        self.normal_dirty = true;
    }

    /// Updates the projection matrix.
    pub fn update_projection(&mut self, projection: Matrix4) {
        self.projection = projection;
        self.model_view_projection_dirty = true;
        self.inverse_projection_dirty = true;
    }

    /// Updates the viewport.
    pub fn update_viewport(&mut self, viewport: BoundingRectangle) {
        self.viewport = viewport;
        self.viewport_dirty = true;
    }

    /// Updates the camera position.
    pub fn update_camera_position(&mut self, position: Cartesian3) {
        self.camera_position = position;
    }

    /// Updates the frustum near/far.
    pub fn update_frustum(&mut self, near: f64, far: f64) {
        self.entire_frustum = Cartesian2::new(near, far);
        self.current_frustum = Cartesian2::new(near, far);
    }

    /// Updates the current render pass.
    pub fn update_pass(&mut self, pass: u32) {
        self.pass = Some(pass);
    }

    // ---- Lazy matrix accessors ----

    /// Returns the model-view matrix, computing it if dirty.
    ///
    /// Mirrors CesiumJS `cleanModelView`: modelView = view × model
    /// (`Matrix4.multiplyTransformation(this._view, this._model, ...)`).
    pub fn model_view(&mut self) -> &Matrix4 {
        if self.model_view_dirty {
            self.model_view = Matrix4::multiply_new(&self.view, &self.model);
            self.model_view_dirty = false;
        }
        &self.model_view
    }

    /// Returns the model-view-projection matrix, computing it if dirty.
    pub fn model_view_projection(&mut self) -> &Matrix4 {
        if self.model_view_projection_dirty {
            let mv = self.model_view().clone();
            self.model_view_projection = Matrix4::multiply_new(&self.projection, &mv);
            self.model_view_projection_dirty = false;
        }
        &self.model_view_projection
    }

    /// Returns the inverse model-view-projection matrix, computing it if dirty.
    pub fn inverse_model_view_projection(&mut self) -> &Matrix4 {
        if self.inverse_model_view_projection_dirty {
            let mvp = self.model_view_projection().clone();
            self.inverse_model_view_projection = Matrix4::inverse_new(&mvp).unwrap_or(Matrix4::IDENTITY);
            self.inverse_model_view_projection_dirty = false;
        }
        &self.inverse_model_view_projection
    }

    /// Returns the inverse model matrix, computing it if dirty.
    pub fn inverse_model(&mut self) -> &Matrix4 {
        if self.inverse_model_dirty {
            self.inverse_model = Matrix4::inverse_new(&self.model).unwrap_or(Matrix4::IDENTITY);
            self.inverse_model_dirty = false;
        }
        &self.inverse_model
    }

    /// Returns the normal matrix (inverse transpose of model-view), computing it if dirty.
    pub fn normal(&mut self) -> &Matrix3 {
        if self.normal_dirty {
            // Normal matrix is the inverse transpose of the upper-left 3x3 of model-view
            let mv = self.model_view().clone();
            // Extract upper-left 3x3 from Matrix4
            let m3 = Matrix4::get_matrix3_new(&mv);
            self.normal = Matrix3::inverse_new(&m3).unwrap_or(Matrix3::IDENTITY);
            self.normal = Matrix3::transpose_new(&self.normal);
            self.normal_dirty = false;
        }
        &self.normal
    }

    // ---- Uniform value accessors ----

    /// Returns the model matrix.
    pub fn model(&self) -> &Matrix4 { &self.model }

    /// Returns the view matrix.
    pub fn view(&self) -> &Matrix4 { &self.view }

    /// Returns the projection matrix.
    pub fn projection(&self) -> &Matrix4 { &self.projection }

    /// Returns the viewport.
    pub fn viewport(&self) -> &BoundingRectangle { &self.viewport }

    /// Returns the camera position.
    pub fn camera_position(&self) -> &Cartesian3 { &self.camera_position }

    /// Returns the pixel ratio.
    pub fn pixel_ratio(&self) -> f32 { self.pixel_ratio }

    /// Sets the pixel ratio.
    pub fn set_pixel_ratio(&mut self, ratio: f32) {
        self.pixel_ratio = ratio;
    }

    /// Returns the gamma value.
    pub fn gamma(&self) -> f32 { self.gamma }

    /// Sets the gamma value.
    pub fn set_gamma(&mut self, gamma: f32) {
        self.gamma = gamma;
    }

    /// Returns the fog density.
    pub fn fog_density(&self) -> f32 { self.fog_density }

    /// Sets the fog density.
    pub fn set_fog_density(&mut self, density: f32) {
        self.fog_density = density;
    }

    /// Returns the background color.
    pub fn background_color(&self) -> &Color { &self.background_color }

    /// Sets the background color.
    pub fn set_background_color(&mut self, color: Color) {
        self.background_color = color;
    }

    /// Returns the ellipsoid.
    pub fn ellipsoid(&self) -> &Ellipsoid { &self.ellipsoid }

    /// Sets the ellipsoid.
    pub fn set_ellipsoid(&mut self, ellipsoid: Ellipsoid) {
        self.ellipsoid = ellipsoid;
    }

    // ---- Inverse / infinite matrix accessors (B3.4) ----

    /// Returns the inverse view matrix (eye → world). Backs `czm_inverseView`
    /// and `czm_viewerPositionWC`.
    pub fn inverse_view(&self) -> &Matrix4 { &self.inverse_view }

    /// Updates the inverse view matrix.
    ///
    /// Mirrors CesiumJS `setInverseView` (called from `updateCamera`).
    pub fn update_inverse_view(&mut self, inverse_view: Matrix4) {
        self.inverse_view = inverse_view;
    }

    /// Returns the infinite projection matrix. Backs `czm_infiniteProjection`.
    pub fn infinite_projection(&self) -> &Matrix4 { &self.infinite_projection }

    /// Updates the infinite projection matrix.
    pub fn update_infinite_projection(&mut self, infinite_projection: Matrix4) {
        self.infinite_projection = infinite_projection;
    }

    /// Returns the inverse projection matrix, computing it if dirty.
    /// Backs `czm_inverseProjection`.
    pub fn inverse_projection(&mut self) -> &Matrix4 {
        if self.inverse_projection_dirty {
            self.inverse_projection =
                Matrix4::inverse_new(&self.projection).unwrap_or(Matrix4::IDENTITY);
            self.inverse_projection_dirty = false;
        }
        &self.inverse_projection
    }

    /// Returns the inverse-transpose of the model matrix's upper-left 3×3,
    /// computing it if dirty (world-space normal transform).
    pub fn inverse_transpose_model(&mut self) -> &Matrix3 {
        if self.inverse_transpose_model_dirty {
            let m3 = Matrix4::get_matrix3_new(&self.model);
            let inv = Matrix3::inverse_new(&m3).unwrap_or(Matrix3::IDENTITY);
            self.inverse_transpose_model = Matrix3::transpose_new(&inv);
            self.inverse_transpose_model_dirty = false;
        }
        &self.inverse_transpose_model
    }

    // ---- Camera orientation accessors (B3.4) ----

    /// Returns the camera direction.
    pub fn camera_direction(&self) -> &Cartesian3 { &self.camera_direction }

    /// Returns the camera right vector.
    pub fn camera_right(&self) -> &Cartesian3 { &self.camera_right }

    /// Returns the camera up vector.
    pub fn camera_up(&self) -> &Cartesian3 { &self.camera_up }

    /// Updates the camera orientation (direction/right/up), mirroring the
    /// orientation half of CesiumJS `setCamera`.
    pub fn update_camera_orientation(
        &mut self,
        direction: Cartesian3,
        right: Cartesian3,
        up: Cartesian3,
    ) {
        self.camera_direction = direction;
        self.camera_right = right;
        self.camera_up = up;
    }

    /// Returns the viewer position in world coordinates, derived from the
    /// inverse view matrix translation.
    ///
    /// Mirrors `czm_viewerPositionWC`'s `getValue`
    /// (`Matrix4.getTranslation(uniformState.inverseView, ...)`).
    pub fn viewer_position_wc(&self) -> Cartesian3 {
        Matrix4::get_translation_new(&self.inverse_view)
    }

    // ---- Frustum accessors (B3.4) ----

    /// Returns the entire (largest possible) frustum near/far.
    /// Backs `czm_entireFrustum`.
    pub fn entire_frustum(&self) -> &Cartesian2 { &self.entire_frustum }

    /// Returns the current (multi-frustum split) near/far.
    /// Backs `czm_currentFrustum`.
    pub fn current_frustum(&self) -> &Cartesian2 { &self.current_frustum }

    // ---- Lighting accessors / updaters (B3.4) ----

    /// Returns the sun position in world coordinates. Backs `czm_sunPositionWC`.
    pub fn sun_position_wc(&self) -> &Cartesian3 { &self.sun_position_wc }

    /// Returns the normalized sun direction in world coordinates.
    /// Backs `czm_sunDirectionWC`.
    pub fn sun_direction_wc(&self) -> &Cartesian3 { &self.sun_direction_wc }

    /// Returns the normalized sun direction in eye coordinates.
    /// Backs `czm_sunDirectionEC`.
    pub fn sun_direction_ec(&self) -> &Cartesian3 { &self.sun_direction_ec }

    /// Returns the normalized moon direction in eye coordinates.
    /// Backs `czm_moonDirectionEC`.
    pub fn moon_direction_ec(&self) -> &Cartesian3 { &self.moon_direction_ec }

    /// Returns the normalized light direction in world coordinates.
    /// Backs `czm_lightDirectionWC`.
    pub fn light_direction_wc(&self) -> &Cartesian3 { &self.light_direction_wc }

    /// Returns the normalized light direction in eye coordinates.
    /// Backs `czm_lightDirectionEC`.
    pub fn light_direction_ec(&self) -> &Cartesian3 { &self.light_direction_ec }

    /// Returns the (LDR) light color. Backs `czm_lightColor`.
    pub fn light_color(&self) -> &Cartesian3 { &self.light_color }

    /// Returns the HDR light color. Backs `czm_lightColorHdr`.
    pub fn light_color_hdr(&self) -> &Cartesian3 { &self.light_color_hdr }

    /// Derives the sun/light directions from the sun position in world
    /// coordinates.
    ///
    /// Mirrors CesiumJS `setSunAndMoonDirections` (UniformState.js L1322):
    /// `sunDirectionWC = normalize(sunPositionWC)`, then
    /// `sunDirectionEC = normalize(viewRotation3D × sunDirectionWC)`. The
    /// default scene light is a SunLight, so the light directions mirror the
    /// sun directions (L1455-1463).
    ///
    /// DEVIATION (B3.4): CesiumJS computes `sunPositionWC` from the
    /// Simon1994PlanetaryPositions ephemeris and uses `viewRotation3D` (the
    /// 3D-equivalent view rotation in 2D/Columbus View). The port takes the sun
    /// position as a parameter (the ephemeris is not yet ported) and uses the
    /// current view rotation, which is identical in 3D.
    pub fn update_sun_direction(&mut self, sun_position_wc: Cartesian3) {
        self.sun_position_wc = sun_position_wc;
        self.sun_direction_wc = Cartesian3::normalize_new(&sun_position_wc);
        let view_rotation = Matrix4::get_matrix3_new(&self.view);
        let ec = Matrix3::multiply_by_vector_new(&view_rotation, &self.sun_direction_wc);
        self.sun_direction_ec = Cartesian3::normalize_new(&ec);
        self.light_direction_wc = self.sun_direction_wc;
        self.light_direction_ec = self.sun_direction_ec;
    }

    /// Sets the moon direction in eye coordinates. Backs `czm_moonDirectionEC`.
    pub fn set_moon_direction_ec(&mut self, moon_direction_ec: Cartesian3) {
        self.moon_direction_ec = moon_direction_ec;
    }

    /// Sets the LDR and HDR light colors. Backs `czm_lightColor` /
    /// `czm_lightColorHdr`.
    pub fn set_light_color(&mut self, light_color: Cartesian3, light_color_hdr: Cartesian3) {
        self.light_color = light_color;
        self.light_color_hdr = light_color_hdr;
    }

    // ---- Environment / render-state accessors (B3.4) ----

    /// Returns the fog minimum brightness. Backs `czm_fogMinimumBrightness`.
    pub fn fog_minimum_brightness(&self) -> f32 { self.fog_minimum_brightness }

    /// Sets the fog minimum brightness.
    pub fn set_fog_minimum_brightness(&mut self, fog_minimum_brightness: f32) {
        self.fog_minimum_brightness = fog_minimum_brightness;
    }

    /// Returns the current render pass. Backs `czm_pass`.
    pub fn pass(&self) -> Option<u32> { self.pass }

    /// Returns the current scene mode. Backs `czm_sceneMode`.
    pub fn mode(&self) -> Option<u32> { self.mode }

    /// Updates the current scene mode.
    pub fn update_mode(&mut self, mode: u32) {
        self.mode = Some(mode);
    }
}

impl Default for UniformState {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_core::cartesian3::Cartesian3;
    use cesium_core::matrix4::Matrix4;

    #[test]
    fn default_sun_direction_is_normalized_and_mirrored_by_light() {
        let state = UniformState::new();
        let dir = *state.sun_direction_wc();
        let mag = Cartesian3::magnitude(&dir);
        assert!((mag - 1.0).abs() < 1e-9, "sun dir not normalized: {mag}");
        // The default scene light is a SunLight → light mirrors the sun.
        assert_eq!(state.light_direction_wc(), state.sun_direction_wc());
        assert_eq!(state.light_direction_ec(), state.sun_direction_ec());
    }

    #[test]
    fn update_sun_direction_derives_wc_and_ec() {
        let mut state = UniformState::new();
        // With an identity view, the eye-space direction equals world-space.
        state.update_sun_direction(Cartesian3::new(0.0, 0.0, 10.0));
        let wc = *state.sun_direction_wc();
        assert!((wc.z - 1.0).abs() < 1e-9);
        assert!(wc.x.abs() < 1e-9);
        let ec = *state.sun_direction_ec();
        assert!((ec.z - 1.0).abs() < 1e-9);
        assert_eq!(state.light_direction_wc(), state.sun_direction_wc());
    }

    #[test]
    fn inverse_view_and_viewer_position_round_trip() {
        let mut state = UniformState::new();
        let mut inv = Matrix4::IDENTITY;
        inv.elements[12] = 3.0;
        inv.elements[13] = 4.0;
        inv.elements[14] = 5.0;
        state.update_inverse_view(inv);
        assert_eq!(state.viewer_position_wc(), Cartesian3::new(3.0, 4.0, 5.0));
    }

    #[test]
    fn inverse_projection_is_lazy_inverse_of_projection() {
        let mut state = UniformState::new();
        let mut proj = Matrix4::IDENTITY;
        proj.elements[0] = 2.0;
        state.update_projection(proj);
        let inv = state.inverse_projection().clone();
        assert!((inv.elements[0] - 0.5).abs() < 1e-9);
    }

    #[test]
    fn update_camera_orientation_stores_vectors() {
        let mut state = UniformState::new();
        state.update_camera_orientation(
            Cartesian3::UNIT_Z,
            Cartesian3::UNIT_X,
            Cartesian3::UNIT_Y,
        );
        assert_eq!(*state.camera_direction(), Cartesian3::UNIT_Z);
        assert_eq!(*state.camera_right(), Cartesian3::UNIT_X);
        assert_eq!(*state.camera_up(), Cartesian3::UNIT_Y);
    }
}
