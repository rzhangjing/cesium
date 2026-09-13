use bevy::prelude::*;
use cesium_scene_mode::SceneMode;

#[derive(Resource, Debug, Clone)]
pub struct SceneModeWidget {
    pub current_mode: SceneMode,
    pub show_indicator: bool,
}

impl Default for SceneModeWidget {
    fn default() -> Self {
        Self {
            current_mode: SceneMode::Scene3D,
            show_indicator: true,
        }
    }
}

impl SceneModeWidget {
    pub fn select_3d(&mut self) {
        self.current_mode = SceneMode::Scene3D;
    }

    pub fn select_2d(&mut self) {
        self.current_mode = SceneMode::Scene2D;
    }

    pub fn select_columbus_view(&mut self) {
        self.current_mode = SceneMode::ColumbusView;
    }

    pub fn mode_label(&self) -> &'static str {
        match self.current_mode {
            SceneMode::Scene3D => "3D",
            SceneMode::Scene2D => "2D",
            SceneMode::ColumbusView => "Columbus View",
            SceneMode::Morphing => "Morphing...",
        }
    }
}

pub fn setup_scene_mode_picker(mut _commands: Commands) {}

pub fn scene_mode_picker_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut widget: ResMut<SceneModeWidget>,
    mut camera_query: Query<&mut crate::camera::CesiumCamera>,
) {
    let mut mode_changed = false;

    if keyboard.just_pressed(KeyCode::KeyU) {
        widget.select_3d();
        mode_changed = true;
    }

    if keyboard.just_pressed(KeyCode::KeyI) {
        widget.select_2d();
        mode_changed = true;
    }

    if keyboard.just_pressed(KeyCode::KeyO) {
        widget.select_columbus_view();
        mode_changed = true;
    }

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
    fn test_scene_mode_widget_default() {
        let widget = SceneModeWidget::default();
        assert_eq!(widget.current_mode, SceneMode::Scene3D);
        assert!(widget.show_indicator);
    }

    #[test]
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
    fn test_mode_label() {
        let mut widget = SceneModeWidget::default();
        assert_eq!(widget.mode_label(), "3D");
        widget.select_2d();
        assert_eq!(widget.mode_label(), "2D");
        widget.select_columbus_view();
        assert_eq!(widget.mode_label(), "Columbus View");
    }
}
