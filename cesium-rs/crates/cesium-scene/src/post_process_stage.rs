//! Ported from `packages/engine/Source/Scene/PostProcessStage.js`.
//!
//! A post-process rendering stage.

use cesium_core::color::Color;

/// A post-process stage that applies a fragment shader to the scene output.
///
/// Runs a post-process stage on either the texture rendered by the scene or
/// the output of a previous post-process stage.
/// Mirrors CesiumJS `PostProcessStage` (1003 lines).
pub struct PostProcessStage {
    /// The unique name of this stage.
    pub name: String,
    /// The fragment shader source code.
    pub fragment_shader: String,
    /// Scale factor for the output texture dimensions (0.0, 1.0].
    pub texture_scale: f64,
    /// Whether to force power-of-two texture dimensions.
    pub force_power_of_two: bool,
    /// The clear color for the output texture.
    pub clear_color: Color,
    /// Whether this stage is enabled.
    pub enabled: bool,
    /// Whether this stage has been initialized (resources created).
    initialized: bool,
    /// Whether this stage is ready for rendering.
    ready: bool,
    /// The width of the output texture in pixels.
    width: i32,
    /// The height of the output texture in pixels.
    height: i32,
}

impl PostProcessStage {
    /// Creates a new PostProcessStage.
    pub fn new(fragment_shader: String) -> Self {
        Self {
            name: String::new(),
            fragment_shader,
            texture_scale: 1.0,
            force_power_of_two: false,
            clear_color: Color::new(0.0, 0.0, 0.0, 1.0),
            enabled: true,
            initialized: false,
            ready: false,
            width: 0,
            height: 0,
        }
    }

    /// Returns whether this stage is ready for rendering.
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Returns whether this stage has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Updates the stage for the current frame.
    pub fn update(&mut self, _width: i32, _height: i32) {
        // DEVIATION: Requires wgpu render pipeline creation and texture management
        if !self.initialized {
            self.width = _width;
            self.height = _height;
            self.initialized = true;
            self.ready = true;
        }
    }

    /// Returns the output texture dimensions.
    pub fn dimensions(&self) -> (i32, i32) {
        (self.width, self.height)
    }
}

impl Default for PostProcessStage {
    fn default() -> Self { Self::new(String::new()) }
}
