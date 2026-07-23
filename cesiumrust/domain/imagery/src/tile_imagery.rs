//! Tile imagery association.
//! Maps to CesiumJS `Scene/TileImagery.js`

use cesium_geospatial::rectangle::Rectangle;
use serde::{Deserialize, Serialize};

use crate::imagery_state::ImageryState;

/// Represents the association between a terrain tile and an imagery tile.
///
/// This tracks the state of imagery loading for a specific terrain tile
/// and imagery layer combination.
/// Maps to CesiumJS `TileImagery`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileImagery {
    /// The imagery layer ID.
    pub layer_id: u64,

    /// The tile X coordinate.
    pub x: u32,

    /// The tile Y coordinate.
    pub y: u32,

    /// The tile level.
    pub level: u32,

    /// The current state of this imagery tile.
    pub state: ImageryState,

    /// The texture coordinate translation (for reprojection).
    pub texture_translation: [f64; 2],

    /// The texture coordinate scale (for reprojection).
    pub texture_scale: [f64; 2],

    /// The rectangle covered by this imagery tile.
    pub rectangle: Rectangle,

    /// Whether this imagery tile needs to be reprojected.
    pub needs_reprojection: bool,
}

impl TileImagery {
    /// Creates a new tile imagery association.
    pub fn new(layer_id: u64, x: u32, y: u32, level: u32, rectangle: Rectangle) -> Self {
        Self {
            layer_id,
            x,
            y,
            level,
            state: ImageryState::Unloaded,
            texture_translation: [0.0, 0.0],
            texture_scale: [1.0, 1.0],
            rectangle,
            needs_reprojection: false,
        }
    }

    /// Sets the state.
    pub fn with_state(mut self, state: ImageryState) -> Self {
        self.state = state;
        self
    }

    /// Sets the texture coordinates for reprojection.
    pub fn with_texture_coords(mut self, translation: [f64; 2], scale: [f64; 2]) -> Self {
        self.texture_translation = translation;
        self.texture_scale = scale;
        self.needs_reprojection = true;
        self
    }

    /// Marks this imagery as needing reprojection.
    pub fn set_needs_reprojection(&mut self, needs: bool) {
        self.needs_reprojection = needs;
    }

    /// Returns true if this imagery is ready to render.
    pub fn is_ready(&self) -> bool {
        self.state.is_renderable()
    }

    /// Returns true if a request should be made for this imagery.
    pub fn should_request(&self) -> bool {
        self.state.should_request()
    }

    /// Computes the texture coordinates for a given position within the tile.
    ///
    /// # Arguments
    /// * `u` - U coordinate (0.0 to 1.0) within the terrain tile
    /// * `v` - V coordinate (0.0 to 1.0) within the terrain tile
    ///
    /// # Returns
    /// The texture coordinates [u, v] for sampling the imagery texture
    pub fn compute_texture_coords(&self, u: f64, v: f64) -> [f64; 2] {
        [
            u * self.texture_scale[0] + self.texture_translation[0],
            v * self.texture_scale[1] + self.texture_translation[1],
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tile_imagery() {
        let tile = TileImagery::new(
            1,
            0,
            0,
            0,
            Rectangle::from_degrees(-180.0, -90.0, 0.0, 90.0),
        );

        assert_eq!(tile.layer_id, 1);
        assert_eq!(tile.x, 0);
        assert_eq!(tile.y, 0);
        assert_eq!(tile.level, 0);
        assert_eq!(tile.state, ImageryState::Unloaded);
    }

    #[test]
    fn test_state_transitions() {
        let mut tile = TileImagery::new(1, 0, 0, 0, Rectangle::MAX_VALUE);

        assert!(tile.should_request());
        assert!(!tile.is_ready());

        tile.state = ImageryState::Transitioning;
        assert!(!tile.should_request());
        assert!(!tile.is_ready());

        tile.state = ImageryState::Ready;
        assert!(!tile.should_request());
        assert!(tile.is_ready());
    }

    #[test]
    fn test_texture_coords() {
        let tile = TileImagery::new(1, 0, 0, 0, Rectangle::MAX_VALUE)
            .with_texture_coords([0.25, 0.25], [0.5, 0.5]);

        let coords = tile.compute_texture_coords(0.5, 0.5);
        assert!((coords[0] - 0.5).abs() < 1e-10); // 0.5 * 0.5 + 0.25
        assert!((coords[1] - 0.5).abs() < 1e-10);
    }
}
