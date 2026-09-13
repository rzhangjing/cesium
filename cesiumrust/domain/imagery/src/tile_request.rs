//! Imagery tile request calculation.
//!
//! Computes which imagery tiles need to be requested for a given terrain tile
//! based on the tiling scheme and layer configuration.

use cesium_geospatial::cartographic::Cartographic;
use cesium_geospatial::rectangle::Rectangle;
use cesium_geospatial::tiling_scheme::TilingScheme;

use crate::imagery_layer::ImageryLayer;

/// A request for an imagery tile.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageryTileRequest {
    /// The imagery layer ID.
    pub layer_id: u64,
    /// The tile X coordinate.
    pub x: u32,
    /// The tile Y coordinate.
    pub y: u32,
    /// The tile level.
    pub level: u32,
}

impl ImageryTileRequest {
    /// Creates a new imagery tile request.
    pub fn new(layer_id: u64, x: u32, y: u32, level: u32) -> Self {
        Self {
            layer_id,
            x,
            y,
            level,
        }
    }
}

/// Computes the imagery tile requests needed to cover a terrain tile rectangle.
///
/// # Arguments
/// * `layer` - The imagery layer configuration
/// * `terrain_rectangle` - The rectangle of the terrain tile
/// * `terrain_level` - The level of the terrain tile
/// * `tiling_scheme` - The tiling scheme used by the imagery provider
///
/// # Returns
/// A list of imagery tile requests that cover the terrain tile
pub fn compute_tile_requests(
    layer: &ImageryLayer,
    terrain_rectangle: &Rectangle,
    terrain_level: u32,
    tiling_scheme: &TilingScheme,
) -> Vec<ImageryTileRequest> {
    let mut requests = Vec::new();

    // Check if the layer is visible and the level is valid
    if !layer.show {
        return requests;
    }

    // Compute the imagery level to use
    // Typically imagery level matches terrain level, but can be clamped
    let imagery_level = terrain_level.clamp(layer.minimum_level, layer.maximum_level);

    // Check if the terrain rectangle intersects the layer rectangle
    let intersection = match terrain_rectangle.intersection(&layer.rectangle) {
        Some(rect) => rect,
        None => return requests,
    };

    // Get the tile range that covers the intersection rectangle
    // Clamp positions slightly inward to handle exact boundary cases
    let (num_x_tiles, num_y_tiles) = tiling_scheme.tiles_at_level(imagery_level);
    let scheme_rect = tiling_scheme.rectangle();
    let epsilon = 1e-12;

    let nw_lon = intersection.west.max(scheme_rect.west);
    let nw_lat = intersection.north.min(scheme_rect.north);
    let se_lon = intersection.east.min(scheme_rect.east - epsilon);
    let se_lat = intersection.south.max(scheme_rect.south + epsilon);

    let nw_carto = Cartographic::from_radians(nw_lon, nw_lat, 0.0);
    let se_carto = Cartographic::from_radians(se_lon, se_lat, 0.0);

    let (x_min, y_min) = tiling_scheme
        .position_to_tile(&nw_carto, imagery_level)
        .unwrap_or_default();
    let (x_max, y_max) = match tiling_scheme.position_to_tile(&se_carto, imagery_level) {
        Some(tile) => tile,
        None => (num_x_tiles.saturating_sub(1), num_y_tiles.saturating_sub(1)),
    };

    // Handle potential wrap-around or invalid coordinates
    let (x_min, x_max) = if x_min <= x_max {
        (x_min, x_max)
    } else {
        (x_max, x_min)
    };
    let (y_min, y_max) = if y_min <= y_max {
        (y_min, y_max)
    } else {
        (y_max, y_min)
    };

    // Clamp to valid tile range
    let x_min = x_min.min(num_x_tiles.saturating_sub(1));
    let x_max = x_max.min(num_x_tiles.saturating_sub(1));
    let y_min = y_min.min(num_y_tiles.saturating_sub(1));
    let y_max = y_max.min(num_y_tiles.saturating_sub(1));

    // Generate requests for all tiles in the range
    for y in y_min..=y_max {
        for x in x_min..=x_max {
            requests.push(ImageryTileRequest::new(layer.id, x, y, imagery_level));
        }
    }

    requests
}

/// Computes the texture coordinate mapping from a terrain tile to an imagery tile.
///
/// # Arguments
/// * `terrain_rectangle` - The rectangle of the terrain tile
/// * `imagery_rectangle` - The rectangle of the imagery tile
///
/// # Returns
/// A tuple of (translation, scale) for texture coordinate mapping
pub fn compute_texture_mapping(
    terrain_rectangle: &Rectangle,
    imagery_rectangle: &Rectangle,
) -> ([f64; 2], [f64; 2]) {
    let terrain_width = terrain_rectangle.width();
    let terrain_height = terrain_rectangle.height();
    let imagery_width = imagery_rectangle.width();
    let imagery_height = imagery_rectangle.height();

    // Compute scale: how much of the imagery tile the terrain tile covers
    let scale_x = terrain_width / imagery_width;
    let scale_y = terrain_height / imagery_height;

    // Compute translation: offset of terrain tile within imagery tile
    let translation_x = (terrain_rectangle.west - imagery_rectangle.west) / imagery_width;
    let translation_y = (terrain_rectangle.south - imagery_rectangle.south) / imagery_height;

    ([translation_x, translation_y], [scale_x, scale_y])
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_geospatial::ellipsoid::Ellipsoid;
    use cesium_geospatial::tiling_scheme::TilingScheme;

    fn create_geographic_tiling_scheme() -> TilingScheme {
        TilingScheme::geographic(Ellipsoid::WGS84)
    }

    #[test]
    fn test_compute_tile_requests() {
        let layer = ImageryLayer::new(1, Rectangle::MAX_VALUE);
        let terrain_rect = Rectangle::from_degrees(-180.0, -90.0, 0.0, 90.0);
        let tiling_scheme = create_geographic_tiling_scheme();

        let requests = compute_tile_requests(&layer, &terrain_rect, 0, &tiling_scheme);

        // At level 0, geographic tiling scheme has 2x1 tiles
        // The terrain rect covers the western half, so should request tile (0, 0)
        assert!(!requests.is_empty());
        assert!(requests.iter().all(|r| r.layer_id == 1));
    }

    #[test]
    fn test_compute_tile_requests_invisible_layer() {
        let layer = ImageryLayer::new(1, Rectangle::MAX_VALUE).with_show(false);
        let terrain_rect = Rectangle::from_degrees(-180.0, -90.0, 0.0, 90.0);
        let tiling_scheme = create_geographic_tiling_scheme();

        let requests = compute_tile_requests(&layer, &terrain_rect, 0, &tiling_scheme);

        assert!(requests.is_empty());
    }

    #[test]
    fn test_compute_tile_requests_level_clamping() {
        let layer = ImageryLayer::new(1, Rectangle::MAX_VALUE)
            .with_level_range(2, 5);
        let terrain_rect = Rectangle::from_degrees(-10.0, -10.0, 10.0, 10.0);
        let tiling_scheme = create_geographic_tiling_scheme();

        // Request at level 0 should be clamped to level 2
        let requests = compute_tile_requests(&layer, &terrain_rect, 0, &tiling_scheme);
        assert!(requests.iter().all(|r| r.level == 2));

        // Request at level 10 should be clamped to level 5
        let requests = compute_tile_requests(&layer, &terrain_rect, 10, &tiling_scheme);
        assert!(requests.iter().all(|r| r.level == 5));
    }

    #[test]
    fn test_compute_texture_mapping() {
        let terrain_rect = Rectangle::from_degrees(-90.0, -45.0, 0.0, 45.0);
        let imagery_rect = Rectangle::from_degrees(-180.0, -90.0, 0.0, 90.0);

        let (translation, scale) = compute_texture_mapping(&terrain_rect, &imagery_rect);

        // Terrain covers the eastern half of imagery in X
        assert!((translation[0] - 0.5).abs() < 1e-10);
        // Terrain covers the middle half of imagery in Y
        assert!((translation[1] - 0.25).abs() < 1e-10);
        // Scale should be 0.5 in both dimensions
        assert!((scale[0] - 0.5).abs() < 1e-10);
        assert!((scale[1] - 0.5).abs() < 1e-10);
    }
}
