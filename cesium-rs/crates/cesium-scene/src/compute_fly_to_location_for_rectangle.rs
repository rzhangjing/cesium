//! Ported from `packages/engine/Source/Scene/computeFlyToLocationForRectangle.js`
//! (75 lines).
//!
//! Computes the final camera location to view a rectangle adjusted for the
//! current terrain. If the terrain does not support availability, the height
//! above the ellipsoid is used.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartographic::Cartographic;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::rectangle::Rectangle;

use crate::scene_mode::SceneMode;

/// The scene surface required by
/// [`compute_fly_to_location_for_rectangle`].
///
/// DEVIATION: the JS function operates on a concrete `Scene`; the headless
/// port injects the scene surface through this trait (mirroring the JS
/// `_sampleTerrainMostDetailed` test seam for the terrain sampler).
pub trait FlyToRectangleScene {
    /// The current scene mode (`scene.mode`).
    fn mode(&self) -> SceneMode;
    /// The ellipsoid of the scene map projection
    /// (`scene.mapProjection.ellipsoid`).
    fn ellipsoid(&self) -> Ellipsoid;
    /// Mirrors `scene.mapProjection.unproject(cartesian)`.
    fn unproject(&self, cartesian: &Cartesian3) -> Cartographic;
    /// Mirrors `scene.camera.getRectangleCameraCoordinates(rectangle)`.
    fn get_rectangle_camera_coordinates(&self, rectangle: &Rectangle) -> Cartesian3;
    /// Whether a terrain provider is set (`defined(scene.terrainProvider)`).
    fn terrain_provider_defined(&self) -> bool;
    /// Whether the terrain provider has availability
    /// (`defined(terrainProvider.availability)`).
    fn terrain_availability_defined(&self) -> bool;
    /// The terrain sampler seam, mirroring the JS
    /// `_sampleTerrainMostDetailed(terrainProvider, positions)` test seam.
    /// Returns the sampled height (above the ellipsoid) for each input
    /// position; `None` mirrors an undefined sampled `height`.
    fn sample_terrain_most_detailed(&self, positions: &[Cartographic]) -> Vec<Option<f64>>;
}

/// Computes the final camera location to view a rectangle adjusted for the
/// current terrain. If the terrain does not support availability, the height
/// above the ellipsoid is used.
///
/// Port of `computeFlyToLocationForRectangle(rectangle, scene)`
/// (synchronous; the JS promise resolves after the terrain sampling seam).
pub fn compute_fly_to_location_for_rectangle(
    rectangle: &Rectangle,
    scene: &dyn FlyToRectangleScene,
) -> Cartographic {
    let ellipsoid = scene.ellipsoid();

    let tmp = scene.get_rectangle_camera_coordinates(rectangle);
    let mut position_without_terrain = Cartographic::default();
    if scene.mode() == SceneMode::Scene3D {
        // DEVIATION: the JS `cartesianToCartographic` returns `undefined`
        // when the point cannot be scaled to the surface; the port keeps
        // the zero cartographic in that case.
        ellipsoid.cartesian_to_cartographic(&tmp, &mut position_without_terrain);
    } else {
        position_without_terrain = scene.unproject(&tmp);
    }

    if !scene.terrain_provider_defined() {
        return position_without_terrain;
    }

    if !scene.terrain_availability_defined() || scene.mode() == SceneMode::Scene2D {
        return position_without_terrain;
    }

    let cartographics = [
        Rectangle::center(rectangle),
        Rectangle::southeast(rectangle),
        Rectangle::southwest(rectangle),
        Rectangle::northeast(rectangle),
        Rectangle::northwest(rectangle),
    ];

    let positions_on_terrain = scene.sample_terrain_most_detailed(&cartographics);

    let mut height_found = false;
    let mut max_height = f64::MIN; // JS: `-Number.MAX_VALUE`
    for item in &positions_on_terrain {
        let Some(height) = item else {
            continue;
        };
        height_found = true;
        max_height = height.max(max_height);
    }

    let mut final_position = position_without_terrain;
    if height_found {
        final_position.height += max_height;
    }

    final_position
}
