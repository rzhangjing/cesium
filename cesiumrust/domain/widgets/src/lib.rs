//! cesium-widgets：Cesium 查看器 UI 的 Widget 视图模型与 i18n。
//!
//! 映射到 CesiumJS `packages/widgets/Source/`：
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
//! # 特性
//! - 纯领域视图模型（无 UI 框架依赖）
//! - 带动感环角度转换的动画控制
//! - 带轨道与高亮区间的时间轴
//! - 场景模式与投影选择器
//! - 带提供器视图模型的基础图层选择器
//! - 带自动补全的地名搜索
//! - 多语言区域的 i18n 支持

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
