//! cesium-widgets: Widget view models and i18n for Cesium viewer UI.
//!
//! Maps to CesiumJS `packages/widgets/Source/`:
//! - `Animation/AnimationViewModel.js` → animation
//! - `Timeline/Timeline.js` → timeline
//! - `SceneModePicker/SceneModePickerViewModel.js` → scene_mode_picker
//! - `ProjectionPicker/ProjectionPickerViewModel.js` → projection_picker
//! - `BaseLayerPicker/BaseLayerPickerViewModel.js` → base_layer_picker
//! - `Geocoder/GeocoderViewModel.js` → geocoder
//! - `HomeButton/HomeButtonViewModel.js` → buttons
//! - `FullscreenButton/FullscreenButtonViewModel.js` → buttons
//! - `NavigationHelpButton/NavigationHelpButtonViewModel.js` → buttons
//! - `VRButton/VRButtonViewModel.js` → buttons
//! - `InfoBox/InfoBoxViewModel.js` → info_box
//! - `SelectionIndicator/SelectionIndicatorViewModel.js` → selection_indicator
//!
//! # Features
//! - Pure domain view models (no UI framework dependency)
//! - Animation control with shuttle ring angle conversion
//! - Timeline with tracks and highlight ranges
//! - Scene mode and projection pickers
//! - Base layer picker with provider view models
//! - Geocoder with autocomplete
//! - i18n support for multiple locales

pub mod animation;
pub mod timeline;
pub mod scene_mode_picker;
pub mod projection_picker;
pub mod base_layer_picker;
pub mod geocoder;
pub mod buttons;
pub mod info_box;
pub mod selection_indicator;
pub mod i18n;

pub use animation::{AnimationViewModel, ShuttleRing};
pub use timeline::{Timeline, TimelineTrack, TimelineHighlightRange, TimelineTicScale};
pub use scene_mode_picker::SceneModePickerViewModel;
pub use projection_picker::{ProjectionPickerViewModel, ProjectionType};
pub use base_layer_picker::{BaseLayerPickerViewModel, ProviderViewModel, ProviderCategory};
pub use geocoder::GeocoderViewModel;
pub use buttons::{
    HomeButtonViewModel, FullscreenButtonViewModel,
    NavigationHelpButtonViewModel, VRButtonViewModel, ToggleButtonViewModel,
};
pub use info_box::InfoBoxViewModel;
pub use selection_indicator::SelectionIndicatorViewModel;
pub use i18n::{Locale, I18n, WidgetStrings};
