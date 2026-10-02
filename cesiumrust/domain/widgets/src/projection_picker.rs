//! 投影选择器视图模型。
//!
//! 封装透视与正交两种相机投影的切换、下拉展开状态，
//! 以及两者之间过渡动画的进度推进。

/// 相机的投影类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProjectionType {
    /// 透视投影。
    #[default]
    Perspective,
    /// 正交投影。
    Orthographic,
}

impl ProjectionType {
    /// 获取显示标签。
    pub fn label(&self) -> &'static str {
        match self {
            Self::Perspective => "Perspective",
            Self::Orthographic => "Orthographic",
        }
    }

    /// 获取提示文本。
    pub fn tooltip(&self) -> &'static str {
        match self {
            Self::Perspective => "Perspective projection",
            Self::Orthographic => "Orthographic projection",
        }
    }
}

/// 投影选择器视图模型。
///
/// 控制透视与正交投影之间的切换。
#[derive(Debug, Clone)]
pub struct ProjectionPickerViewModel {
    /// 当前选中的投影类型。
    pub selected_projection: ProjectionType,
    /// 下拉菜单是否展开。
    pub is_dropdown_open: bool,
    /// widget 是否可见。
    pub show: bool,
    /// 过渡是否带动画。
    pub is_transitioning: bool,
    /// 过渡进度 [0, 1]。
    pub transition_progress: f64,
}

impl Default for ProjectionPickerViewModel {
    /// 默认选中透视投影、下拉收起、widget 可见、无过渡。
    fn default() -> Self {
        Self {
            selected_projection: ProjectionType::Perspective,
            is_dropdown_open: false,
            show: true,
            is_transitioning: false,
            transition_progress: 0.0,
        }
    }
}

impl ProjectionPickerViewModel {
    /// 创建一个新的投影选择器。
    pub fn new() -> Self {
        Self::default()
    }

    /// 选择一种投影类型。
    pub fn select_projection(&mut self, projection: ProjectionType) {
        // 仅在切换到不同投影时启动过渡；无论否同都收起下拉
        if self.selected_projection != projection {
            self.selected_projection = projection;
            self.is_transitioning = true;
            self.transition_progress = 0.0;
        }
        self.is_dropdown_open = false;
    }

    /// 切换到透视投影。
    pub fn select_perspective(&mut self) {
        self.select_projection(ProjectionType::Perspective);
    }

    /// 切换到正交投影。
    pub fn select_orthographic(&mut self) {
        self.select_projection(ProjectionType::Orthographic);
    }

    /// 切换下拉菜单。
    pub fn toggle_dropdown(&mut self) {
        // 就地翻转展开标志
        self.is_dropdown_open = !self.is_dropdown_open;
    }

    /// 关闭下拉菜单。
    pub fn close_dropdown(&mut self) {
        // 失焦或外部选择时统一收起下拉
        self.is_dropdown_open = false;
    }

    /// 更新过渡动画。
    /// 若过渡已完成则返回 true。
    pub fn update_transition(&mut self, delta_seconds: f64) -> bool {
        // 未在过渡中则视为已完成，无需推进
        if !self.is_transitioning {
            return true;
        }

        let duration = 0.5; // 0.5 秒过渡
        // 按时长比例推进进度，达 1.0 时收敛并关闭过渡标志
        self.transition_progress += delta_seconds / duration;

        if self.transition_progress >= 1.0 {
            self.transition_progress = 1.0;
            self.is_transitioning = false;
            true
        } else {
            false
        }
    }

    /// 获取当前标签。
    pub fn current_label(&self) -> &'static str {
        // 直接代理到当前投影类型的显示标签
        self.selected_projection.label()
    }

    /// 检查某种投影是否已选中。
    pub fn is_selected(&self, projection: ProjectionType) -> bool {
        // 供下拉项判断高亮选中样式
        self.selected_projection == projection
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let vm = ProjectionPickerViewModel::default();
        assert_eq!(vm.selected_projection, ProjectionType::Perspective);
        assert!(!vm.is_dropdown_open);
        assert!(vm.show);
        assert!(!vm.is_transitioning);
    }

    #[test]
    fn test_select_projection() {
        let mut vm = ProjectionPickerViewModel::new();
        vm.select_orthographic();
        assert_eq!(vm.selected_projection, ProjectionType::Orthographic);
        assert!(vm.is_transitioning);
        assert_eq!(vm.transition_progress, 0.0);
    }

    #[test]
    fn test_select_same_projection() {
        let mut vm = ProjectionPickerViewModel::new();
        vm.select_perspective();
        // 已是透视投影，无过渡
        assert!(!vm.is_transitioning);
    }

    #[test]
    fn test_transition_update() {
        let mut vm = ProjectionPickerViewModel::new();
        vm.select_orthographic();
        assert!(vm.is_transitioning);

        // 部分更新
        let done = vm.update_transition(0.25);
        assert!(!done);
        assert!((vm.transition_progress - 0.5).abs() < 1e-10);

        // 完成
        let done = vm.update_transition(0.3);
        assert!(done);
        assert!(!vm.is_transitioning);
        assert_eq!(vm.transition_progress, 1.0);
    }

    #[test]
    fn test_toggle_dropdown() {
        let mut vm = ProjectionPickerViewModel::new();
        vm.toggle_dropdown();
        assert!(vm.is_dropdown_open);
        vm.toggle_dropdown();
        assert!(!vm.is_dropdown_open);
    }

    #[test]
    fn test_projection_type_labels() {
        assert_eq!(ProjectionType::Perspective.label(), "Perspective");
        assert_eq!(ProjectionType::Orthographic.label(), "Orthographic");
    }

    #[test]
    fn test_current_label() {
        let mut vm = ProjectionPickerViewModel::new();
        assert_eq!(vm.current_label(), "Perspective");
        vm.select_orthographic();
        assert_eq!(vm.current_label(), "Orthographic");
    }
}
