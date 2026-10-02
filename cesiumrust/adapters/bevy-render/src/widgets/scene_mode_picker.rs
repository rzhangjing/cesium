//! 场景模式选择控件：以键盘快捷键在 3D / 2D / Columbus View 之间切换。
//!
//! [`SceneModeWidget`] 作为资源保存当前模式与指示器开关；
//! [`scene_mode_picker_system`] 监听 U/I/O 按键，切换后把模式同步到相机。
use bevy::prelude::*;
use cesium_scene_mode::SceneMode;

/// 场景模式选择控件资源（持有当前模式与是否打印指示）。
#[derive(Resource, Debug, Clone)]
pub struct SceneModeWidget {
    /// 当前场景模式。
    pub current_mode: SceneMode,
    /// 切换时是否在日志中打印模式名称。
    pub show_indicator: bool,
}

impl Default for SceneModeWidget {
    /// 默认：初始为 3D 模式，开启指示器。
    fn default() -> Self {
        Self {
            current_mode: SceneMode::Scene3D,
            show_indicator: true,
        }
    }
}

impl SceneModeWidget {
    /// 切换到 3D 球面模式。
    pub fn select_3d(&mut self) {
        self.current_mode = SceneMode::Scene3D;
    }

    /// 切换到 2D 平面（墨卡托）模式。
    pub fn select_2d(&mut self) {
        self.current_mode = SceneMode::Scene2D;
    }

    /// 切换到 Columbus View（2D 俯视）模式。
    pub fn select_columbus_view(&mut self) {
        self.current_mode = SceneMode::ColumbusView;
    }

    /// 返回当前模式的短标签字符串（用于日志/指示器）。
    pub fn mode_label(&self) -> &'static str {
        match self.current_mode {
            SceneMode::Scene3D => "3D",
            SceneMode::Scene2D => "2D",
            SceneMode::ColumbusView => "Columbus View",
            SceneMode::Morphing => "Morphing...",
        }
    }
}

/// 初始化场景模式控件资源（当前无需额外实体，留作扩展点）。
///
/// # 参数
/// - `_commands`：Bevy 命令封装（未使用）
pub fn setup_scene_mode_picker(mut _commands: Commands) {}

/// 监听快捷键切换场景模式并同步到相机的系统。
///
/// # 参数
/// - `keyboard`：按键输入状态
/// - `widget`：场景模式控件资源（可写）
/// - `camera_query`：所有 Cesium 相机组件
pub fn scene_mode_picker_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut widget: ResMut<SceneModeWidget>,
    mut camera_query: Query<&mut crate::camera::CesiumCamera>,
) {
    // 记录本次是否有模式变化，避免每帧无谓地同步相机。
    let mut mode_changed = false;

    // U → 3D 球面。
    if keyboard.just_pressed(KeyCode::KeyU) {
        widget.select_3d();
        mode_changed = true;
    }

    // I → 2D 平面。
    if keyboard.just_pressed(KeyCode::KeyI) {
        widget.select_2d();
        mode_changed = true;
    }

    // O → Columbus View。
    if keyboard.just_pressed(KeyCode::KeyO) {
        widget.select_columbus_view();
        mode_changed = true;
    }

    // 仅在模式真正变化时把新模式写回所有相机，并按需打印指示。
    if mode_changed {
        for mut cam in camera_query.iter_mut() {
            cam.scene_mode = widget.current_mode;
        }

        if widget.show_indicator {
            info!("Scene mode: {}", widget.mode_label());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 默认控件应为 3D 模式且开启指示器。
    fn test_scene_mode_widget_default() {
        let widget = SceneModeWidget::default();
        assert_eq!(widget.current_mode, SceneMode::Scene3D);
        assert!(widget.show_indicator);
    }

    #[test]
    /// 三个选择器应分别把模式切到 2D/Columbus/3D。
    fn test_scene_mode_selectors() {
        let mut widget = SceneModeWidget::default();

        widget.select_2d();
        assert_eq!(widget.current_mode, SceneMode::Scene2D);

        widget.select_columbus_view();
        assert_eq!(widget.current_mode, SceneMode::ColumbusView);

        widget.select_3d();
        assert_eq!(widget.current_mode, SceneMode::Scene3D);
    }

    #[test]
    /// mode_label 应随当前模式返回对应的短标签字符串。
    fn test_mode_label() {
        let mut widget = SceneModeWidget::default();
        assert_eq!(widget.mode_label(), "3D");
        widget.select_2d();
        assert_eq!(widget.mode_label(), "2D");
        widget.select_columbus_view();
        assert_eq!(widget.mode_label(), "Columbus View");
    }
}
