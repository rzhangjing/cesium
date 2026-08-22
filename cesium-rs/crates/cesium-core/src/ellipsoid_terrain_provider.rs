//! Ported from `packages/engine/Source/Core/EllipsoidTerrainProvider.js`.

use crate::ellipsoid::Ellipsoid;
use crate::geographic_tiling_scheme::GeographicTilingScheme;
use crate::tiling_scheme::TilingScheme;
use crate::terrain_provider;

/// A very simple terrain provider that produces geometry by tessellating an ellipsoidal surface.
pub struct EllipsoidTerrainProvider {
    _tiling_scheme: GeographicTilingScheme,
    level_zero_maximum_geometric_error: f64,
}

impl EllipsoidTerrainProvider {
    /// Creates a new EllipsoidTerrainProvider.
    pub fn new(
        tiling_scheme: Option<GeographicTilingScheme>,
        ellipsoid: Option<Ellipsoid>,
    ) -> Self {
        let tiling_scheme = tiling_scheme.unwrap_or_else(|| {
            GeographicTilingScheme::new(ellipsoid, None, None, None)
        });

        let level_zero_maximum_geometric_error =
            terrain_provider::get_estimated_level_zero_geometric_error_for_a_heightmap(
                tiling_scheme.ellipsoid(),
                64.0,
                tiling_scheme.get_number_of_x_tiles_at_level(0),
            );

        Self {
            _tiling_scheme: tiling_scheme,
            level_zero_maximum_geometric_error,
        }
    }

    /// Gets the maximum geometric error at a given level.
    pub fn get_level_maximum_geometric_error(&self, level: i32) -> f64 {
        self.level_zero_maximum_geometric_error / (1i64 << level) as f64
    }
}
