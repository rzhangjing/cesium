//! Ported from `packages/engine/Source/Scene/Scene.js`.
//!
//! The main 3D scene containing the globe, primitives, and camera.

use cesium_core::color::Color;
use cesium_core::julian_date::JulianDate;
use crate::camera::Camera;
use crate::credit_display::CreditDisplay;
use crate::frame_state::FrameState;
use crate::globe::Globe;
use crate::scene_mode::SceneMode;

/// The main 3D scene.
///
/// Contains the globe, camera, primitives, and manages the render loop.
/// This is the largest single module in CesiumJS (172KB).
pub struct Scene {
    camera: Camera,
    globe: Option<Globe>,
    frame_state: FrameState,
    credit_display: CreditDisplay,
    mode: SceneMode,
    morph_time: f64,
    background_color: Color,
    debug_show_frames_per_second: bool,
    is_destroyed: bool,
}

impl Scene {
    /// Creates a new scene.
    pub fn new() -> Self {
        Self {
            camera: Camera::default(),
            globe: None,
            frame_state: FrameState::default(),
            credit_display: CreditDisplay::default(),
            mode: SceneMode::Scene3D,
            morph_time: 1.0,
            background_color: Color::new(0.0, 0.0, 0.0, 1.0),
            debug_show_frames_per_second: false,
            is_destroyed: false,
        }
    }

    /// Returns the camera.
    pub fn camera(&self) -> &Camera { &self.camera }

    /// Returns a mutable reference to the camera.
    pub fn camera_mut(&mut self) -> &mut Camera { &mut self.camera }

    /// Returns the globe, if any.
    pub fn globe(&self) -> Option<&Globe> { self.globe.as_ref() }

    /// Sets the globe.
    pub fn set_globe(&mut self, globe: Option<Globe>) { self.globe = globe; }

    /// Returns the current scene mode.
    pub fn mode(&self) -> SceneMode { self.mode }

    /// Sets the scene mode.
    pub fn set_mode(&mut self, mode: SceneMode) {
        self.mode = mode;
        self.morph_time = SceneMode::get_morph_time(mode).unwrap_or(0.0);
    }

    /// Returns the morph time.
    pub fn morph_time(&self) -> f64 { self.morph_time }

    /// Returns the frame state.
    pub fn frame_state(&self) -> &FrameState { &self.frame_state }

    /// Returns the credit display.
    pub fn credit_display(&self) -> &CreditDisplay { &self.credit_display }

    /// Returns the background color.
    pub fn background_color(&self) -> &Color { &self.background_color }

    /// Sets the background color.
    pub fn set_background_color(&mut self, color: Color) { self.background_color = color; }

    /// Updates the scene for the current frame.
    pub fn update(&mut self, time: &JulianDate) {
        self.frame_state.time = time.clone();
        self.frame_state.frame_number += 1;
        self.frame_state.mode = self.mode;
        self.frame_state.morph_time = self.morph_time;
        self.credit_display.begin_frame();
        // DEVIATION: Full update pipeline requires primitive collection traversal
        self.credit_display.end_frame();
    }

    /// Renders the scene.
    pub fn render(&mut self, time: &JulianDate) {
        self.update(time);
        // DEVIATION: Full render pipeline requires wgpu render pass creation
    }

    /// Returns whether this scene has been destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys the scene.
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for Scene {
    fn default() -> Self { Self::new() }
}
