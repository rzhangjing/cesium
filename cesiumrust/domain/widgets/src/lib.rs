//! cesium-widgets：Cesium 查看器 UI 的 Widget 视图模型与 i18n。
//!
//! 按子模块划分各视图模型：
//! - [`animation`]：动画播放控制与动感环
//! - [`timeline`]：时间轴轨道、刻度与高亮区间
//! - [`scene_mode_picker`]：3D/2D/Columbus 视图等模式切换
//! - [`projection_picker`]：透视/正交投影选择
//! - [`base_layer_picker`]：基础影像与地形图层选择
//! - [`geocoder`]：地名搜索与自动补全
//! - [`buttons`]：主页/全屏/导航帮助/VR 等按钮
//! - [`info_box`]：实体信息展示框
//! - [`selection_indicator`]：选中项高亮指示
//! - [`i18n`]：多语言区域的文案与本地化
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
