//! Tests for remaining stub modules (batch B): KTX, pipelines,
//! resource/scheduler, screen space, task processor, video, etc.

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::ion_resource::{IonEndpoint, IonResource};
use cesium_core::ktx2_transcoder::Ktx2Transcoder;
use cesium_core::load_image_from_typed_array::LoadImageFromTypedArray;
use cesium_core::load_ktx2::LoadKtx2;
use cesium_core::open_cage_geocoder_service::OpenCageGeocoderService;
use cesium_core::pelias_geocoder_service::PeliasGeocoderService;
use cesium_core::polygon_pipeline::PolygonPipeline;
use cesium_core::polyline_pipeline::PolylinePipeline;
use cesium_core::quantized_mesh_terrain_data::{
    QuantizedMeshTerrainData, QuantizedMeshTerrainDataOptions,
};
use cesium_core::request_scheduler::RequestScheduler;
use cesium_core::ellipsoid_terrain_provider::EllipsoidTerrainProvider;
use cesium_core::sample_terrain::sample_terrain;
use cesium_core::sample_terrain_most_detailed::sample_terrain_most_detailed;
use cesium_core::screen_space_event_handler::ScreenSpaceEventHandler;
use cesium_core::task_processor::TaskProcessor;
use cesium_core::transcode_ktx2::TranscodeKtx2;
use cesium_core::vector_pipeline::VectorPipeline;
use cesium_core::vector_provider::VectorProvider;
use cesium_core::video_synchronizer::VideoSynchronizer;

// --- KTX / image loading ---
#[test]
fn ktx2_transcoder_new() {
    let _ = Ktx2Transcoder::new();
    let _ = Ktx2Transcoder::default();
}

#[test]
fn load_image_from_typed_array_new() {
    let _ = LoadImageFromTypedArray::new();
    let _ = LoadImageFromTypedArray::default();
}

#[test]
fn load_ktx2_new() {
    let _ = LoadKtx2::new();
    let _ = LoadKtx2::default();
}

// --- Geocoder services (substantiated by Track A7; construction smoke) ---
#[test]
fn open_cage_geocoder_service_new() {
    let _ = OpenCageGeocoderService::new(
        Some("https://api.opencagedata.com/geocode/v1/"),
        Some("key"),
        None,
    );
}

#[test]
fn pelias_geocoder_service_new() {
    let _ = PeliasGeocoderService::new(Some("http://test.invalid/v1/"));
}

// --- Pipelines ---
#[test]
fn polygon_pipeline_new() {
    let _ = PolygonPipeline::new();
    let _ = PolygonPipeline::default();
}

#[test]
fn polyline_pipeline_new() {
    let _ = PolylinePipeline::new();
    let _ = PolylinePipeline::default();
}

#[test]
fn vector_pipeline_new() {
    let _ = VectorPipeline::new();
    let _ = VectorPipeline::default();
}

// --- Terrain data ---
#[test]
fn quantized_mesh_terrain_data_new() {
    // DEVIATION: `QuantizedMeshTerrainData` is now substantiated and requires
    // an options object (mirrors the JS `options` parameter); the old
    // parameter-less stub constructor no longer exists.
    let _ = QuantizedMeshTerrainData::new(QuantizedMeshTerrainDataOptions {
        quantized_vertices: Some(vec![0u16; 9]),
        indices: Some(vec![0, 1, 2]),
        minimum_height: Some(0.0),
        maximum_height: Some(1.0),
        bounding_sphere: Some(BoundingSphere::default()),
        horizon_occlusion_point: Some(Cartesian3::default()),
        west_indices: Some(vec![]),
        south_indices: Some(vec![]),
        east_indices: Some(vec![]),
        north_indices: Some(vec![]),
        west_skirt_height: Some(1.0),
        south_skirt_height: Some(1.0),
        east_skirt_height: Some(1.0),
        north_skirt_height: Some(1.0),
        ..Default::default()
    });
}

// --- Resource / scheduling ---
#[test]
fn ion_resource_new() {
    // DEVIATION: CesiumJS `IonResource` has no public parameter-less
    // constructor; construction mirrors `new IonResource(endpoint,
    // endpointResource)` via `from_endpoint`.
    let endpoint = IonEndpoint {
        url: "https://assets.ion.cesium.com/1/tileset.json".to_string(),
        external_type: None,
        access_token: None,
        options_url: None,
    };
    let resource = IonResource::from_endpoint(endpoint).expect("from_endpoint");
    assert!(!resource.is_external());
    assert_eq!(resource.resource.retry_attempts(), 1);
}

#[test]
fn request_scheduler_new() {
    let _ = RequestScheduler::new();
    let _ = RequestScheduler::default();
}

// --- Terrain sampling ---
#[test]
fn sample_terrain_call() {
    // DEVIATION: sync port; call with empty positions.
    let provider = EllipsoidTerrainProvider::new(None, None);
    let mut positions = vec![];
    let result = sample_terrain(&provider, 0, &mut positions, false);
    assert!(result.is_ok());
}

#[test]
fn sample_terrain_most_detailed_call() {
    // DEVIATION: sync port; call with empty positions (no availability).
    let provider = EllipsoidTerrainProvider::new(None, None);
    let availability = cesium_core::tile_availability::TileAvailability::new(
        Box::new(cesium_core::geographic_tiling_scheme::GeographicTilingScheme::new(None, None, None, None)),
        0,
    );
    let mut positions = vec![];
    let result = sample_terrain_most_detailed(&provider, &availability, &mut positions, false);
    assert!(result.is_ok());
}

// --- UI / events ---
#[test]
fn screen_space_event_handler_new() {
    let _ = ScreenSpaceEventHandler::new();
    let _ = ScreenSpaceEventHandler::default();
}

// --- Processing ---
#[test]
fn task_processor_new() {
    let _ = TaskProcessor::new();
    let _ = TaskProcessor::default();
}

#[test]
fn transcode_ktx2_new() {
    let _ = TranscodeKtx2::new();
    let _ = TranscodeKtx2::default();
}

// --- Providers ---
#[test]
fn vector_provider_new() {
    let _ = VectorProvider::new();
    let _ = VectorProvider::default();
}

#[test]
fn video_synchronizer_new() {
    let _ = VideoSynchronizer::new();
    let _ = VideoSynchronizer::default();
}
