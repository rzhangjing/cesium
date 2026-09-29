//! 选中指示器（selection indicator）widget 视图模型。
//!
//! 映射到 CesiumJS `SelectionIndicator/SelectionIndicatorViewModel.js`。

/// 选中指示器 widget 视图模型。
///
/// 在选中实体的屏幕位置显示一个可视指示器。
#[derive(Debug, Clone)]
pub struct SelectionIndicatorViewModel {
    /// 指示器是否可见。
    pub show: bool,
    /// 屏幕 X 坐标（像素）。
    pub screen_x: f64,
    /// 屏幕 Y 坐标（像素）。
    pub screen_y: f64,
    /// 指示器的缩放比例。
    pub scale: f64,
    /// 旋转角度（弧度）。
    pub rotation: f64,
    /// 指示器当前是否正在动画（出现/消失）。
    pub is_animating: bool,
    /// 动画进度 [0, 1]。
    pub animation_progress: f64,
    /// 选中实体是否在屏幕内。
    pub is_on_screen: bool,
}

impl Default for SelectionIndicatorViewModel {
    fn default() -> Self {
        Self {
            show: false,
            screen_x: 0.0,
            screen_y: 0.0,
            scale: 1.0,
            rotation: 0.0,
            is_animating: false,
            animation_progress: 0.0,
            is_on_screen: false,
        }
    }
}

impl SelectionIndicatorViewModel {
    /// 创建一个新的选中指示器。
    pub fn new() -> Self {
        Self::default()
    }

    /// 在指定的屏幕位置显示指示器。
    pub fn show_at(&mut self, x: f64, y: f64) {
        self.show = true;
        self.screen_x = x;
        self.screen_y = y;
        self.is_on_screen = true;
        if !self.is_animating {
            self.is_animating = true;
            self.animation_progress = 0.0;
        }
    }

    /// 隐藏指示器。
    pub fn hide(&mut self) {
        self.show = false;
        self.is_on_screen = false;
        self.is_animating = false;
        self.animation_progress = 0.0;
    }

    /// 更新屏幕位置。
    pub fn update_position(&mut self, x: f64, y: f64, on_screen: bool) {
        self.screen_x = x;
        self.screen_y = y;
        self.is_on_screen = on_screen;
    }

    /// 更新出现/消失动画。
    /// 若动画已完成则返回 true。
    pub fn update_animation(&mut self, delta_seconds: f64) -> bool {
        if !self.is_animating {
            return true;
        }

        let duration = 0.3; // 300ms 动画
        if self.show {
            // 出现中
            self.animation_progress += delta_seconds / duration;
            if self.animation_progress >= 1.0 {
                self.animation_progress = 1.0;
                self.is_animating = false;
                self.scale = 1.0;
                return true;
            }
            // 从 2.0 缩放到 1.0（弹入）
            self.scale = 2.0 - self.animation_progress;
        } else {
            // 消失中
            self.animation_progress += delta_seconds / duration;
            if self.animation_progress >= 1.0 {
                self.animation_progress = 1.0;
                self.is_animating = false;
                return true;
            }
            self.scale = 1.0 - self.animation_progress;
        }

        false
    }

    /// 设置旋转角度。
    pub fn set_rotation(&mut self, radians: f64) {
        self.rotation = radians;
    }

    /// 检查指示器是否应当被渲染。
    pub fn should_render(&self) -> bool {
        self.show && self.is_on_screen
    }

    /// 获取指示器的类 CSS transform 字符串。
    pub fn transform_description(&self) -> String {
        format!(
            "translate({:.1}px, {:.1}px) scale({:.3}) rotate({:.2}rad)",
            self.screen_x, self.screen_y, self.scale, self.rotation
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let vm = SelectionIndicatorViewModel::default();
        assert!(!vm.show);
        assert!(!vm.is_on_screen);
        assert_eq!(vm.scale, 1.0);
    }

    #[test]
    fn test_show_at() {
        let mut vm = SelectionIndicatorViewModel::new();
        vm.show_at(100.0, 200.0);
        assert!(vm.show);
        assert_eq!(vm.screen_x, 100.0);
        assert_eq!(vm.screen_y, 200.0);
        assert!(vm.is_on_screen);
        assert!(vm.is_animating);
    }

    #[test]
    fn test_hide() {
        let mut vm = SelectionIndicatorViewModel::new();
        vm.show_at(100.0, 200.0);
        vm.hide();
        assert!(!vm.show);
        assert!(!vm.is_on_screen);
        assert!(!vm.is_animating);
    }

    #[test]
    fn test_update_position() {
        let mut vm = SelectionIndicatorViewModel::new();
        vm.show_at(100.0, 200.0);
        vm.update_position(150.0, 250.0, true);
        assert_eq!(vm.screen_x, 150.0);
        assert_eq!(vm.screen_y, 250.0);

        vm.update_position(300.0, 400.0, false);
        assert!(!vm.is_on_screen);
    }

    #[test]
    fn test_appear_animation() {
        let mut vm = SelectionIndicatorViewModel::new();
        vm.show_at(100.0, 200.0);
        assert!(vm.is_animating);
        assert_eq!(vm.animation_progress, 0.0);

        // 部分更新
        let done = vm.update_animation(0.15);
        assert!(!done);
        assert!((vm.animation_progress - 0.5).abs() < 1e-10);
        assert!(vm.scale > 1.0 && vm.scale < 2.0);

        // 完成
        let done = vm.update_animation(0.2);
        assert!(done);
        assert!(!vm.is_animating);
        assert_eq!(vm.scale, 1.0);
    }

    #[test]
    fn test_should_render() {
        let mut vm = SelectionIndicatorViewModel::new();
        assert!(!vm.should_render());
        vm.show_at(100.0, 200.0);
        assert!(vm.should_render());
        vm.update_position(100.0, 200.0, false);
        assert!(!vm.should_render());
    }

    #[test]
    fn test_rotation() {
        let mut vm = SelectionIndicatorViewModel::new();
        vm.set_rotation(std::f64::consts::FRAC_PI_4);
        assert!((vm.rotation - std::f64::consts::FRAC_PI_4).abs() < 1e-10);
    }

    #[test]
    fn test_transform_description() {
        let mut vm = SelectionIndicatorViewModel::new();
        vm.show_at(100.0, 200.0);
        vm.update_animation(1.0); // 完成动画
        let desc = vm.transform_description();
        assert!(desc.contains("translate(100.0px, 200.0px)"));
        assert!(desc.contains("scale(1.000)"));
    }
}
