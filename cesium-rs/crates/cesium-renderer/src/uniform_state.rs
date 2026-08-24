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
use cesium_core::cartesian4::Cartesian4;
use cesium_core::color::Color;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::matrix3::Matrix3;
use cesium_core::matrix4::Matrix4;

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
            sun_direction_wc: Cartesian3::ZERO,
            sun_direction_ec: Cartesian3::ZERO,
            moon_direction_ec: Cartesian3::ZERO,
            light_direction_wc: Cartesian3::ZERO,
            light_direction_ec: Cartesian3::ZERO,
            light_color: Cartesian3::ZERO,
            light_color_hdr: Cartesian3::ZERO,
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
}

impl Default for UniformState {
    fn default() -> Self { Self::new() }
}
