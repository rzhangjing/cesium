//! Ported from `packages/widgets/Source/Viewer/Viewer.js`.
//!
//! The main Cesium viewer widget.

use cesium_core::clock::Clock;
use cesium_core::event::Event;
use cesium_data_sources::data_source_collection::DataSourceCollection;
use crate::cesium_widget::{CesiumWidget, CesiumWidgetOptions};

/// Configuration options for creating a Viewer.
///
/// In CesiumJS, Viewer.js is ~2000 lines. It wraps CesiumWidget and adds
/// timeline, animation controls, geocoder, base layer picker, etc.
pub struct ViewerOptions {
    /// CesiumWidget options.
    pub cesium_widget: Option<CesiumWidgetOptions>,
    /// Whether to show the animation widget.
    pub animation: bool,
    /// Whether to show the base layer picker.
    pub base_layer_picker: bool,
    /// Whether to show the fullscreen button.
    pub fullscreen_button: bool,
    /// Whether to show the geocoder.
    pub geocoder: bool,
    /// Whether to show the home button.
    pub home_button: bool,
    /// Whether to show the info box.
    pub info_box: bool,
    /// Whether to show the navigation help button.
    pub navigation_help_button: bool,
    /// Whether to show the projection picker.
    pub projection_picker: bool,
    /// Whether to show the scene mode picker.
    pub scene_mode_picker: bool,
    /// Whether to show the selection indicator.
    pub selection_indicator: bool,
    /// Whether to show the timeline.
    pub timeline: bool,
    /// Whether to show the VR button.
    pub vr_button: bool,
}

impl Default for ViewerOptions {
    fn default() -> Self {
        Self {
            cesium_widget: None,
            animation: true,
            base_layer_picker: true,
            fullscreen_button: true,
            geocoder: true,
            home_button: true,
            info_box: true,
            navigation_help_button: true,
            projection_picker: true,
            scene_mode_picker: true,
            selection_indicator: true,
            timeline: true,
            vr_button: false,
        }
    }
}

/// The main Cesium viewer widget.
///
/// In CesiumJS, Viewer.js is ~2000 lines. It provides:
/// - A CesiumWidget for 3D rendering
/// - Animation widget (play/pause/speed)
/// - Timeline widget (time scrubbing)
/// - Geocoder (location search)
/// - Home button (reset view)
/// - Scene mode picker (3D/2D/Columbus)
/// - Base layer picker (imagery selection)
/// - Fullscreen button
/// - Info box (entity details)
/// - Selection indicator
/// - Navigation help button
/// - 5 mixins: dragDrop, performanceWatchdog, cesiumInspector, 3dTilesInspector, voxelInspector
///
/// In Rust, the UI widgets are abstracted. The viewer focuses on
/// engine-side logic: scene management, data source display, entity tracking.
pub struct Viewer {
    /// The underlying Cesium widget.
    cesium_widget: CesiumWidget,
    /// Whether the viewer is currently rendering.
    is_rendering: bool,
    /// The currently selected entity ID.
    selected_entity_id: Option<String>,
    /// Event fired when the selected entity changes
    /// (`selectedEntityChanged`, raised with the new selection).
    selected_entity_changed: Event<Option<String>>,
    /// Whether the viewer has been destroyed.
    is_destroyed: bool,
}

impl Viewer {
    /// Creates a new viewer with default options.
    pub fn new(options: Option<ViewerOptions>) -> Self {
        let opts = options.unwrap_or_default();
        let cesium_widget = CesiumWidget::new(opts.cesium_widget);

        Self {
            cesium_widget,
            is_rendering: false,
            selected_entity_id: None,
            selected_entity_changed: Event::new(),
            is_destroyed: false,
        }
    }

    /// Returns the underlying Cesium widget.
    pub fn cesium_widget(&self) -> &CesiumWidget {
        &self.cesium_widget
    }

    /// Returns a mutable reference to the underlying Cesium widget.
    pub fn cesium_widget_mut(&mut self) -> &mut CesiumWidget {
        &mut self.cesium_widget
    }

    /// Returns the currently selected entity ID.
    pub fn selected_entity_id(&self) -> Option<&str> {
        self.selected_entity_id.as_deref()
    }

    /// Sets the selected entity by ID, raising
    /// [`Viewer::selected_entity_changed`] with the new selection when
    /// it changes (mirrors the knockout observable write on
    /// `selectedEntity` in CesiumJS).
    pub fn set_selected_entity_id(&mut self, id: Option<String>) {
        if self.selected_entity_id != id {
            self.selected_entity_id = id.clone();
            self.selected_entity_changed.raise_event(&id);
        }
    }

    /// Returns the `selectedEntityChanged` event.
    pub fn selected_entity_changed(&self) -> &Event<Option<String>> {
        &self.selected_entity_changed
    }

    /// Returns the clock used to control simulation time
    /// (`viewer.clock`, the cesiumWidget clock).
    pub fn clock(&self) -> &Clock {
        self.cesium_widget.clock()
    }

    /// Returns the data source collection (`viewer.dataSources`).
    pub fn data_sources(&self) -> &DataSourceCollection {
        self.cesium_widget.data_sources()
    }

    /// Returns a mutable reference to the data source collection.
    pub fn data_sources_mut(&mut self) -> &mut DataSourceCollection {
        self.cesium_widget.data_sources_mut()
    }

    /// Returns the currently tracked entity ID (`viewer.trackedEntity`,
    /// delegated to the cesiumWidget).
    pub fn tracked_entity_id(&self) -> Option<&str> {
        self.cesium_widget.tracked_entity_id()
    }

    /// Sets the tracked entity by ID, delegating to the cesiumWidget
    /// (mirrors the `viewer.trackedEntity` knockout observable, which
    /// is literally `cesiumWidget.trackedEntity` in CesiumJS).
    pub fn set_tracked_entity_id(&mut self, id: Option<String>) {
        self.cesium_widget.set_tracked_entity_id(id);
    }

    /// Returns the `trackedEntityChanged` event (delegated to the
    /// cesiumWidget, as in CesiumJS).
    pub fn tracked_entity_changed(&self) -> &Event<Option<String>> {
        self.cesium_widget.tracked_entity_changed()
    }

    /// Resizes the viewer.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.cesium_widget.resize(width, height);
    }

    /// Renders a single frame.
    pub fn render(&mut self) {
        if self.is_destroyed {
            return;
        }
        self.is_rendering = true;
        self.cesium_widget.render();
        self.is_rendering = false;
    }

    /// Returns whether the viewer is currently rendering.
    pub fn is_rendering(&self) -> bool {
        self.is_rendering
    }

    /// Returns whether this viewer has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys this viewer and all sub-widgets.
    pub fn destroy(&mut self) {
        self.cesium_widget.destroy();
        self.is_destroyed = true;
    }
}

impl Default for Viewer {
    fn default() -> Self {
        Self::new(None)
    }
}
