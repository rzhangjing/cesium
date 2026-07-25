//! Split direction for imagery layer splitting.
//!
//! Maps to CesiumJS `Scene/SplitDirection.js`.

use serde::{Deserialize, Serialize};

/// The direction to display a primitive or ImageryLayer relative to the split position.
///
/// Maps to CesiumJS `Scene/SplitDirection.js`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SplitDirection {
    /// Display to the left of the split position.
    Left,
    /// Always display (no splitting).
    #[default]
    None,
    /// Display to the right of the split position.
    Right,
}

impl SplitDirection {
    /// Get the numeric value for shader use.
    ///
    /// - Left: -1.0
    /// - None: 0.0
    /// - Right: 1.0
    pub fn to_shader_value(&self) -> f64 {
        match self {
            Self::Left => -1.0,
            Self::None => 0.0,
            Self::Right => 1.0,
        }
    }

    /// Create from a shader value.
    pub fn from_shader_value(value: f64) -> Self {
        if value < -0.5 {
            Self::Left
        } else if value > 0.5 {
            Self::Right
        } else {
            Self::None
        }
    }

    /// Check if splitting is active (not None).
    pub fn is_split(&self) -> bool {
        !matches!(self, Self::None)
    }

    /// Check if this direction should show at a given split position.
    ///
    /// `split_position` is in [0, 1] range (0 = left edge, 1 = right edge).
    /// `screen_x` is the normalized screen X coordinate [0, 1].
    pub fn should_show_at(&self, screen_x: f64, split_position: f64) -> bool {
        match self {
            Self::None => true,
            Self::Left => screen_x <= split_position,
            Self::Right => screen_x > split_position,
        }
    }
}

/// Splitter configuration for the scene.
///
/// Maps to CesiumJS `Scene/Splitter.js` and `Scene.splitPosition`.
#[derive(Debug, Clone, PartialEq)]
pub struct SplitterConfig {
    /// Whether splitting is enabled.
    pub enabled: bool,
    /// The split position as a fraction of the screen width [0, 1].
    pub split_position: f64,
}

impl Default for SplitterConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            split_position: 0.5,
        }
    }
}

impl SplitterConfig {
    /// Create a new splitter config.
    pub fn new(enabled: bool, split_position: f64) -> Self {
        Self {
            enabled,
            split_position: split_position.clamp(0.0, 1.0),
        }
    }

    /// Set the split position, clamped to [0, 1].
    pub fn set_split_position(&mut self, position: f64) {
        self.split_position = position.clamp(0.0, 1.0);
    }

    /// Get the split position in pixels for a given viewport width.
    pub fn split_position_pixels(&self, viewport_width: f64) -> f64 {
        self.split_position * viewport_width
    }

    /// Modify a fragment shader to include split logic.
    ///
    /// Returns the additional shader code to insert.
    pub fn shader_modification(&self) -> &str {
        if self.enabled {
            r#"
    // Split direction check
    float splitPosition = czm_splitPosition;
    if (v_splitDirection < 0.0 && gl_FragCoord.x > splitPosition) {
        discard;
    }
    if (v_splitDirection > 0.0 && gl_FragCoord.x <= splitPosition) {
        discard;
    }
"#
        } else {
            ""
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_direction_default() {
        assert_eq!(SplitDirection::default(), SplitDirection::None);
    }

    #[test]
    fn test_split_direction_shader_values() {
        assert_eq!(SplitDirection::Left.to_shader_value(), -1.0);
        assert_eq!(SplitDirection::None.to_shader_value(), 0.0);
        assert_eq!(SplitDirection::Right.to_shader_value(), 1.0);
    }

    #[test]
    fn test_split_direction_from_shader_value() {
        assert_eq!(SplitDirection::from_shader_value(-1.0), SplitDirection::Left);
        assert_eq!(SplitDirection::from_shader_value(0.0), SplitDirection::None);
        assert_eq!(SplitDirection::from_shader_value(1.0), SplitDirection::Right);
        assert_eq!(SplitDirection::from_shader_value(-0.3), SplitDirection::None);
        assert_eq!(SplitDirection::from_shader_value(0.3), SplitDirection::None);
    }

    #[test]
    fn test_split_direction_is_split() {
        assert!(SplitDirection::Left.is_split());
        assert!(!SplitDirection::None.is_split());
        assert!(SplitDirection::Right.is_split());
    }

    #[test]
    fn test_split_direction_should_show() {
        let split_pos = 0.5;

        // None always shows
        assert!(SplitDirection::None.should_show_at(0.0, split_pos));
        assert!(SplitDirection::None.should_show_at(0.5, split_pos));
        assert!(SplitDirection::None.should_show_at(1.0, split_pos));

        // Left shows at or before split
        assert!(SplitDirection::Left.should_show_at(0.0, split_pos));
        assert!(SplitDirection::Left.should_show_at(0.5, split_pos));
        assert!(!SplitDirection::Left.should_show_at(0.6, split_pos));

        // Right shows after split
        assert!(!SplitDirection::Right.should_show_at(0.0, split_pos));
        assert!(!SplitDirection::Right.should_show_at(0.5, split_pos));
        assert!(SplitDirection::Right.should_show_at(0.6, split_pos));
    }

    #[test]
    fn test_splitter_config_default() {
        let config = SplitterConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.split_position, 0.5);
    }

    #[test]
    fn test_splitter_config_new() {
        let config = SplitterConfig::new(true, 0.7);
        assert!(config.enabled);
        assert_eq!(config.split_position, 0.7);

        // Clamped
        let config2 = SplitterConfig::new(true, 1.5);
        assert_eq!(config2.split_position, 1.0);

        let config3 = SplitterConfig::new(true, -0.5);
        assert_eq!(config3.split_position, 0.0);
    }

    #[test]
    fn test_splitter_config_set_position() {
        let mut config = SplitterConfig::default();
        config.set_split_position(0.3);
        assert_eq!(config.split_position, 0.3);

        config.set_split_position(2.0);
        assert_eq!(config.split_position, 1.0);
    }

    #[test]
    fn test_splitter_config_pixels() {
        let config = SplitterConfig::new(true, 0.5);
        assert_eq!(config.split_position_pixels(1920.0), 960.0);
        assert_eq!(config.split_position_pixels(1080.0), 540.0);
    }

    #[test]
    fn test_splitter_shader_modification() {
        let disabled = SplitterConfig::default();
        assert_eq!(disabled.shader_modification(), "");

        let enabled = SplitterConfig::new(true, 0.5);
        let shader = enabled.shader_modification();
        assert!(shader.contains("czm_splitPosition"));
        assert!(shader.contains("discard"));
    }

    #[test]
    fn test_split_direction_serialization() {
        let dir = SplitDirection::Left;
        let json = serde_json::to_string(&dir).unwrap();
        let deserialized: SplitDirection = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, SplitDirection::Left);
    }
}
