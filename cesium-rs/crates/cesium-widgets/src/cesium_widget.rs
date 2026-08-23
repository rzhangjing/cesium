//! Ported from `packages/engine/Source/Widget/CesiumWidget.js`.
//!
//! A widget containing a Cesium scene.

use cesium_core::clock::Clock;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::event::Event;
use cesium_data_sources::data_source_collection::DataSourceCollection;
use cesium_data_sources::data_source_display::DataSourceDisplay;
use cesium_scene::scene::Scene;
use cesium_scene::scene_mode::SceneMode;

/// Configuration options for creating a CesiumWidget.
///
/// In CesiumJS, these are passed as the second argument to the constructor.
pub struct CesiumWidgetOptions {
    /// The clock to use for controlling simulation time.
    pub clock: Option<Clock>,
    /// Whether the clock should attempt to advance time by default.
    pub should_animate: bool,
    /// The default ellipsoid.
    pub ellipsoid: Ellipsoid,
    /// The initial scene mode.
    pub scene_mode: SceneMode,
    /// Whether each geometry instance will only be rendered in 3D.
    pub scene3d_only: bool,
    /// Whether to use order independent translucency.
    pub order_independent_translucency: bool,
    /// Whether this widget should control the render loop.
    pub use_default_render_loop: bool,
    /// Whether to render at the browser's recommended resolution.
    pub use_browser_recommended_resolution: bool,
    /// The target frame rate when using the default render loop.
    pub target_frame_rate: Option<f64>,
    /// Whether to display error panel on render loop errors.
    pub show_render_loop_errors: bool,
    /// Whether to automatically track DataSource clock settings.
    pub automatically_track_data_source_clocks: bool,
    /// Whether to use request-based rendering.
    pub request_render_mode: bool,
    /// Maximum time change before requesting a render.
    pub maximum_render_time_change: f64,
    /// Multisample antialiasing samples.
    pub msaa_samples: u32,
}

impl Default for CesiumWidgetOptions {
    fn default() -> Self {
        Self {
            clock: None,
            should_animate: false,
            ellipsoid: Ellipsoid::WGS84,
            scene_mode: SceneMode::Scene3D,
            scene3d_only: false,
            order_independent_translucency: true,
            use_default_render_loop: true,
            use_browser_recommended_resolution: true,
            target_frame_rate: None,
            show_render_loop_errors: true,
            automatically_track_data_source_clocks: true,
            request_render_mode: false,
            maximum_render_time_change: 0.0,
            msaa_samples: 4,
        }
    }
}

/// A widget containing a Cesium scene.
///
/// In CesiumJS, CesiumWidget.js is ~1600 lines. It manages:
/// - Canvas/DOM setup and resize handling
/// - Scene creation with globe, skybox, sky atmosphere
/// - DataSourceDisplay for entity visualization
/// - Render loop (requestAnimationFrame)
/// - Entity tracking and camera fly-to
/// - Clock synchronization with data sources
///
/// In Rust, DOM operations are replaced by winit window/surface management.
/// The render loop is driven by winit's event loop.
pub struct CesiumWidget {
    /// The 3D scene.
    scene: Scene,
    /// The clock controlling simulation time.
    clock: Clock,
    /// The collection of data sources.
    data_sources: DataSourceCollection,
    /// The data source display for entity visualization.
    data_source_display: DataSourceDisplay,
    /// Whether this widget controls the render loop.
    use_default_render_loop: bool,
    /// The target frame rate (None = unlimited).
    target_frame_rate: Option<f64>,
    /// Whether to show render loop errors.
    show_render_loop_errors: bool,
    /// Whether to automatically track data source clocks.
    automatically_track_data_source_clocks: bool,
    /// The resolution scale factor.
    resolution_scale: f64,
    /// Whether to use browser recommended resolution.
    use_browser_recommended_resolution: bool,
    /// Canvas client width in logical pixels.
    canvas_client_width: u32,
    /// Canvas client height in logical pixels.
    canvas_client_height: u32,
    /// Whether the widget can currently render (non-zero size).
    can_render: bool,
    /// Whether the render loop is running.
    render_loop_running: bool,
    /// The currently tracked entity ID.
    tracked_entity_id: Option<String>,
    /// Event fired when the tracked entity changes.
    tracked_entity_changed: Event,
    /// Whether a tracked entity update is needed.
    need_tracked_entity_update: bool,
    /// Whether a force resize is needed.
    force_resize: bool,
    /// Whether this widget has been destroyed.
    is_destroyed: bool,
}

impl CesiumWidget {
    /// Creates a new Cesium widget with default options.
    ///
    /// In CesiumJS, this takes a DOM container element and options.
    /// In Rust, the container is abstracted; the caller manages the window.
    pub fn new(options: Option<CesiumWidgetOptions>) -> Self {
        let opts = options.unwrap_or_default();
        let clock = opts.clock.unwrap_or_else(|| Clock::new(None, None, None, None, None, None, None, None));
        let data_sources = DataSourceCollection::new();
        let data_source_display = DataSourceDisplay::new(DataSourceCollection::new());

        let mut scene = Scene::new();
        scene.set_mode(opts.scene_mode);

        Self {
            scene,
            clock,
            data_sources,
            data_source_display,
            use_default_render_loop: opts.use_default_render_loop,
            target_frame_rate: opts.target_frame_rate,
            show_render_loop_errors: opts.show_render_loop_errors,
            automatically_track_data_source_clocks: opts.automatically_track_data_source_clocks,
            resolution_scale: 1.0,
            use_browser_recommended_resolution: opts.use_browser_recommended_resolution,
            canvas_client_width: 0,
            canvas_client_height: 0,
            can_render: false,
            render_loop_running: false,
            tracked_entity_id: None,
            tracked_entity_changed: Event::new(),
            need_tracked_entity_update: false,
            force_resize: false,
            is_destroyed: false,
        }
    }

    /// Returns the scene.
    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    /// Returns a mutable reference to the scene.
    pub fn scene_mut(&mut self) -> &mut Scene {
        &mut self.scene
    }

    /// Returns the clock.
    pub fn clock(&self) -> &Clock {
        &self.clock
    }

    /// Returns a mutable reference to the clock.
    pub fn clock_mut(&mut self) -> &mut Clock {
        &mut self.clock
    }

    /// Returns the data source collection.
    pub fn data_sources(&self) -> &DataSourceCollection {
        &self.data_sources
    }

    /// Returns a mutable reference to the data source collection.
    pub fn data_sources_mut(&mut self) -> &mut DataSourceCollection {
        &mut self.data_sources
    }

    /// Returns the data source display.
    pub fn data_source_display(&self) -> &DataSourceDisplay {
        &self.data_source_display
    }

    /// Returns a mutable reference to the data source display.
    pub fn data_source_display_mut(&mut self) -> &mut DataSourceDisplay {
        &mut self.data_source_display
    }

    /// Returns the tracked entity ID.
    pub fn tracked_entity_id(&self) -> Option<&str> {
        self.tracked_entity_id.as_deref()
    }

    /// Sets the tracked entity by ID.
    ///
    /// In CesiumJS, this triggers camera tracking of the entity.
    pub fn set_tracked_entity_id(&mut self, id: Option<String>) {
        if self.tracked_entity_id != id {
            self.tracked_entity_id = id;
            self.need_tracked_entity_update = true;
        }
    }

    /// Returns the resolution scale.
    pub fn resolution_scale(&self) -> f64 {
        self.resolution_scale
    }

    /// Sets the resolution scale.
    pub fn set_resolution_scale(&mut self, scale: f64) {
        self.resolution_scale = scale;
        self.force_resize = true;
    }

    /// Returns whether the widget can render.
    pub fn can_render(&self) -> bool {
        self.can_render
    }

    /// Resizes the widget.
    ///
    /// In CesiumJS, this reads canvas.clientWidth/Height and adjusts
    /// the canvas resolution and camera frustum.
    ///
    /// In Rust, this is called by the winit event handler on WindowResized.
    pub fn resize(&mut self, width: u32, height: u32) {
        let pixel_ratio = if self.use_browser_recommended_resolution {
            1.0
        } else {
            1.0 // DEVIATION: window.devicePixelRatio not available in Rust
        } * self.resolution_scale;

        self.canvas_client_width = width;
        self.canvas_client_height = height;

        let physical_width = (width as f64 * pixel_ratio) as u32;
        let physical_height = (height as f64 * pixel_ratio) as u32;

        self.can_render = physical_width != 0 && physical_height != 0;

        // In CesiumJS, this also updates scene.camera.frustum aspect ratio
        // DEVIATION: Camera frustum update requires camera access
    }

    /// Renders a single frame.
    ///
    /// In CesiumJS, this:
    /// 1. Updates the clock
    /// 2. Updates the data source display
    /// 3. Updates tracked entity camera
    /// 4. Renders the scene
    pub fn render(&mut self) {
        if self.is_destroyed || !self.can_render {
            return;
        }

        let time_ref = self.clock.current_time();
        let time = time_ref.day_number as f64 * 86400.0 + time_ref.seconds_of_day;

        // Update data source display
        self.data_source_display.update(time);

        // Update scene
        // DEVIATION: Full render requires wgpu render pass
        // self.scene.render(time);
    }

    /// Returns whether this widget controls the render loop.
    pub fn use_default_render_loop(&self) -> bool {
        self.use_default_render_loop
    }

    /// Sets whether this widget controls the render loop.
    pub fn set_use_default_render_loop(&mut self, value: bool) {
        self.use_default_render_loop = value;
    }

    /// Returns whether this widget has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys this widget and releases resources.
    ///
    /// In CesiumJS, this destroys the scene, data source display,
    /// and removes DOM elements.
    pub fn destroy(&mut self) {
        self.scene.destroy();
        self.data_source_display.destroy();
        self.is_destroyed = true;
    }
}

impl Default for CesiumWidget {
    fn default() -> Self {
        Self::new(None)
    }
}
