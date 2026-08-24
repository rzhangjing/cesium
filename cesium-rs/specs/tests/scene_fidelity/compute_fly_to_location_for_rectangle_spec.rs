//! Scene fidelity spec mirror for
//! `packages/engine/Specs/Scene/computeFlyToLocationForRectangleSpec.js`.
//!
//! DEVIATION: the JS spec drives a real `Scene` + `MockTerrainProvider` and
//! spies on `_sampleTerrainMostDetailed`; the Rust port injects the scene
//! surface through [`FlyToRectangleScene`] (the sampler seam mirrors the JS
//! `_sampleTerrainMostDetailed` test seam).

use std::cell::RefCell;

use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartographic::Cartographic;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::rectangle::Rectangle;
use cesium_scene::compute_fly_to_location_for_rectangle::{
    compute_fly_to_location_for_rectangle, FlyToRectangleScene,
};
use cesium_scene::scene_mode::SceneMode;

struct MockScene {
    mode: SceneMode,
    terrain_provider_defined: bool,
    availability_defined: bool,
    /// Mirrors `camera.getRectangleCameraCoordinates(rectangle)`.
    rectangle_position: Cartesian3,
    /// Mirrors `mapProjection.unproject`.
    unprojected: Cartographic,
    /// Heights returned by the mocked sampler (one per position; `None`
    /// mirrors an undefined sampled height).
    sampled_heights: Vec<Option<f64>>,
    /// Records every sampler invocation (mirrors `toHaveBeenCalledWith`).
    sampler_calls: RefCell<Vec<Vec<Cartographic>>>,
}

impl FlyToRectangleScene for MockScene {
    fn mode(&self) -> SceneMode {
        self.mode
    }

    fn ellipsoid(&self) -> Ellipsoid {
        Ellipsoid::WGS84
    }

    fn unproject(&self, _cartesian: &Cartesian3) -> Cartographic {
        self.unprojected
    }

    fn get_rectangle_camera_coordinates(&self, _rectangle: &Rectangle) -> Cartesian3 {
        self.rectangle_position
    }

    fn terrain_provider_defined(&self) -> bool {
        self.terrain_provider_defined
    }

    fn terrain_availability_defined(&self) -> bool {
        self.availability_defined
    }

    fn sample_terrain_most_detailed(&self, positions: &[Cartographic]) -> Vec<Option<f64>> {
        self.sampler_calls.borrow_mut().push(positions.to_vec());
        self.sampled_heights.clone()
    }
}

const RECTANGLE: Rectangle = Rectangle {
    west: 0.2,
    south: 0.4,
    east: 0.6,
    north: 0.8,
};

fn sample_scene(mode: SceneMode) -> MockScene {
    // Pretend we have terrain with availability. Mirrors the JS
    // `sampleTest` mock: same positions but with heights
    // [145, 1211, -123, 1234, undefined].
    let mut rectangle_position = Cartesian3::default();
    Ellipsoid::WGS84.cartographic_to_cartesian(
        &Cartographic {
            longitude: 0.5,
            latitude: 0.6,
            height: 0.0,
        },
        &mut rectangle_position,
    );
    MockScene {
        mode,
        terrain_provider_defined: true,
        availability_defined: true,
        rectangle_position,
        unprojected: Cartographic {
            longitude: 1.0,
            latitude: 2.0,
            height: 300.0,
        },
        sampled_heights: vec![Some(145.0), Some(1211.0), Some(-123.0), Some(1234.0), None],
        sampler_calls: RefCell::new(Vec::new()),
    }
}

fn expected_cartographics() -> Vec<Cartographic> {
    vec![
        Rectangle::center(&RECTANGLE),
        Rectangle::southeast(&RECTANGLE),
        Rectangle::southwest(&RECTANGLE),
        Rectangle::northeast(&RECTANGLE),
        Rectangle::northwest(&RECTANGLE),
    ]
}

#[test]
fn samples_terrain_and_returns_expected_result_in_3d() {
    let scene = sample_scene(SceneMode::Scene3D);

    let result = compute_fly_to_location_for_rectangle(&RECTANGLE, &scene);

    // Basically do the computation ourselves with our known values:
    let mut expected = Cartographic::default();
    Ellipsoid::WGS84.cartesian_to_cartographic(&scene.rectangle_position, &mut expected);
    expected.height += 1234.0; // maxHeight
    assert_eq!(result, expected);

    let calls = scene.sampler_calls.borrow();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0], expected_cartographics());
}

#[test]
fn samples_terrain_and_returns_expected_result_in_cv() {
    let scene = sample_scene(SceneMode::ColumbusView);

    let result = compute_fly_to_location_for_rectangle(&RECTANGLE, &scene);

    let mut expected = scene.unprojected;
    expected.height += 1234.0; // maxHeight
    assert_eq!(result, expected);

    let calls = scene.sampler_calls.borrow();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0], expected_cartographics());
}

#[test]
fn returns_height_above_ellipsoid_when_in_2d() {
    let scene = sample_scene(SceneMode::Scene2D);

    let result = compute_fly_to_location_for_rectangle(&RECTANGLE, &scene);

    assert_eq!(result, scene.unprojected);
    assert!(scene.sampler_calls.borrow().is_empty());
}

#[test]
fn returns_height_above_ellipsoid_when_terrain_not_available() {
    let mut scene = sample_scene(SceneMode::Scene3D);
    scene.availability_defined = false;

    let result = compute_fly_to_location_for_rectangle(&RECTANGLE, &scene);

    let mut expected = Cartographic::default();
    Ellipsoid::WGS84.cartesian_to_cartographic(&scene.rectangle_position, &mut expected);
    assert_eq!(result, expected);
    assert!(scene.sampler_calls.borrow().is_empty());
}

#[test]
fn returns_height_above_ellipsoid_when_terrain_undefined() {
    let mut scene = sample_scene(SceneMode::Scene3D);
    scene.terrain_provider_defined = false;

    let result = compute_fly_to_location_for_rectangle(&RECTANGLE, &scene);

    let mut expected = Cartographic::default();
    Ellipsoid::WGS84.cartesian_to_cartographic(&scene.rectangle_position, &mut expected);
    assert_eq!(result, expected);
    assert!(scene.sampler_calls.borrow().is_empty());
}
