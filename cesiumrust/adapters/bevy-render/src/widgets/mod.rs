pub mod animation;
pub mod geocoder;
pub mod scene_mode_picker;

pub use animation::{
    animation_widget_system, setup_animation_widget, AnimationWidget,
};
pub use geocoder::{
    geocoder_widget_system, setup_geocoder_widget, GeocoderWidget,
};
pub use scene_mode_picker::{
    scene_mode_picker_system, setup_scene_mode_picker, SceneModeWidget,
};

use bevy::prelude::*;

pub struct CesiumWidgetPlugin;

impl Plugin for CesiumWidgetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AnimationWidget>()
            .init_resource::<GeocoderWidget>()
            .init_resource::<SceneModeWidget>()
            .add_systems(
                Update,
                (
                    animation_widget_system,
                    geocoder_widget_system,
                    scene_mode_picker_system,
                ),
            );
    }
}
