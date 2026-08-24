//! Mirror of `packages/engine/Specs/Core/CustomHeightmapTerrainProviderSpec.js`
//! (384 lines).
//!
//! The JS typed-array variants (`Int8Array` .. `Float64Array`) map to the
//! [`HeightmapBuffer`] variants; the JS `number[]` case (converted to
//! `Float64Array` inside `requestTileGeometry`) maps to
//! [`HeightmapBuffer::F64`] provided by the callback (module DEVIATION 1).

use cesium_core::custom_heightmap_terrain_provider::{
    CustomHeightmapTerrainProvider, CustomHeightmapTerrainProviderOptions,
};
use cesium_core::heightmap_terrain_data::{HeightmapBuffer, HeightmapBufferType};
use cesium_core::terrain_provider::{
    get_estimated_level_zero_geometric_error_for_a_heightmap, TerrainProvider,
};
use cesium_core::web_mercator_tiling_scheme::WebMercatorTilingScheme;

const WIDTH: usize = 2;
const HEIGHT: usize = 2;

/// The shared callback of most JS tests: returns a zero-filled typed array.
fn zero_callback(buffer_type: HeightmapBufferType) -> Box<dyn Fn(i32, i32, i32) -> Option<HeightmapBuffer>> {
    Box::new(move |_x, _y, _level| {
        Some(HeightmapBuffer::zeroed(buffer_type, WIDTH * HEIGHT))
    })
}

fn make_provider(options: CustomHeightmapTerrainProviderOptions) -> CustomHeightmapTerrainProvider {
    CustomHeightmapTerrainProvider::new(Some(options))
}

/// Mirrors "conforms to TerrainProvider interface".
#[test]
fn conforms_to_terrain_provider_interface() {
    fn assert_terrain_provider<T: TerrainProvider>(_: &T) {}
    let provider = make_provider(CustomHeightmapTerrainProviderOptions {
        callback: Some(zero_callback(HeightmapBufferType::Float32)),
        width: Some(WIDTH),
        height: Some(HEIGHT),
        ..Default::default()
    });
    assert_terrain_provider(&provider);
}

/// Mirrors "constructor throws if callback is not provided".
#[test]
#[should_panic(expected = "options.callback is required, actual value was undefined")]
fn constructor_throws_if_callback_is_not_provided() {
    let _ = make_provider(CustomHeightmapTerrainProviderOptions {
        width: Some(WIDTH),
        height: Some(HEIGHT),
        ..Default::default()
    });
}

/// Mirrors "constructor throws if width is not provided".
#[test]
#[should_panic(expected = "options.width is required, actual value was undefined")]
fn constructor_throws_if_width_is_not_provided() {
    let _ = make_provider(CustomHeightmapTerrainProviderOptions {
        callback: Some(zero_callback(HeightmapBufferType::Float32)),
        height: Some(HEIGHT),
        ..Default::default()
    });
}

/// Mirrors "constructor throws if height is not provided".
#[test]
#[should_panic(expected = "options.height is required, actual value was undefined")]
fn constructor_throws_if_height_is_not_provided() {
    let _ = make_provider(CustomHeightmapTerrainProviderOptions {
        callback: Some(zero_callback(HeightmapBufferType::Float32)),
        width: Some(WIDTH),
        ..Default::default()
    });
}

/// Mirrors "constructs with a credit".
#[test]
fn constructs_with_a_credit() {
    let provider = make_provider(CustomHeightmapTerrainProviderOptions {
        callback: Some(zero_callback(HeightmapBufferType::Float32)),
        width: Some(WIDTH),
        height: Some(HEIGHT),
        credit: Some("Credit".to_string()),
        ..Default::default()
    });
    assert!(provider.credit().is_some());
}

/// Mirrors "constructs with a tiling scheme" (the JS `toBeInstanceOf`
/// check is mirrored behaviorally: a WebMercator tiling scheme has one
/// level-zero tile in x, unlike the default geographic scheme's two).
#[test]
fn constructs_with_a_tiling_scheme() {
    let provider = make_provider(CustomHeightmapTerrainProviderOptions {
        callback: Some(zero_callback(HeightmapBufferType::Float32)),
        width: Some(WIDTH),
        height: Some(HEIGHT),
        tiling_scheme: Some(Box::new(WebMercatorTilingScheme::new(
            None, None, None, None, None,
        ))),
        ..Default::default()
    });
    assert_eq!(provider.tiling_scheme().get_number_of_x_tiles_at_level(0), 1);
    assert_eq!(provider.tiling_scheme().get_number_of_y_tiles_at_level(0), 1);
}

/// Mirrors "has error event".
#[test]
fn has_error_event() {
    let provider = make_provider(CustomHeightmapTerrainProviderOptions {
        callback: Some(zero_callback(HeightmapBufferType::Float32)),
        width: Some(WIDTH),
        height: Some(HEIGHT),
        ..Default::default()
    });
    // JS: `provider.errorEvent === provider.errorEvent` (same instance).
    assert!(std::ptr::eq(provider.error_event(), provider.error_event()));
}

/// Mirrors "gets geometric error".
#[test]
fn gets_geometric_error() {
    let provider = make_provider(CustomHeightmapTerrainProviderOptions {
        callback: Some(zero_callback(HeightmapBufferType::Float32)),
        width: Some(WIDTH),
        height: Some(HEIGHT),
        ..Default::default()
    });
    let geometric_error = get_estimated_level_zero_geometric_error_for_a_heightmap(
        provider.tiling_scheme().ellipsoid(),
        provider.width().max(provider.height()) as f64,
        provider.tiling_scheme().get_number_of_x_tiles_at_level(0),
    );
    assert_eq!(provider.get_level_maximum_geometric_error(0), geometric_error);
}

/// Mirrors "water mask is disabled".
#[test]
fn water_mask_is_disabled() {
    let provider = make_provider(CustomHeightmapTerrainProviderOptions {
        callback: Some(zero_callback(HeightmapBufferType::Float32)),
        width: Some(WIDTH),
        height: Some(HEIGHT),
        ..Default::default()
    });
    assert!(!provider.has_water_mask());
}

/// Mirrors "vertex normals are disabled".
#[test]
fn vertex_normals_are_disabled() {
    let provider = make_provider(CustomHeightmapTerrainProviderOptions {
        callback: Some(zero_callback(HeightmapBufferType::Float32)),
        width: Some(WIDTH),
        height: Some(HEIGHT),
        ..Default::default()
    });
    assert!(!provider.has_vertex_normals());
}

/// Mirrors the nine "requestTileGeometry receives heightmap data as ..."
/// tests (Int8/Uint8/Int16/Uint16/Int32/Uint32/Float32/Float64 typed arrays
/// and the JS `number[]` case, which JS converts to `Float64Array`).
#[test]
fn request_tile_geometry_receives_heightmap_data_of_every_buffer_type() {
    for buffer_type in [
        HeightmapBufferType::Int8,
        HeightmapBufferType::Uint8,
        HeightmapBufferType::Int16,
        HeightmapBufferType::Uint16,
        HeightmapBufferType::Int32,
        HeightmapBufferType::Uint32,
        HeightmapBufferType::Float32,
        HeightmapBufferType::Float64,
    ] {
        let provider = make_provider(CustomHeightmapTerrainProviderOptions {
            callback: Some(zero_callback(buffer_type)),
            width: Some(WIDTH),
            height: Some(HEIGHT),
            ..Default::default()
        });
        let terrain_data = provider.request_tile_geometry(0, 0, 0);
        assert!(terrain_data.is_some(), "typed-array case failed");
    }

    // The JS `number[]` callback case (converted to Float64Array).
    let provider = make_provider(CustomHeightmapTerrainProviderOptions {
        callback: Some(Box::new(|_x, _y, _level| {
            Some(HeightmapBuffer::F64(vec![0.0; WIDTH * HEIGHT]))
        })),
        width: Some(WIDTH),
        height: Some(HEIGHT),
        ..Default::default()
    });
    assert!(provider.request_tile_geometry(0, 0, 0).is_some());
}

/// Mirrors "requestTileGeometry returns undefined when callback function
/// returns undefined".
#[test]
fn request_tile_geometry_returns_none_when_callback_returns_none() {
    let provider = make_provider(CustomHeightmapTerrainProviderOptions {
        callback: Some(Box::new(|_x, _y, _level| None)),
        width: Some(WIDTH),
        height: Some(HEIGHT),
        ..Default::default()
    });
    assert!(provider.request_tile_geometry(0, 0, 0).is_none());
}

/// Mirrors "gets width and height".
#[test]
fn gets_width_and_height() {
    let width = 2usize;
    let height = 3usize;
    let provider = make_provider(CustomHeightmapTerrainProviderOptions {
        callback: Some(Box::new(move |_x, _y, _level| {
            Some(HeightmapBuffer::zeroed(
                HeightmapBufferType::Float32,
                width * height,
            ))
        })),
        width: Some(width),
        height: Some(height),
        ..Default::default()
    });
    assert_eq!(provider.width(), width);
    assert_eq!(provider.height(), height);
}

/// Mirrors "returns undefined for getTileDataAvailable".
#[test]
fn returns_none_for_get_tile_data_available() {
    let provider = make_provider(CustomHeightmapTerrainProviderOptions {
        callback: Some(zero_callback(HeightmapBufferType::Int16)),
        width: Some(WIDTH),
        height: Some(HEIGHT),
        ..Default::default()
    });
    assert_eq!(provider.get_tile_data_available(0, 0, 0), None);
}

/// Mirrors "returns undefined for loadTileDataAvailability".
#[test]
fn returns_none_for_load_tile_data_availability() {
    let provider = make_provider(CustomHeightmapTerrainProviderOptions {
        callback: Some(zero_callback(HeightmapBufferType::Int16)),
        width: Some(WIDTH),
        height: Some(HEIGHT),
        ..Default::default()
    });
    assert_eq!(provider.load_tile_data_availability(0, 0, 0), None);
}
