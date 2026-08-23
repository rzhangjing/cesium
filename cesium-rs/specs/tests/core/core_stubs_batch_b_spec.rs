//! Tests for remaining stub modules (batch B): KTX, pipelines,
//! resource/scheduler, screen space, task processor, video, etc.

use cesium_core::ion_resource::IonResource;
use cesium_core::ktx2_transcoder::Ktx2Transcoder;
use cesium_core::load_image_from_typed_array::LoadImageFromTypedArray;
use cesium_core::load_ktx2::LoadKtx2;
use cesium_core::open_cage_geocoder_service::OpenCageGeocoderService;
use cesium_core::pelias_geocoder_service::PeliasGeocoderService;
use cesium_core::polygon_pipeline::PolygonPipeline;
use cesium_core::polyline_pipeline::PolylinePipeline;
use cesium_core::quantized_mesh_terrain_data::QuantizedMeshTerrainData;
use cesium_core::request_scheduler::RequestScheduler;
use cesium_core::sample_terrain::SampleTerrain;
use cesium_core::sample_terrain_most_detailed::SampleTerrainMostDetailed;
use cesium_core::screen_space_event_handler::ScreenSpaceEventHandler;
use cesium_core::task_processor::TaskProcessor;
use cesium_core::transcode_ktx2::TranscodeKtx2;
use cesium_core::vector_pipeline::VectorPipeline;
use cesium_core::vector_provider::VectorProvider;
use cesium_core::video_synchronizer::VideoSynchronizer;
use cesium_core::vr_the_world_terrain_provider::VRTHEWorldTerrainProvider;

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

// --- Geocoder services ---
#[test]
fn open_cage_geocoder_service_new() {
    let _ = OpenCageGeocoderService::new();
    let _ = OpenCageGeocoderService::default();
}

#[test]
fn pelias_geocoder_service_new() {
    let _ = PeliasGeocoderService::new();
    let _ = PeliasGeocoderService::default();
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
    let _ = QuantizedMeshTerrainData::new();
    let _ = QuantizedMeshTerrainData::default();
}

// --- Resource / scheduling ---
#[test]
fn ion_resource_new() {
    let _ = IonResource::new();
    let _ = IonResource::default();
}

#[test]
fn request_scheduler_new() {
    let _ = RequestScheduler::new();
    let _ = RequestScheduler::default();
}

// --- Terrain sampling ---
#[test]
fn sample_terrain_new() {
    let _ = SampleTerrain::new();
    let _ = SampleTerrain::default();
}

#[test]
fn sample_terrain_most_detailed_new() {
    let _ = SampleTerrainMostDetailed::new();
    let _ = SampleTerrainMostDetailed::default();
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

#[test]
fn vr_the_world_terrain_provider_new() {
    let _ = VRTHEWorldTerrainProvider::new();
    let _ = VRTHEWorldTerrainProvider::default();
}
