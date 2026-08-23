//! Tests for remaining modules: Request, TranslationRotationScale,
//! TrackingReferenceFrame, TrustedServers, UriJS, Ion, TerrainPicker,
//! and stub modules (fullscreen, create_color_ramp, etc.).

use cesium_core::cartesian3::Cartesian3;
use cesium_core::create_color_ramp::CreateColorRamp;
use cesium_core::default_proxy::DefaultProxy;
use cesium_core::ion;
use cesium_core::pin_builder::PinBuilder;
use cesium_core::quaternion::Quaternion;
use cesium_core::request::Request;
use cesium_core::request_state::RequestState;
use cesium_core::request_type::RequestType;
use cesium_core::terrain_picker::TerrainPicker;
use cesium_core::tracking_reference_frame::TrackingReferenceFrame;
use cesium_core::translation_rotation_scale::TranslationRotationScale;
use cesium_core::trusted_servers::TrustedServers;
// urijs module is private; test via public API if available
use cesium_core::write_text_to_canvas::WriteTextToCanvas;
use cesium_core::build_module_url::BuildModuleUrl;
use cesium_core::approximate_terrain_heights::ApproximateTerrainHeights;

// --- Request ---
#[test]
fn request_new_defaults() {
    let req = Request::new(None, None, None, None, None, None);
    assert!(req.url.is_none());
    assert_eq!(req.priority, 0.0);
    assert!(!req.throttle);
    assert!(!req.throttle_by_server);
    assert_eq!(req.request_type, RequestType::Other);
    assert_eq!(req.state, RequestState::Unissued);
    assert!(!req.cancelled);
}

#[test]
fn request_new_with_url() {
    let req = Request::new(Some("http://example.com".to_string()), Some(5.0), Some(true), None, Some(RequestType::Terrain), None);
    assert_eq!(req.url.as_deref(), Some("http://example.com"));
    assert_eq!(req.priority, 5.0);
    assert!(req.throttle);
    assert_eq!(req.request_type, RequestType::Terrain);
}

#[test]
fn request_cancel() {
    let mut req = Request::new(None, None, None, None, None, None);
    assert!(!req.cancelled);
    req.cancel();
    assert!(req.cancelled);
}

#[test]
fn request_clone_request() {
    let req = Request::new(Some("http://example.com".to_string()), Some(3.0), None, None, None, None);
    let cloned = req.clone_request();
    assert_eq!(cloned.url.as_deref(), Some("http://example.com"));
    assert_eq!(cloned.priority, 3.0);
    assert_eq!(cloned.state, RequestState::Unissued); // reset
}

// --- TranslationRotationScale ---
#[test]
fn trs_default() {
    let trs = TranslationRotationScale::default();
    assert_eq!(trs.translation, Cartesian3::ZERO);
    assert_eq!(trs.rotation, Quaternion::IDENTITY);
    assert_eq!(trs.scale, Cartesian3::new(1.0, 1.0, 1.0));
}

#[test]
fn trs_new_custom() {
    let t = Cartesian3::new(1.0, 2.0, 3.0);
    let r = Quaternion::new(0.0, 0.0, 0.0, 1.0);
    let s = Cartesian3::new(2.0, 2.0, 2.0);
    let trs = TranslationRotationScale::new(t, r, s);
    assert_eq!(trs.translation, t);
    assert_eq!(trs.scale, s);
}

#[test]
fn trs_equals() {
    let a = TranslationRotationScale::default();
    let b = TranslationRotationScale::default();
    assert!(a.equals(&b));
}

// --- TrackingReferenceFrame ---
#[test]
fn tracking_reference_frame_variants() {
    assert_eq!(TrackingReferenceFrame::Autodetect as i32, 0);
    assert_eq!(TrackingReferenceFrame::Enu as i32, 1);
    assert_eq!(TrackingReferenceFrame::Inertial as i32, 2);
    assert_eq!(TrackingReferenceFrame::Velocity as i32, 3);
}

// --- TrustedServers ---
#[test]
fn trusted_servers_add_and_check() {
    TrustedServers::reset();
    TrustedServers::add("http://example.com");
    assert!(TrustedServers::is_trusted("http://example.com"));
    assert!(!TrustedServers::is_trusted("http://other.com"));
    TrustedServers::reset();
}

#[test]
fn trusted_servers_remove() {
    TrustedServers::reset();
    TrustedServers::add("http://example.com");
    TrustedServers::remove("http://example.com");
    assert!(!TrustedServers::is_trusted("http://example.com"));
    TrustedServers::reset();
}

#[test]
fn trusted_servers_reset() {
    TrustedServers::add("http://test.com");
    TrustedServers::reset();
    assert!(!TrustedServers::is_trusted("http://test.com"));
}

// --- UriJS (module is private, tests deferred) ---

// --- Ion ---
#[test]
fn ion_default_access_token() {
    let token = ion::default_access_token();
    assert!(!token.is_empty());
}

#[test]
fn ion_default_server() {
    let server = ion::default_server();
    assert!(server.contains("cesium"));
}

// --- TerrainPicker ---
#[test]
fn terrain_picker_new() {
    let picker = TerrainPicker::new();
    assert!(picker.needs_rebuild);
}

#[test]
fn terrain_picker_default() {
    let picker = TerrainPicker::default();
    assert!(picker.needs_rebuild);
}

// --- Stub modules ---
#[test]
fn create_color_ramp_new() {
    let _ = CreateColorRamp::new();
    let _ = CreateColorRamp::default();
}

#[test]
fn write_text_to_canvas_new() {
    let _ = WriteTextToCanvas::new();
    let _ = WriteTextToCanvas::default();
}

#[test]
fn build_module_url_new() {
    let _ = BuildModuleUrl::new();
    let _ = BuildModuleUrl::default();
}

#[test]
fn pin_builder_new() {
    let _ = PinBuilder::new();
    let _ = PinBuilder::default();
}

#[test]
fn approximate_terrain_heights_new() {
    let _ = ApproximateTerrainHeights::new();
    let _ = ApproximateTerrainHeights::default();
}
