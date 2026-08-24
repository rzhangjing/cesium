//! Mirrors of the CesiumJS terrain-data Jasmine specs (Track A4 batch):
//!
//! - `packages/engine/Specs/Core/HeightmapTerrainDataSpec.js`      (447 lines)
//! - `packages/engine/Specs/Core/ApproximateTerrainHeightsSpec.js` (90 lines)
//! - `packages/engine/Specs/Core/QuantizedMeshTerrainDataSpec.js`  (pure-logic subset)
//!
//! Conventions:
//! - Jasmine `it(...)` titles map to `#[test] fn` names (snake_case).
//! - `toEqualEpsilon` -> `assert_epsilon` (EPSILON8).
//! - `toThrowDeveloperError` -> `#[should_panic]` (debug builds).
//!
//! DEVIATIONS (mirroring notes):
//! - The JS `toConformToInterface(TerrainData)` checks are guaranteed by the
//!   Rust `TerrainData` trait impls and are not mirrored as tests.
//! - JS `createMesh({ tilingScheme: undefined, ... })` / `x: undefined` etc.
//!   debug checks are compile-time guaranteed in Rust (required fields /
//!   `&dyn TilingScheme`), so the `requires tilingScheme/x/y/level` cases for
//!   `createMesh` have no Rust counterpart.
//! - The upsample flow tests pass `throttle: Some(false)` explicitly: JS runs
//!   specs sequentially, while Rust test threads run in parallel, so sharing
//!   the 5-slot throttle pool across tests would be racy.
//! - `QuantizedMeshTerrainData.createMesh` / `upsample` depend on the
//!   `createVerticesFromQuantizedTerrainMesh` / `upsampleQuantizedTerrainMesh`
//!   workers, which are materialized by the Globe terrain batch
//!   (Track B4-3/4/5); those spec cases are `#[ignore]`d placeholders.

use std::panic::AssertUnwindSafe;
use std::pin::Pin;

use cesium_core::approximate_terrain_heights;
use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::geographic_tiling_scheme::GeographicTilingScheme;
use cesium_core::heightmap_encoding::HeightmapEncoding;
use cesium_core::heightmap_terrain_data::{
    CreateMeshOptions, HeightmapBuffer, HeightmapBufferType, HeightmapStructureOptions,
    HeightmapTerrainData, HeightmapTerrainDataOptions,
};
use cesium_core::math::CesiumMath;
use cesium_core::quantized_mesh_terrain_data::{
    QuantizedMeshTerrainData, QuantizedMeshTerrainDataOptions,
};
use cesium_core::rectangle::Rectangle;
use cesium_core::terrain_data::TerrainData;
use cesium_core::tiling_scheme::TilingScheme;

fn assert_epsilon(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= CesiumMath::EPSILON8,
        "expected {expected}, got {actual}"
    );
}

fn buffer_values(buffer: &HeightmapBuffer) -> Vec<f64> {
    (0..buffer.len()).map(|i| buffer.get(i)).collect()
}

// ════════════════════════════════════════════════════════════════════════
// Core/HeightmapTerrainData
// ════════════════════════════════════════════════════════════════════════

// ── constructor ──────────────────────────────────────────────────────────

#[test]
#[should_panic]
fn heightmap_constructor_requires_buffer() {
    // `new HeightmapTerrainData()` and `{ width: 5, height: 5 }` both throw.
    let _ = HeightmapTerrainData::new(HeightmapTerrainDataOptions::default());
}

#[test]
#[should_panic]
fn heightmap_constructor_requires_buffer_when_width_height_given() {
    let _ = HeightmapTerrainData::new(HeightmapTerrainDataOptions {
        width: Some(5),
        height: Some(5),
        ..Default::default()
    });
}

#[test]
#[should_panic]
fn heightmap_constructor_requires_width() {
    let _ = HeightmapTerrainData::new(HeightmapTerrainDataOptions {
        buffer: Some(HeightmapBuffer::F32(vec![0.0; 25])),
        height: Some(5),
        ..Default::default()
    });
}

#[test]
#[should_panic]
fn heightmap_constructor_requires_height() {
    let _ = HeightmapTerrainData::new(HeightmapTerrainDataOptions {
        buffer: Some(HeightmapBuffer::F32(vec![0.0; 25])),
        width: Some(5),
        ..Default::default()
    });
}

#[test]
fn heightmap_constructor_non_lerc_encoded_buffers_set_correct_buffer_type() {
    let data = HeightmapTerrainData::new(HeightmapTerrainDataOptions {
        buffer: Some(HeightmapBuffer::U16(vec![0; 25])),
        width: Some(5),
        height: Some(5),
        ..Default::default()
    });

    assert_eq!(data.encoding(), HeightmapEncoding::None);
    assert_eq!(data.buffer_type(), HeightmapBufferType::Uint16);
}

#[test]
fn heightmap_constructor_lerc_encoded_buffers_set_correct_buffer_type() {
    let data = HeightmapTerrainData::new(HeightmapTerrainDataOptions {
        buffer: Some(HeightmapBuffer::U16(vec![0; 25])),
        width: Some(5),
        height: Some(5),
        encoding: Some(HeightmapEncoding::Lerc),
        ..Default::default()
    });

    assert_eq!(data.encoding(), HeightmapEncoding::Lerc);
    assert_eq!(data.buffer_type(), HeightmapBufferType::Float32);
}

// ── createMesh ───────────────────────────────────────────────────────────

fn create_sample_terrain_data() -> HeightmapTerrainData {
    HeightmapTerrainData::new(HeightmapTerrainDataOptions {
        buffer: Some(HeightmapBuffer::F32(vec![0.0; 25])),
        width: Some(5),
        height: Some(5),
        ..Default::default()
    })
}

#[tokio::test]
async fn heightmap_create_mesh_enables_throttling_for_asynchronous_tasks() {
    let tiling_scheme = GeographicTilingScheme::new(None, None, None, None);
    let task_count = <HeightmapTerrainData as TerrainData>::MAXIMUM_ASYNCHRONOUS_TASKS + 1;

    let mut datas: Vec<HeightmapTerrainData> =
        (0..task_count).map(|_| create_sample_terrain_data()).collect();

    let mut promises: Vec<Pin<Box<dyn std::future::Future<Output = ()>>>> = Vec::new();
    for data in datas.iter_mut() {
        let future = data.create_mesh(CreateMeshOptions {
            tiling_scheme: &tiling_scheme,
            x: 0,
            y: 0,
            level: 0,
            exaggeration: None,
            exaggeration_relative_height: None,
            throttle: Some(true),
        });
        if let Some(future) = future {
            promises.push(Box::pin(future));
        }
    }

    assert_eq!(
        promises.len(),
        <HeightmapTerrainData as TerrainData>::MAXIMUM_ASYNCHRONOUS_TASKS
    );

    // `Promise.all(promises)`
    for promise in promises {
        promise.await;
    }
}

#[tokio::test]
async fn heightmap_create_mesh_disables_throttling_for_asynchronous_tasks() {
    let tiling_scheme = GeographicTilingScheme::new(None, None, None, None);
    let task_count = <HeightmapTerrainData as TerrainData>::MAXIMUM_ASYNCHRONOUS_TASKS + 1;

    let mut datas: Vec<HeightmapTerrainData> =
        (0..task_count).map(|_| create_sample_terrain_data()).collect();

    let mut promises: Vec<Pin<Box<dyn std::future::Future<Output = ()>>>> = Vec::new();
    for data in datas.iter_mut() {
        let future = data.create_mesh(CreateMeshOptions {
            tiling_scheme: &tiling_scheme,
            x: 0,
            y: 0,
            level: 0,
            exaggeration: None,
            exaggeration_relative_height: None,
            throttle: Some(false),
        });
        if let Some(future) = future {
            promises.push(Box::pin(future));
        }
    }

    assert_eq!(promises.len(), task_count);

    for promise in promises {
        promise.await;
    }
}

// ── upsample ─────────────────────────────────────────────────────────────

fn create_upsample_base_data() -> HeightmapTerrainData {
    HeightmapTerrainData::new(HeightmapTerrainDataOptions {
        buffer: Some(HeightmapBuffer::F32(vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0,
        ])),
        width: Some(3),
        height: Some(3),
        ..Default::default()
    })
}

#[test]
#[should_panic]
fn heightmap_upsample_requires_tiling_scheme() {
    let data = create_upsample_base_data();
    data.upsample(None, Some(0), Some(0), Some(0), Some(0), Some(0), Some(0));
}

#[test]
#[should_panic]
fn heightmap_upsample_requires_this_x() {
    let tiling_scheme = GeographicTilingScheme::new(None, None, None, None);
    let data = create_upsample_base_data();
    data.upsample(Some(&tiling_scheme), None, Some(0), Some(0), Some(0), Some(0), Some(0));
}

#[test]
#[should_panic]
fn heightmap_upsample_requires_this_y() {
    let tiling_scheme = GeographicTilingScheme::new(None, None, None, None);
    let data = create_upsample_base_data();
    data.upsample(Some(&tiling_scheme), Some(0), None, Some(0), Some(0), Some(0), Some(0));
}

#[test]
#[should_panic]
fn heightmap_upsample_requires_this_level() {
    let tiling_scheme = GeographicTilingScheme::new(None, None, None, None);
    let data = create_upsample_base_data();
    data.upsample(Some(&tiling_scheme), Some(0), Some(0), None, Some(0), Some(0), Some(0));
}

#[test]
#[should_panic]
fn heightmap_upsample_requires_descendant_x() {
    let tiling_scheme = GeographicTilingScheme::new(None, None, None, None);
    let data = create_upsample_base_data();
    data.upsample(Some(&tiling_scheme), Some(0), Some(0), Some(0), None, Some(0), Some(0));
}

#[test]
#[should_panic]
fn heightmap_upsample_requires_descendant_y() {
    let tiling_scheme = GeographicTilingScheme::new(None, None, None, None);
    let data = create_upsample_base_data();
    data.upsample(Some(&tiling_scheme), Some(0), Some(0), Some(0), Some(0), None, Some(0));
}

#[test]
#[should_panic]
fn heightmap_upsample_requires_descendant_level() {
    let tiling_scheme = GeographicTilingScheme::new(None, None, None, None);
    let data = create_upsample_base_data();
    data.upsample(Some(&tiling_scheme), Some(0), Some(0), Some(0), Some(0), Some(0), None);
}

#[test]
#[should_panic]
fn heightmap_upsample_can_only_upsample_cross_one_level() {
    let tiling_scheme = GeographicTilingScheme::new(None, None, None, None);
    let data = create_upsample_base_data();
    data.upsample(Some(&tiling_scheme), Some(0), Some(0), Some(0), Some(0), Some(0), Some(2));
}

/// Runs `createMesh` (unthrottled, see module DEVIATION) then upsamples to
/// the western child of level 1.
async fn create_mesh_then_upsample(
    data: &mut HeightmapTerrainData,
    tiling_scheme: &GeographicTilingScheme,
    descendant_x: i32,
) -> HeightmapTerrainData {
    {
        let future = data.create_mesh(CreateMeshOptions {
            tiling_scheme,
            x: 0,
            y: 0,
            level: 0,
            exaggeration: None,
            exaggeration_relative_height: None,
            throttle: Some(false),
        });
        if let Some(future) = future {
            future.await;
        }
    }
    data.upsample(
        Some(tiling_scheme as &dyn TilingScheme),
        Some(0),
        Some(0),
        Some(0),
        Some(descendant_x),
        Some(0),
        Some(1),
    )
    .expect("upsample should succeed after createMesh")
}

#[tokio::test]
async fn heightmap_upsamples() {
    let tiling_scheme = GeographicTilingScheme::new(None, None, None, None);
    let mut data = HeightmapTerrainData::new(HeightmapTerrainDataOptions {
        buffer: Some(HeightmapBuffer::F32(vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
            16.0,
        ])),
        width: Some(4),
        height: Some(4),
        ..Default::default()
    });

    let upsampled = create_mesh_then_upsample(&mut data, &tiling_scheme, 0).await;

    assert!(upsampled.was_created_by_upsampling());
    assert_eq!(upsampled.width(), 4);
    assert_eq!(upsampled.height(), 4);
    assert_eq!(
        buffer_values(upsampled.buffer().unwrap()),
        vec![
            1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 4.5, 5.0, 5.5, 6.0, 6.5, 7.0, 7.5, 8.0, 8.5,
        ]
    );
}

#[tokio::test]
async fn heightmap_upsample_works_with_a_stride() {
    let tiling_scheme = GeographicTilingScheme::new(None, None, None, None);
    let mut data = HeightmapTerrainData::new(HeightmapTerrainDataOptions {
        buffer: Some(HeightmapBuffer::U8(vec![
            1, 1, 10, 2, 1, 10, 3, 1, 10, 4, 1, 10, 5, 1, 10, 6, 1, 10, 7, 1, 10, 8, 1, 10,
            9, 1, 10, 10, 1, 10, 11, 1, 10, 12, 1, 10, 13, 1, 10, 14, 1, 10, 15, 1, 10, 16,
            1, 10,
        ])),
        width: Some(4),
        height: Some(4),
        structure: Some(HeightmapStructureOptions {
            stride: Some(3),
            elements_per_height: Some(2),
            ..Default::default()
        }),
        ..Default::default()
    });

    let upsampled = create_mesh_then_upsample(&mut data, &tiling_scheme, 0).await;

    assert!(upsampled.was_created_by_upsampling());
    assert_eq!(upsampled.width(), 4);
    assert_eq!(upsampled.height(), 4);
    assert_eq!(
        buffer_values(upsampled.buffer().unwrap()),
        vec![
            1.0, 1.0, 0.0, 1.0, 1.0, 0.0, 2.0, 1.0, 0.0, 2.0, 1.0, 0.0, 3.0, 1.0, 0.0, 3.0,
            1.0, 0.0, 4.0, 1.0, 0.0, 4.0, 1.0, 0.0, 5.0, 1.0, 0.0, 5.0, 1.0, 0.0, 6.0, 1.0,
            0.0, 6.0, 1.0, 0.0, 7.0, 1.0, 0.0, 7.0, 1.0, 0.0, 8.0, 1.0, 0.0, 8.0, 1.0, 0.0,
        ]
    );
}

#[tokio::test]
async fn heightmap_upsample_works_with_a_big_endian_stride() {
    let tiling_scheme = GeographicTilingScheme::new(None, None, None, None);
    let mut data = HeightmapTerrainData::new(HeightmapTerrainDataOptions {
        buffer: Some(HeightmapBuffer::U8(vec![
            1, 1, 10, 1, 2, 10, 1, 3, 10, 1, 4, 10, 1, 5, 10, 1, 6, 10, 1, 7, 10, 1, 8, 10,
            1, 9, 10, 1, 10, 10, 1, 11, 10, 1, 12, 10, 1, 13, 10, 1, 14, 10, 1, 15, 10, 1,
            16, 10,
        ])),
        width: Some(4),
        height: Some(4),
        structure: Some(HeightmapStructureOptions {
            stride: Some(3),
            elements_per_height: Some(2),
            is_big_endian: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    });

    let upsampled = create_mesh_then_upsample(&mut data, &tiling_scheme, 0).await;

    assert!(upsampled.was_created_by_upsampling());
    assert_eq!(upsampled.width(), 4);
    assert_eq!(upsampled.height(), 4);
    assert_eq!(
        buffer_values(upsampled.buffer().unwrap()),
        vec![
            1.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 2.0, 0.0, 1.0, 2.0, 0.0, 1.0, 3.0, 0.0, 1.0,
            3.0, 0.0, 1.0, 4.0, 0.0, 1.0, 4.0, 0.0, 1.0, 5.0, 0.0, 1.0, 5.0, 0.0, 1.0, 6.0,
            0.0, 1.0, 6.0, 0.0, 1.0, 7.0, 0.0, 1.0, 7.0, 0.0, 1.0, 8.0, 0.0, 1.0, 8.0, 0.0,
        ]
    );
}

#[tokio::test]
async fn heightmap_upsample_works_for_an_eastern_child() {
    let tiling_scheme = GeographicTilingScheme::new(None, None, None, None);
    let mut data = HeightmapTerrainData::new(HeightmapTerrainDataOptions {
        buffer: Some(HeightmapBuffer::F32(vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
            16.0,
        ])),
        width: Some(4),
        height: Some(4),
        ..Default::default()
    });

    let upsampled = create_mesh_then_upsample(&mut data, &tiling_scheme, 1).await;

    assert!(upsampled.was_created_by_upsampling());
    assert_eq!(upsampled.width(), 4);
    assert_eq!(upsampled.height(), 4);
    assert_eq!(
        buffer_values(upsampled.buffer().unwrap()),
        vec![
            2.5, 3.0, 3.5, 4.0, 4.5, 5.0, 5.5, 6.0, 6.5, 7.0, 7.5, 8.0, 8.5, 9.0, 9.5, 10.0,
        ]
    );
}

#[tokio::test]
async fn heightmap_upsample_works_with_a_stride_for_an_eastern_child() {
    let tiling_scheme = GeographicTilingScheme::new(None, None, None, None);
    let mut data = HeightmapTerrainData::new(HeightmapTerrainDataOptions {
        buffer: Some(HeightmapBuffer::U8(vec![
            1, 1, 10, 2, 1, 10, 3, 1, 10, 4, 1, 10, 5, 1, 10, 6, 1, 10, 7, 1, 10, 8, 1, 10,
            9, 1, 10, 10, 1, 10, 11, 1, 10, 12, 1, 10, 13, 1, 10, 14, 1, 10, 15, 1, 10, 16,
            1, 10,
        ])),
        width: Some(4),
        height: Some(4),
        structure: Some(HeightmapStructureOptions {
            stride: Some(3),
            elements_per_height: Some(2),
            ..Default::default()
        }),
        ..Default::default()
    });

    let upsampled = create_mesh_then_upsample(&mut data, &tiling_scheme, 1).await;

    assert!(upsampled.was_created_by_upsampling());
    assert_eq!(upsampled.width(), 4);
    assert_eq!(upsampled.height(), 4);
    assert_eq!(
        buffer_values(upsampled.buffer().unwrap()),
        vec![
            2.0, 1.0, 0.0, 3.0, 1.0, 0.0, 3.0, 1.0, 0.0, 4.0, 1.0, 0.0, 4.0, 1.0, 0.0, 5.0,
            1.0, 0.0, 5.0, 1.0, 0.0, 6.0, 1.0, 0.0, 6.0, 1.0, 0.0, 7.0, 1.0, 0.0, 7.0, 1.0,
            0.0, 8.0, 1.0, 0.0, 8.0, 1.0, 0.0, 9.0, 1.0, 0.0, 9.0, 1.0, 0.0, 10.0, 1.0, 0.0,
        ]
    );
}

#[tokio::test]
async fn heightmap_upsample_clamps_out_of_range_data() {
    let tiling_scheme = GeographicTilingScheme::new(None, None, None, None);
    let mut data = HeightmapTerrainData::new(HeightmapTerrainDataOptions {
        buffer: Some(HeightmapBuffer::F32(vec![
            -1.0, -2.0, -3.0, -4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0,
            15.0, 16.0,
        ])),
        width: Some(4),
        height: Some(4),
        structure: Some(HeightmapStructureOptions {
            stride: Some(1),
            elements_per_height: Some(1),
            lowest_encoded_height: Some(1.0),
            highest_encoded_height: Some(7.0),
            ..Default::default()
        }),
        ..Default::default()
    });

    let upsampled = create_mesh_then_upsample(&mut data, &tiling_scheme, 0).await;

    assert!(upsampled.was_created_by_upsampling());
    assert_eq!(upsampled.width(), 4);
    assert_eq!(upsampled.height(), 4);
    assert_eq!(
        buffer_values(upsampled.buffer().unwrap()),
        vec![1.0, 1.0, 1.0, 1.0, 2.0, 1.5, 2.0, 1.5, 5.0, 5.5, 6.0, 6.5, 7.0, 7.0, 7.0, 7.0,]
    );
}

// ── isChildAvailable ─────────────────────────────────────────────────────

#[test]
#[should_panic]
fn heightmap_is_child_available_requires_this_x() {
    let data = create_upsample_base_data();
    data.is_child_available(None, Some(0), Some(0), Some(0));
}

#[test]
#[should_panic]
fn heightmap_is_child_available_requires_this_y() {
    let data = create_upsample_base_data();
    data.is_child_available(Some(0), None, Some(0), Some(0));
}

#[test]
#[should_panic]
fn heightmap_is_child_available_requires_child_x() {
    let data = create_upsample_base_data();
    data.is_child_available(Some(0), Some(0), None, Some(0));
}

#[test]
#[should_panic]
fn heightmap_is_child_available_requires_child_y() {
    let data = create_upsample_base_data();
    data.is_child_available(Some(0), Some(0), Some(0), None);
}

// ════════════════════════════════════════════════════════════════════════
// Core/ApproximateTerrainHeights
// ════════════════════════════════════════════════════════════════════════

#[test]
#[should_panic]
fn approximate_get_minimum_maximum_heights_throws_with_no_rectangle() {
    // The `Check.typeOf.object("rectangle", ...)` debug check fires before
    // the initialized check, so no initialization is needed here.
    let _ = approximate_terrain_heights::get_minimum_maximum_heights(None, None);
}

/// Sequential mirror of the remaining `ApproximateTerrainHeightsSpec` cases.
/// They share the module-level `_terrainHeights` slot (mirrored as a global
/// `Mutex<Option<HashMap>>`), so they are folded into a single test to avoid
/// parallel-test interference that the sequential Jasmine runner never had.
#[test]
fn approximate_terrain_heights_spec() {
    // it("initializes")
    approximate_terrain_heights::initialize()
        .expect("initialize resolves (asset is read from the CesiumJS source tree)");
    assert!(approximate_terrain_heights::initialized());

    let rectangle = Rectangle::from_degrees(-121.0, 10.0, -120.0, 11.0);

    // it("getMinimumMaximumHeights computes minimum and maximum terrain heights")
    let result = approximate_terrain_heights::get_minimum_maximum_heights(Some(&rectangle), None);
    assert_epsilon(result.minimum_terrain_height, -5269.86);
    assert_epsilon(result.maximum_terrain_height, -28.53);

    // it("getMinimumMaximumHeights throws if ApproximateTerrainHeights was
    //     not initialized first") — save/clear/restore the global slot, as
    //     the JS spec does with `_terrainHeights = undefined`.
    {
        let saved = std::mem::take(&mut *approximate_terrain_heights_slot_for_test());
        let panicked = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let _ = approximate_terrain_heights::get_minimum_maximum_heights(
                Some(&rectangle),
                None,
            );
        }))
        .is_err();
        *approximate_terrain_heights_slot_for_test() = saved;
        assert!(panicked, "expected DeveloperError while uninitialized");
    }

    // it("getBoundingSphere computes a bounding sphere")
    let result = approximate_terrain_heights::get_bounding_sphere(Some(&rectangle), None);
    assert_epsilon(result.center.x, -3183013.849117281);
    assert_epsilon(result.center.y, -5403772.559109628);
    assert_epsilon(result.center.z, 1154581.5821590829);
    assert_epsilon(result.radius, 77884.16321007285);

    // it("getBoundingSphere throws if ApproximateTerrainHeights was not
    //     initialized first")
    {
        let saved = std::mem::take(&mut *approximate_terrain_heights_slot_for_test());
        let panicked = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let _ = approximate_terrain_heights::get_bounding_sphere(Some(&rectangle), None);
        }))
        .is_err();
        *approximate_terrain_heights_slot_for_test() = saved;
        assert!(panicked, "expected DeveloperError while uninitialized");
    }
}

/// Helper mirroring the JS spec's direct `_terrainHeights` assignment.
fn approximate_terrain_heights_slot_for_test(
) -> std::sync::MutexGuard<'static, Option<std::collections::HashMap<String, [f64; 2]>>> {
    approximate_terrain_heights::terrain_heights_slot_for_test()
}

// ════════════════════════════════════════════════════════════════════════
// Core/QuantizedMeshTerrainData
// ════════════════════════════════════════════════════════════════════════

/// Mirrors the shared fixture of the `isChildAvailable` describe block
/// (`childTileMask` overridable; `None` mirrors the omitted option).
fn quantized_quad_options(child_tile_mask: Option<i32>) -> QuantizedMeshTerrainDataOptions {
    QuantizedMeshTerrainDataOptions {
        quantized_vertices: Some(vec![
            // order is sw nw se ne
            // u
            0, 0, 32767, 32767, // v
            0, 32767, 0, 32767, // heights
            16384, 0, 32767, 16384,
        ]),
        indices: Some(vec![0, 3, 1, 0, 2, 3]),
        minimum_height: Some(-16384.0),
        maximum_height: Some(16383.0),
        bounding_sphere: Some(BoundingSphere::new(Cartesian3::default(), 0.0)),
        oriented_bounding_box: None,
        horizon_occlusion_point: Some(Cartesian3::default()),
        west_indices: Some(vec![0, 1]),
        south_indices: Some(vec![0, 1]),
        east_indices: Some(vec![2, 3]),
        north_indices: Some(vec![1, 3]),
        west_skirt_height: Some(1.0),
        south_skirt_height: Some(1.0),
        east_skirt_height: Some(1.0),
        north_skirt_height: Some(1.0),
        child_tile_mask,
        created_by_upsampling: None,
        encoded_normals: None,
        water_mask: None,
        credits: None,
    }
}

// ── interpolateHeight ────────────────────────────────────────────────────

fn quantized_clamp_mesh() -> QuantizedMeshTerrainData {
    // Same quad as above, but with heights mirroring the JS spec's
    // `new Uint16Array([32767/4, 2*32767/4, 3*32767/4, 32767])` truncation.
    QuantizedMeshTerrainData::new(QuantizedMeshTerrainDataOptions {
        quantized_vertices: Some(vec![
            // u
            0, 0, 32767, 32767, // v
            0, 32767, 0, 32767, // heights (JS truncates to Uint16)
            8191, 16383, 24575, 32767,
        ]),
        indices: Some(vec![0, 3, 1, 0, 2, 3]),
        minimum_height: Some(0.0),
        maximum_height: Some(4.0),
        bounding_sphere: Some(BoundingSphere::new(Cartesian3::default(), 0.0)),
        oriented_bounding_box: None,
        horizon_occlusion_point: Some(Cartesian3::default()),
        west_indices: Some(vec![0, 1]),
        south_indices: Some(vec![0, 1]),
        east_indices: Some(vec![2, 3]),
        north_indices: Some(vec![1, 3]),
        west_skirt_height: Some(1.0),
        south_skirt_height: Some(1.0),
        east_skirt_height: Some(1.0),
        north_skirt_height: Some(1.0),
        child_tile_mask: Some(15),
        created_by_upsampling: None,
        encoded_normals: None,
        water_mask: None,
        credits: None,
    })
}

#[test]
fn quantized_interpolate_height_clamps_coordinates_outside_the_mesh() {
    let tiling_scheme = GeographicTilingScheme::new(None, None, None, None);
    let mut rectangle = Rectangle::default();
    tiling_scheme.tile_xy_to_rectangle(7, 6, 5, &mut rectangle);

    let mesh = quantized_clamp_mesh();

    assert_eq!(
        mesh.interpolate_height(&rectangle, 0.0, 0.0),
        mesh.interpolate_height(&rectangle, rectangle.east, rectangle.south)
    );
}

#[test]
fn quantized_interpolate_height_returns_a_height_interpolated_from_the_correct_triangle() {
    let tiling_scheme = GeographicTilingScheme::new(None, None, None, None);
    let mut rectangle = Rectangle::default();
    tiling_scheme.tile_xy_to_rectangle(7, 6, 5, &mut rectangle);

    // zero height along line between southwest and northeast corners.
    // Negative height in the northwest corner, positive height in the southeast.
    let mesh = QuantizedMeshTerrainData::new(quantized_quad_options(Some(15)));

    // position in the northwest quadrant of the tile.
    let mut longitude = rectangle.west + (rectangle.east - rectangle.west) * 0.25;
    let mut latitude = rectangle.south + (rectangle.north - rectangle.south) * 0.75;

    let result = mesh.interpolate_height(&rectangle, longitude, latitude).unwrap();
    assert!(result < 0.0);

    // position in the southeast quadrant of the tile.
    longitude = rectangle.west + (rectangle.east - rectangle.west) * 0.75;
    latitude = rectangle.south + (rectangle.north - rectangle.south) * 0.25;

    let result = mesh.interpolate_height(&rectangle, longitude, latitude).unwrap();
    assert!(result > 0.0);

    // position on the line between the southwest and northeast corners.
    longitude = rectangle.west + (rectangle.east - rectangle.west) * 0.5;
    latitude = rectangle.south + (rectangle.north - rectangle.south) * 0.5;

    let result = mesh.interpolate_height(&rectangle, longitude, latitude).unwrap();
    assert!(
        result.abs() <= 1e-10,
        "expected ~0.0, got {result}"
    );
}

// ── isChildAvailable ─────────────────────────────────────────────────────

#[test]
#[should_panic]
fn quantized_is_child_available_requires_this_x() {
    let data = QuantizedMeshTerrainData::new(quantized_quad_options(Some(15)));
    data.is_child_available(None, Some(0), Some(0), Some(0));
}

#[test]
#[should_panic]
fn quantized_is_child_available_requires_this_y() {
    let data = QuantizedMeshTerrainData::new(quantized_quad_options(Some(15)));
    data.is_child_available(Some(0), None, Some(0), Some(0));
}

#[test]
#[should_panic]
fn quantized_is_child_available_requires_child_x() {
    let data = QuantizedMeshTerrainData::new(quantized_quad_options(Some(15)));
    data.is_child_available(Some(0), Some(0), None, Some(0));
}

#[test]
#[should_panic]
fn quantized_is_child_available_requires_child_y() {
    let data = QuantizedMeshTerrainData::new(quantized_quad_options(Some(15)));
    data.is_child_available(Some(0), Some(0), Some(0), None);
}

#[test]
fn quantized_is_child_available_returns_true_for_all_children_when_mask_not_specified() {
    let data = QuantizedMeshTerrainData::new(quantized_quad_options(None));

    assert!(data.is_child_available(Some(10), Some(20), Some(20), Some(40)));
    assert!(data.is_child_available(Some(10), Some(20), Some(21), Some(40)));
    assert!(data.is_child_available(Some(10), Some(20), Some(20), Some(41)));
    assert!(data.is_child_available(Some(10), Some(20), Some(21), Some(41)));
}

#[test]
fn quantized_is_child_available_works_when_only_southwest_child_is_available() {
    let data = QuantizedMeshTerrainData::new(quantized_quad_options(Some(1)));

    assert!(!data.is_child_available(Some(10), Some(20), Some(20), Some(40)));
    assert!(!data.is_child_available(Some(10), Some(20), Some(21), Some(40)));
    assert!(data.is_child_available(Some(10), Some(20), Some(20), Some(41)));
    assert!(!data.is_child_available(Some(10), Some(20), Some(21), Some(41)));
}

#[test]
fn quantized_is_child_available_works_when_only_southeast_child_is_available() {
    let data = QuantizedMeshTerrainData::new(quantized_quad_options(Some(2)));

    assert!(!data.is_child_available(Some(10), Some(20), Some(20), Some(40)));
    assert!(!data.is_child_available(Some(10), Some(20), Some(21), Some(40)));
    assert!(!data.is_child_available(Some(10), Some(20), Some(20), Some(41)));
    assert!(data.is_child_available(Some(10), Some(20), Some(21), Some(41)));
}

#[test]
fn quantized_is_child_available_works_when_only_northwest_child_is_available() {
    let data = QuantizedMeshTerrainData::new(quantized_quad_options(Some(4)));

    assert!(data.is_child_available(Some(10), Some(20), Some(20), Some(40)));
    assert!(!data.is_child_available(Some(10), Some(20), Some(21), Some(40)));
    assert!(!data.is_child_available(Some(10), Some(20), Some(20), Some(41)));
    assert!(!data.is_child_available(Some(10), Some(20), Some(21), Some(41)));
}

#[test]
fn quantized_is_child_available_works_when_only_northeast_child_is_available() {
    let data = QuantizedMeshTerrainData::new(quantized_quad_options(Some(8)));

    assert!(!data.is_child_available(Some(10), Some(20), Some(20), Some(40)));
    assert!(data.is_child_available(Some(10), Some(20), Some(21), Some(40)));
    assert!(!data.is_child_available(Some(10), Some(20), Some(20), Some(41)));
    assert!(!data.is_child_available(Some(10), Some(20), Some(21), Some(41)));
}

// ── createMesh / upsample (worker-dependent, Track B4-3/4/5) ────────────

// DEVIATION: the remaining QuantizedMeshTerrainDataSpec cases — all 4
// `upsample` cases and the `createMesh` cases (skirt vertices, exaggeration,
// 32-bit indices, throttling) — exercise the
// `createVerticesFromQuantizedTerrainMesh` / `upsampleQuantizedTerrainMesh`
// workers (~1400 lines of triangle clipping), which are materialized by the
// Globe terrain batch (Track B4-3/4/5). They are mirrored here as ignored
// placeholders so they appear in the spec coverage ledger.

#[test]
#[ignore = "requires createVerticesFromQuantizedTerrainMesh worker (Track B4-3/4/5)"]
fn quantized_create_mesh_creates_specified_vertices_plus_skirt_vertices() {
    let tiling_scheme = GeographicTilingScheme::new(None, None, None, None);
    let mut data = quantized_clamp_mesh();
    data.create_mesh(cesium_core::quantized_mesh_terrain_data::CreateMeshOptions {
        tiling_scheme: &tiling_scheme,
        x: 0,
        y: 0,
        level: 0,
        exaggeration: None,
        exaggeration_relative_height: None,
        throttle: None,
    });
}

#[test]
#[ignore = "requires upsampleQuantizedTerrainMesh worker (Track B4-3/4/5)"]
fn quantized_upsample_works_for_all_four_children_of_a_simple_quad() {
    let tiling_scheme = GeographicTilingScheme::new(None, None, None, None);
    let data = QuantizedMeshTerrainData::new(quantized_quad_options(Some(15)));
    let _ = data.upsample(
        &tiling_scheme as &dyn TilingScheme,
        0,
        0,
        0,
        0,
        0,
        1,
    );
}
