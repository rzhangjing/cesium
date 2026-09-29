//! 场景模式选择器视图模型。
//!
//! 映射到 CesiumJS `SceneModePicker/SceneModePickerViewModel.js`。

use cesium_scene_mode::SceneMode;

/// 场景模式选择器视图模型。
///
/// 控制 3D、2D 与 Columbus View 模式之间的切换。
#[derive(Debug, Clone)]
pub struct SceneModePickerViewModel {
    /// 当前选中的场景模式。
    pub selected_mode: SceneMode,
    /// 下拉菜单是否展开。
    pub is_dropdown_open: bool,
    /// widget 是否可见。
    pub show: bool,
    /// 模式渐变（morph）过渡的时长（秒）。
    pub morph_duration: f64,
}

impl Default for SceneModePickerViewModel {
    fn default() -> Self {
        Self {
            selected_mode: SceneMode::Scene3D,
            is_dropdown_open: false,
            show: true,
            morph_duration: 2.0,
        }
    }
}

impl SceneModePickerViewModel {
    /// 创建一个新的场景模式选择器。
    pub fn new() -> Self {
        Self::default()
    }

    /// 选择一个场景模式并触发渐变。
    pub fn select_mode(&mut self, mode: SceneMode) {
        if mode != SceneMode::Morphing {
            self.selected_mode = mode;
            self.is_dropdown_open = false;
        }
    }

    /// 选择 3D 模式。
    pub fn select_3d(&mut self) {
        self.select_mode(SceneMode::Scene3D);
    }

    /// 选择 2D 模式。
    pub fn select_2d(&mut self) {
        self.select_mode(SceneMode::Scene2D);
    }

    /// 选择 Columbus View 模式。
    pub fn select_columbus_view(&mut self) {
        self.select_mode(SceneMode::ColumbusView);
    }

    /// 切换下拉菜单。
    pub fn toggle_dropdown(&mut self) {
        self.is_dropdown_open = !self.is_dropdown_open;
    }

    /// 关闭下拉菜单。
    pub fn close_dropdown(&mut self) {
        self.is_dropdown_open = false;
    }

    /// 获取当前模式的显示标签。
    pub fn current_label(&self) -> &'static str {
        match self.selected_mode {
            SceneMode::Scene3D => "3D",
            SceneMode::Scene2D => "2D",
            SceneMode::ColumbusView => "Columbus View",
            SceneMode::Morphing => "Morphing",
        }
    }

    /// 获取给定模式的提示文本。
    pub fn tooltip_for_mode(mode: SceneMode) -> &'static str {
        match mode {
            SceneMode::Scene3D => "3D globe view",
            SceneMode::Scene2D => "2D flat map view",
            SceneMode::ColumbusView => "Columbus View (2.5D)",
            SceneMode::Morphing => "Morphing between modes",
        }
    }

    /// 获取所有可选模式（不包括 Morphing）。
    pub fn available_modes() -> &'static [SceneMode] {
        &[SceneMode::Scene3D, SceneMode::Scene2D, SceneMode::ColumbusView]
    }

    /// 检查某个模式当前是否已选中。
    pub fn is_mode_selected(&self, mode: SceneMode) -> bool {
        self.selected_mode == mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let vm = SceneModePickerViewModel::default();
        assert_eq!(vm.selected_mode, SceneMode::Scene3D);
        assert!(!vm.is_dropdown_open);
        assert!(vm.show);
        assert_eq!(vm.morph_duration, 2.0);
    }

    #[test]
    fn test_select_mode() {
        let mut vm = SceneModePickerViewModel::new();
        vm.is_dropdown_open = true;
        vm.select_mode(SceneMode::Scene2D);
        assert_eq!(vm.selected_mode, SceneMode::Scene2D);
        assert!(!vm.is_dropdown_open);
    }

    #[test]
    fn test_select_morphing_ignored() {
        let mut vm = SceneModePickerViewModel::new();
        vm.select_mode(SceneMode::Morphing);
        // 不应变为 Morphing
        assert_eq!(vm.selected_mode, SceneMode::Scene3D);
    }

    #[test]
    fn test_convenience_selectors() {
        let mut vm = SceneModePickerViewModel::new();
        vm.select_2d();
        assert_eq!(vm.selected_mode, SceneMode::Scene2D);
        vm.select_columbus_view();
        assert_eq!(vm.selected_mode, SceneMode::ColumbusView);
        vm.select_3d();
        assert_eq!(vm.selected_mode, SceneMode::Scene3D);
    }

    #[test]
    fn test_toggle_dropdown() {
        let mut vm = SceneModePickerViewModel::new();
        assert!(!vm.is_dropdown_open);
        vm.toggle_dropdown();
        assert!(vm.is_dropdown_open);
        vm.toggle_dropdown();
        assert!(!vm.is_dropdown_open);
    }

    #[test]
    fn test_current_label() {
        let mut vm = SceneModePickerViewModel::new();
        assert_eq!(vm.current_label(), "3D");
        vm.select_2d();
        assert_eq!(vm.current_label(), "2D");
        vm.select_columbus_view();
        assert_eq!(vm.current_label(), "Columbus View");
    }

    #[test]
    fn test_available_modes() {
        let modes = SceneModePickerViewModel::available_modes();
        assert_eq!(modes.len(), 3);
        assert!(!modes.contains(&SceneMode::Morphing));
    }

    #[test]
    fn test_is_mode_selected() {
        let mut vm = SceneModePickerViewModel::new();
        assert!(vm.is_mode_selected(SceneMode::Scene3D));
        assert!(!vm.is_mode_selected(SceneMode::Scene2D));
        vm.select_2d();
        assert!(vm.is_mode_selected(SceneMode::Scene2D));
        assert!(!vm.is_mode_selected(SceneMode::Scene3D));
    }
}
