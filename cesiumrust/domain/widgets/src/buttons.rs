//! 按钮 widget 视图模型。
//!
//! 映射到 CesiumJS：
//! - `HomeButton/HomeButtonViewModel.js`
//! - `FullscreenButton/FullscreenButtonViewModel.js`
//! - `NavigationHelpButton/NavigationHelpButtonViewModel.js`
//! - `VRButton/VRButtonViewModel.js`

// 遗留 CesiumJS 移植风格债（deferred.md #18）；将在 M13 lint 清理，或本文件在其所属里程碑被重写时重新审视
#![allow(clippy::field_reassign_with_default)]
/// 一个通用的切换按钮视图模型。
#[derive(Debug, Clone)]
pub struct ToggleButtonViewModel {
    /// 按钮是否已切换开启。
    pub is_toggled: bool,
    /// 按钮提示文本。
    pub tooltip: String,
    /// 按钮是否可见。
    pub show: bool,
    /// 按钮是否启用。
    pub is_enabled: bool,
}

impl ToggleButtonViewModel {
    /// 创建一个新的切换按钮。
    pub fn new(tooltip: impl Into<String>) -> Self {
        Self {
            is_toggled: false,
            tooltip: tooltip.into(),
            show: true,
            is_enabled: true,
        }
    }

    /// 切换按钮状态。
    pub fn toggle(&mut self) {
        if self.is_enabled {
            self.is_toggled = !self.is_toggled;
        }
    }

    /// 设置切换状态。
    pub fn set_toggled(&mut self, toggled: bool) {
        if self.is_enabled {
            self.is_toggled = toggled;
        }
    }
}

/// 主页（Home）按钮视图模型。
///
/// 将相机重置到默认的主页视图。
#[derive(Debug, Clone)]
pub struct HomeButtonViewModel {
    /// 按钮提示文本。
    pub tooltip: String,
    /// 按钮是否可见。
    pub show: bool,
    /// 主页飞行的时长（秒）。
    pub duration: f64,
    /// 主页视图经度（弧度）。
    pub home_longitude: f64,
    /// 主页视图纬度（弧度）。
    pub home_latitude: f64,
    /// 主页视图高度（米）。
    pub home_height: f64,
}

impl Default for HomeButtonViewModel {
    fn default() -> Self {
        Self {
            tooltip: "View Home".to_string(),
            show: true,
            duration: 1.5,
            // 默认主页：从远处眺望地球
            home_longitude: 0.0,
            home_latitude: 0.0,
            home_height: 15_000_000.0,
        }
    }
}

impl HomeButtonViewModel {
    /// 创建一个新的主页按钮。
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置主页视图位置。
    pub fn set_home(&mut self, longitude: f64, latitude: f64, height: f64) {
        self.home_longitude = longitude;
        self.home_latitude = latitude;
        self.home_height = height;
    }

    /// 以 (经度, 纬度, 高度) 获取主页位置。
    pub fn home_position(&self) -> (f64, f64, f64) {
        (self.home_longitude, self.home_latitude, self.home_height)
    }
}

/// 全屏按钮视图模型。
///
/// 切换浏览器全屏模式。
#[derive(Debug, Clone)]
pub struct FullscreenButtonViewModel {
    /// 当前是否处于全屏。
    pub is_fullscreen: bool,
    /// 非全屏时的提示文本。
    pub enter_tooltip: String,
    /// 全屏时的提示文本。
    pub exit_tooltip: String,
    /// 按钮是否可见。
    pub show: bool,
    /// 环境是否支持全屏。
    pub is_supported: bool,
}

impl Default for FullscreenButtonViewModel {
    fn default() -> Self {
        Self {
            is_fullscreen: false,
            enter_tooltip: "Full screen".to_string(),
            exit_tooltip: "Exit full screen".to_string(),
            show: true,
            is_supported: true,
        }
    }
}

impl FullscreenButtonViewModel {
    /// 创建一个新的全屏按钮。
    pub fn new() -> Self {
        Self::default()
    }

    /// 切换全屏状态。
    pub fn toggle_fullscreen(&mut self) {
        if self.is_supported {
            self.is_fullscreen = !self.is_fullscreen;
        }
    }

    /// 获取当前提示文本。
    pub fn current_tooltip(&self) -> &str {
        if self.is_fullscreen {
            &self.exit_tooltip
        } else {
            &self.enter_tooltip
        }
    }
}

/// 导航帮助按钮视图模型。
///
/// 显示/隐藏导航帮助覆盖层。
#[derive(Debug, Clone)]
pub struct NavigationHelpButtonViewModel {
    /// 帮助面板是否可见。
    pub is_help_visible: bool,
    /// 按钮提示文本。
    pub tooltip: String,
    /// 按钮是否可见。
    pub show: bool,
    /// 是否显示触摸导航帮助（而非鼠标）。
    pub show_touch: bool,
}

impl Default for NavigationHelpButtonViewModel {
    fn default() -> Self {
        Self {
            is_help_visible: false,
            tooltip: "Navigation Instructions".to_string(),
            show: true,
            show_touch: false,
        }
    }
}

impl NavigationHelpButtonViewModel {
    /// 创建一个新的导航帮助按钮。
    pub fn new() -> Self {
        Self::default()
    }

    /// 切换帮助面板。
    pub fn toggle_help(&mut self) {
        self.is_help_visible = !self.is_help_visible;
    }

    /// 显示帮助面板。
    pub fn show_help(&mut self) {
        self.is_help_visible = true;
    }

    /// 隐藏帮助面板。
    pub fn hide_help(&mut self) {
        self.is_help_visible = false;
    }

    /// 切换到鼠标导航说明。
    pub fn show_mouse_help(&mut self) {
        self.show_touch = false;
    }

    /// 切换到触摸导航说明。
    pub fn show_touch_help(&mut self) {
        self.show_touch = true;
    }
}

/// VR 按钮视图模型。
///
/// 切换 VR 模式。
#[derive(Debug, Clone)]
pub struct VRButtonViewModel {
    /// VR 模式是否已激活。
    pub is_vr_active: bool,
    /// 未进入 VR 时的提示文本。
    pub enter_tooltip: String,
    /// 处于 VR 时的提示文本。
    pub exit_tooltip: String,
    /// 按钮是否可见。
    pub show: bool,
    /// 是否支持 VR。
    pub is_supported: bool,
}

impl Default for VRButtonViewModel {
    fn default() -> Self {
        Self {
            is_vr_active: false,
            enter_tooltip: "Enter VR".to_string(),
            exit_tooltip: "Exit VR".to_string(),
            show: true,
            is_supported: false,
        }
    }
}

impl VRButtonViewModel {
    /// 创建一个新的 VR 按钮。
    pub fn new() -> Self {
        Self::default()
    }

    /// 切换 VR 模式。
    pub fn toggle_vr(&mut self) {
        if self.is_supported {
            self.is_vr_active = !self.is_vr_active;
        }
    }

    /// 获取当前提示文本。
    pub fn current_tooltip(&self) -> &str {
        if self.is_vr_active {
            &self.exit_tooltip
        } else {
            &self.enter_tooltip
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toggle_button() {
        let mut btn = ToggleButtonViewModel::new("Test");
        assert!(!btn.is_toggled);
        btn.toggle();
        assert!(btn.is_toggled);
        btn.toggle();
        assert!(!btn.is_toggled);
    }

    #[test]
    fn test_toggle_button_disabled() {
        let mut btn = ToggleButtonViewModel::new("Test");
        btn.is_enabled = false;
        btn.toggle();
        assert!(!btn.is_toggled);
    }

    #[test]
    fn test_home_button_default() {
        let btn = HomeButtonViewModel::default();
        assert_eq!(btn.tooltip, "View Home");
        assert_eq!(btn.duration, 1.5);
        assert_eq!(btn.home_height, 15_000_000.0);
    }

    #[test]
    fn test_home_button_set_home() {
        let mut btn = HomeButtonViewModel::new();
        btn.set_home(1.0, 0.5, 1000.0);
        assert_eq!(btn.home_position(), (1.0, 0.5, 1000.0));
    }

    #[test]
    fn test_fullscreen_button() {
        let mut btn = FullscreenButtonViewModel::default();
        assert!(!btn.is_fullscreen);
        assert_eq!(btn.current_tooltip(), "Full screen");
        btn.toggle_fullscreen();
        assert!(btn.is_fullscreen);
        assert_eq!(btn.current_tooltip(), "Exit full screen");
    }

    #[test]
    fn test_fullscreen_unsupported() {
        let mut btn = FullscreenButtonViewModel::default();
        btn.is_supported = false;
        btn.toggle_fullscreen();
        assert!(!btn.is_fullscreen);
    }

    #[test]
    fn test_navigation_help_button() {
        let mut btn = NavigationHelpButtonViewModel::default();
        assert!(!btn.is_help_visible);
        btn.toggle_help();
        assert!(btn.is_help_visible);
        btn.hide_help();
        assert!(!btn.is_help_visible);
        btn.show_help();
        assert!(btn.is_help_visible);
    }

    #[test]
    fn test_navigation_help_touch() {
        let mut btn = NavigationHelpButtonViewModel::default();
        assert!(!btn.show_touch);
        btn.show_touch_help();
        assert!(btn.show_touch);
        btn.show_mouse_help();
        assert!(!btn.show_touch);
    }

    #[test]
    fn test_vr_button() {
        let mut btn = VRButtonViewModel::default();
        assert!(!btn.is_vr_active);
        assert!(!btn.is_supported);
        // VR 不支持，切换不应生效
        btn.toggle_vr();
        assert!(!btn.is_vr_active);

        btn.is_supported = true;
        btn.toggle_vr();
        assert!(btn.is_vr_active);
        assert_eq!(btn.current_tooltip(), "Exit VR");
    }
}
