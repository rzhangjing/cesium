use bevy::prelude::*;
use cesium_effects::{
    OitConfig as DomainOitConfig, OitMode,
    SplitterConfig,
};

#[derive(Resource, Debug, Clone)]
pub struct OitConfig {
    pub enabled: bool,
    pub mode: OitMode,
    pub depth_test: bool,
    pub depth_write: bool,
    pub accumulation_clear: [f64; 4],
    pub revealage_clear: f64,
}

impl Default for OitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: OitMode::None,
            depth_test: true,
            depth_write: true,
            accumulation_clear: [0.0, 0.0, 0.0, 0.0],
            revealage_clear: 1.0,
        }
    }
}

impl OitConfig {
    pub fn from_domain(config: &DomainOitConfig) -> Self {
        Self {
            enabled: config.is_active(),
            mode: config.mode,
            depth_test: true,
            depth_write: true,
            accumulation_clear: [0.0, 0.0, 0.0, 0.0],
            revealage_clear: 1.0,
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct SplitConfig {
    pub enabled: bool,
    pub split_position: f64,
    pub dragging: bool,
    pub drag_start_x: f64,
}

impl Default for SplitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            split_position: 0.5,
            dragging: false,
            drag_start_x: 0.0,
        }
    }
}

impl SplitConfig {
    pub fn from_splitter(config: &SplitterConfig) -> Self {
        Self {
            enabled: config.enabled,
            split_position: config.split_position,
            ..Default::default()
        }
    }
}

#[derive(Event)]
pub struct SplitDragEvent {
    pub position: f64,
}

pub struct OITPlugin;

impl Plugin for OITPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OitConfig>()
            .init_resource::<SplitConfig>()
            .add_event::<SplitDragEvent>()
            .add_systems(Update, split_direction_system);
    }
}

pub fn split_direction_system(
    config: Res<SplitConfig>,
    _mouse_input: Res<ButtonInput<MouseButton>>,
    mut cursor_moved: EventReader<CursorMoved>,
    mut split_events: EventWriter<SplitDragEvent>,
    windows: Query<&Window>,
) {
    if !config.enabled {
        return;
    }

    for cursor in cursor_moved.read() {
        if config.dragging {
            if let Ok(window) = windows.get_single() {
                let pos = cursor.position.x as f64 / window.width() as f64;
                split_events.send(SplitDragEvent {
                    position: pos.clamp(0.0, 1.0),
                });
            }
        }
    }
}

pub fn accumulate_pass_system(
    _config: Res<OitConfig>,
    _camera_query: Query<&Camera3d>,
) {
}

pub fn revealage_pass_system(
    _config: Res<OitConfig>,
    _camera_query: Query<&Camera3d>,
) {
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_effects::{OitCapabilities, SplitDirection};

    #[test]
    fn test_oit_config_default() {
        let config = OitConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.mode, OitMode::None);
        assert!(config.depth_test);
    }

    #[test]
    fn test_oit_config_from_domain_active() {
        let domain = DomainOitConfig::default();
        let config = OitConfig::from_domain(&domain);
        assert!(!config.enabled);
    }

    #[test]
    fn test_oit_config_from_domain_mrt() {
        let caps = OitCapabilities {
            mrt_supported: true,
            float_blend_supported: true,
            depth_texture_supported: true,
            color_buffer_float: true,
        };
        let domain = DomainOitConfig::from_capabilities(&caps);
        let config = OitConfig::from_domain(&domain);
        assert!(config.enabled);
        assert_eq!(config.mode, OitMode::WeightedBlendedMrt);
    }

    #[test]
    fn test_split_config_default() {
        let config = SplitConfig::default();
        assert!(!config.enabled);
        assert!(!config.dragging);
        assert_eq!(config.split_position, 0.5);
    }

    #[test]
    fn test_split_config_from_splitter() {
        let domain_cfg = SplitterConfig::new(true, 0.3);
        let config = SplitConfig::from_splitter(&domain_cfg);
        assert!(config.enabled);
        assert_eq!(config.split_position, 0.3);
    }

    #[test]
    fn test_split_direction_properties() {
        assert_eq!(SplitDirection::Left.to_shader_value(), -1.0);
        assert_eq!(SplitDirection::None.to_shader_value(), 0.0);
        assert_eq!(SplitDirection::Right.to_shader_value(), 1.0);
        assert!(SplitDirection::Left.is_split());
        assert!(!SplitDirection::None.is_split());
    }
}
