use bevy::prelude::*;
use bevy::input::ButtonInput;
use bevy::gizmos::gizmos::Gizmos;

use crate::components::CesiumTileNode;
use crate::resources::RenderScale;

#[derive(Resource)]
pub struct DebugConfig {
    pub show_bounding_volumes: bool,
    pub show_tile_stats: bool,
    pub wireframe_mode: bool,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            show_bounding_volumes: false,
            show_tile_stats: false,
            wireframe_mode: false,
        }
    }
}

#[derive(Component)]
pub struct TilesetStatsText;

pub fn debug_toggle_system(keys: Res<ButtonInput<KeyCode>>, mut config: ResMut<DebugConfig>) {
    if keys.just_pressed(KeyCode::F1) {
        config.show_bounding_volumes = !config.show_bounding_volumes;
        info!("Bounding volumes: {}", config.show_bounding_volumes);
    }
    if keys.just_pressed(KeyCode::F2) {
        config.show_tile_stats = !config.show_tile_stats;
        info!("Tile stats: {}", config.show_tile_stats);
    }
    if keys.just_pressed(KeyCode::F3) {
        config.wireframe_mode = !config.wireframe_mode;
        info!("Wireframe mode: {}", config.wireframe_mode);
    }
}

pub fn draw_bounding_volumes(
    config: Res<DebugConfig>,
    tiles: Query<&CesiumTileNode>,
    render_scale: Res<RenderScale>,
    mut gizmos: Gizmos,
) {
    if !config.show_bounding_volumes {
        return;
    }

    let scale = render_scale.0;
    let segments = 32u32;

    for node in tiles.iter() {
        let (Some(center), Some(radius)) = (node.bounding_sphere_center, node.bounding_sphere_radius)
        else {
            continue;
        };

        let center_render = (center / scale).as_vec3();
        let radius_render = (radius / scale) as f32;

        let sse = node.screen_space_error as f32;
        let t = (sse / 32.0).clamp(0.0, 1.0);
        let color = Color::hsl(120.0 * (1.0 - t), 1.0, 0.5);

        for plane in 0..3u32 {
            for i in 0..segments {
                let angle0 = 2.0 * std::f32::consts::PI * i as f32 / segments as f32;
                let angle1 =
                    2.0 * std::f32::consts::PI * (i + 1) as f32 / segments as f32;
                let (sin0, cos0) = angle0.sin_cos();
                let (sin1, cos1) = angle1.sin_cos();

                let (p0, p1) = match plane {
                    0 => (
                        Vec3::new(cos0 * radius_render, sin0 * radius_render, 0.0),
                        Vec3::new(cos1 * radius_render, sin1 * radius_render, 0.0),
                    ),
                    1 => (
                        Vec3::new(cos0 * radius_render, 0.0, sin0 * radius_render),
                        Vec3::new(cos1 * radius_render, 0.0, sin1 * radius_render),
                    ),
                    _ => (
                        Vec3::new(0.0, cos0 * radius_render, sin0 * radius_render),
                        Vec3::new(0.0, cos1 * radius_render, sin1 * radius_render),
                    ),
                };

                gizmos.line(center_render + p0, center_render + p1, color);
            }
        }
    }
}

pub fn update_tile_stats(
    config: Res<DebugConfig>,
    tiles: Query<&CesiumTileNode>,
    mut stats_query: Query<(&mut Text, &mut Visibility), With<TilesetStatsText>>,
) {
    let Ok((mut text, mut visibility)) = stats_query.get_single_mut() else {
        return;
    };

    if config.show_tile_stats {
        *visibility = Visibility::Visible;

        let total = tiles.iter().count();
        let ready = tiles
            .iter()
            .filter(|t| matches!(t.state, crate::components::TileContentState::Ready))
            .count();
        let loading = tiles
            .iter()
            .filter(|t| matches!(t.state, crate::components::TileContentState::Loading))
            .count();
        let unloaded = tiles
            .iter()
            .filter(|t| matches!(t.state, crate::components::TileContentState::Unloaded))
            .count();

        text.0 = format!(
            "Tiles: {} total | {} ready | {} loading | {} pending",
            total, ready, loading, unloaded
        );
    } else {
        *visibility = Visibility::Hidden;
    }
}

pub fn spawn_stats_overlay(mut commands: Commands) {
    commands.spawn((
        Text::new("Tiles: 0 total | 0 ready | 0 loading | 0 pending"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::WHITE),
        TilesetStatsText,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        Visibility::Hidden,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_config_default() {
        let config = DebugConfig::default();
        assert!(!config.show_bounding_volumes);
        assert!(!config.show_tile_stats);
        assert!(!config.wireframe_mode);
    }

    #[test]
    fn test_debug_config_toggle() {
        let mut config = DebugConfig::default();

        config.show_bounding_volumes = !config.show_bounding_volumes;
        assert!(config.show_bounding_volumes);

        config.show_tile_stats = !config.show_tile_stats;
        assert!(config.show_tile_stats);

        config.wireframe_mode = !config.wireframe_mode;
        assert!(config.wireframe_mode);

        config.show_bounding_volumes = !config.show_bounding_volumes;
        assert!(!config.show_bounding_volumes);

        config.show_tile_stats = !config.show_tile_stats;
        assert!(!config.show_tile_stats);

        config.wireframe_mode = !config.wireframe_mode;
        assert!(!config.wireframe_mode);
    }
}
