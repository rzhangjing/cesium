//! Ported from `packages/widgets/Source/InfoBox/InfoBoxViewModel.js`.
//!
//! The view model for the InfoBox widget.

use cesium_core::event::Event;

const CAMERA_ENABLED_PATH: &str = "M 13.84375 7.03125 C 11.412798 7.03125 9.46875 8.975298 9.46875 11.40625 L 9.46875 11.59375 L 2.53125 7.21875 L 2.53125 24.0625 L 9.46875 19.6875 C 9.4853444 22.104033 11.423165 24.0625 13.84375 24.0625 L 25.875 24.0625 C 28.305952 24.0625 30.28125 22.087202 30.28125 19.65625 L 30.28125 11.40625 C 30.28125 8.975298 28.305952 7.03125 25.875 7.03125 L 13.84375 7.03125 z";
const CAMERA_DISABLED_PATH: &str = "M 27.34375 1.65625 L 5.28125 27.9375 L 8.09375 30.3125 L 30.15625 4.03125 L 27.34375 1.65625 z M 13.84375 7.03125 C 11.412798 7.03125 9.46875 8.975298 9.46875 11.40625 L 9.46875 11.59375 L 2.53125 7.21875 L 2.53125 24.0625 L 9.46875 19.6875 C 9.4724893 20.232036 9.5676108 20.7379 9.75 21.21875 L 21.65625 7.03125 L 13.84375 7.03125 z M 28.21875 7.71875 L 14.53125 24.0625 L 25.875 24.0625 C 28.305952 24.0625 30.28125 22.087202 30.28125 19.65625 L 30.28125 11.40625 C 30.28125 9.8371439 29.456025 8.4902779 28.21875 7.71875 z";

/// The view model for the InfoBox widget.
pub struct InfoBoxViewModel {
    camera_clicked: Event<()>,
    close_clicked: Event<()>,
    /// Gets or sets the maximum height of the info box in pixels (mirrors
    /// the `maxHeight` observable).
    max_height: f64,
    /// Gets or sets whether the camera tracking icon is enabled (mirrors
    /// the `enableCamera` observable).
    enable_camera: bool,
    /// Gets or sets the status of current camera tracking of the selected
    /// object (mirrors the `isCameraTracking` observable).
    is_camera_tracking: bool,
    /// Gets or sets the visibility of the info box (mirrors the `showInfo`
    /// observable).
    show_info: bool,
    /// Gets or sets the title text in the info box (mirrors the
    /// `titleText` observable).
    title_text: String,
    /// Gets or sets the description HTML for the info box (mirrors the
    /// `description` observable).
    description: String,
    _loading_indicator_html: String,
}

impl InfoBoxViewModel {
    /// Port of `new InfoBoxViewModel()`.
    pub fn new() -> Self {
        Self {
            camera_clicked: Event::new(),
            close_clicked: Event::new(),
            max_height: 500.0,
            enable_camera: false,
            is_camera_tracking: false,
            show_info: false,
            title_text: String::new(),
            description: String::new(),
            _loading_indicator_html: "<div class=\"cesium-infoBox-loadingContainer\"><span class=\"cesium-infoBox-loading\"></span></div>".to_string(),
        }
    }

    /// Gets the maximum height of sections within the info box, minus an
    /// offset, in CSS-ready form (mirrors `maxHeightOffset`).
    pub fn max_height_offset(&self, offset: f64) -> String {
        format!("{}px", self.max_height - offset)
    }

    /// Gets the SVG path of the camera icon, which can change to be
    /// "crossed out" or not (mirrors the `cameraIconPath` computed).
    pub fn camera_icon_path(&self) -> &'static str {
        if !self.enable_camera || self.is_camera_tracking {
            CAMERA_DISABLED_PATH
        } else {
            CAMERA_ENABLED_PATH
        }
    }

    /// Mirrors the private `_bodyless` computed: true when the description
    /// is undefined or empty.
    pub fn bodyless(&self) -> bool {
        self.description.is_empty()
    }

    /// Gets an [`Event`] that is fired when the user clicks the camera
    /// icon (mirrors the read-only `cameraClicked` property).
    pub fn camera_clicked(&self) -> &Event<()> {
        &self.camera_clicked
    }

    /// Gets an [`Event`] that is fired when the user closes the info box
    /// (mirrors the read-only `closeClicked` property).
    pub fn close_clicked(&self) -> &Event<()> {
        &self.close_clicked
    }

    /// Gets the maximum height of the info box in pixels.
    pub fn max_height(&self) -> f64 {
        self.max_height
    }

    /// Sets the maximum height of the info box in pixels.
    pub fn set_max_height(&mut self, value: f64) {
        self.max_height = value;
    }

    /// Gets whether the camera tracking icon is enabled.
    pub fn enable_camera(&self) -> bool {
        self.enable_camera
    }

    /// Sets whether the camera tracking icon is enabled.
    pub fn set_enable_camera(&mut self, value: bool) {
        self.enable_camera = value;
    }

    /// Gets the status of current camera tracking of the selected object.
    pub fn is_camera_tracking(&self) -> bool {
        self.is_camera_tracking
    }

    /// Sets the status of current camera tracking of the selected object.
    pub fn set_is_camera_tracking(&mut self, value: bool) {
        self.is_camera_tracking = value;
    }

    /// Gets the visibility of the info box.
    pub fn show_info(&self) -> bool {
        self.show_info
    }

    /// Sets the visibility of the info box.
    pub fn set_show_info(&mut self, value: bool) {
        self.show_info = value;
    }

    /// Gets the title text in the info box.
    pub fn title_text(&self) -> &str {
        &self.title_text
    }

    /// Sets the title text in the info box.
    pub fn set_title_text(&mut self, value: &str) {
        self.title_text = value.to_string();
    }

    /// Gets the description HTML for the info box.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Sets the description HTML for the info box.
    pub fn set_description(&mut self, value: &str) {
        self.description = value.to_string();
    }
}

impl Default for InfoBoxViewModel {
    fn default() -> Self {
        Self::new()
    }
}
