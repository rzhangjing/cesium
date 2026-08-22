//! Mirrors packages/engine/Specs/Core/FeatureDetectionSpec.js
//!
//! The JS specs mostly assert `typeof result === "boolean"`; in Rust the
//! return types are statically `bool`, so each mirror calls the function and
//! uses the value where the JS spec branched on it.

use cesium_core::feature_detection::{self as fd, FeatureDetector};
use cesium_test_utils::expect_to_throw_dev_error;

// describe("Core/FeatureDetection")

#[test]
fn detects_fullscreen_support() {
    let supports_fullscreen: bool = fd::supports_fullscreen();
    let _ = supports_fullscreen; // typeof === "boolean" is static in Rust
}

#[test]
fn detects_web_worker_support() {
    let supports_web_workers: bool = fd::supports_web_workers();
    let _ = supports_web_workers;
}

#[test]
fn detects_typed_array_support() {
    let supports_typed_arrays: bool = fd::supports_typed_arrays();
    let _ = supports_typed_arrays;
}

#[test]
fn detects_big_int64_array_support() {
    let supports_big_int64_array: bool = fd::supports_big_int64_array();
    let _ = supports_big_int64_array;
}

#[test]
fn detects_big_uint64_array_support() {
    let supports_big_uint64_array: bool = fd::supports_big_uint64_array();
    let _ = supports_big_uint64_array;
}

#[test]
fn detects_big_int_support() {
    let supports_big_int: bool = fd::supports_big_int();
    let _ = supports_big_int;
}

#[test]
fn detects_web_assembly_support() {
    let supports_web_assembly: bool = fd::supports_web_assembly();
    let _ = supports_web_assembly;
}

// JS helper `checkVersionArray`: array of numbers — Vec<f64> in Rust.
fn check_version_array(array: &[f64]) {
    for d in array {
        let _: &f64 = d; // typeof d === "number" is static in Rust
    }
}

#[test]
fn detects_chrome() {
    let is_chrome: bool = fd::is_chrome();
    if is_chrome {
        let chrome_version = fd::chrome_version().expect("version after isChrome");
        check_version_array(&chrome_version);
    }
}

#[test]
fn detects_safari() {
    let is_safari: bool = fd::is_safari();
    if is_safari {
        let safari_version = fd::safari_version().expect("version after isSafari");
        check_version_array(&safari_version);
    }
}

#[test]
fn detects_webkit() {
    let is_webkit: bool = fd::is_webkit();
    if is_webkit {
        let webkit_version = fd::webkit_version().expect("version after isWebkit");
        check_version_array(&webkit_version.parts);
        let _: bool = webkit_version.is_nightly;
    }
}

#[test]
fn detects_edge() {
    let is_edge: bool = fd::is_edge();
    if is_edge {
        let edge_version = fd::edge_version().expect("version after isEdge");
        check_version_array(&edge_version);
    }
}

#[test]
fn detects_firefox() {
    let is_firefox: bool = fd::is_firefox();
    if is_firefox {
        let firefox_version = fd::firefox_version().expect("version after isFirefox");
        check_version_array(&firefox_version);
    }
}

#[test]
fn detects_ipad_or_ios() {
    let ipad_or_ios: bool = fd::is_ipad_or_ios();
    let _ = ipad_or_ios;
}

#[test]
fn detects_image_rendering_support() {
    let supports_image_rendering_pixelated = fd::supports_image_rendering_pixelated();
    if supports_image_rendering_pixelated {
        assert!(fd::image_rendering_value().is_some());
    } else {
        assert!(fd::image_rendering_value().is_none());
    }
}

// JS: the WebP tests mutate `supportsWebP._promise/_result` between two
// `it` blocks. They share one process-wide state machine, so the Rust
// mirror keeps both steps in a single test to stay order-independent.
#[tokio::test]
async fn supports_web_p_throws_when_not_initialized_then_detects_after_initialize() {
    // it("supportWebP throws when it has not been initialized")
    fd::reset_web_p_state_for_specs();
    expect_to_throw_dev_error(|| {
        let _ = fd::supports_web_p();
    });

    // it("detects WebP support")
    fd::reset_web_p_state_for_specs();
    let supports_web_p = fd::supports_web_p_initialize().await;
    let _: bool = supports_web_p;
    assert!(fd::supports_web_p_initialized());
    assert_eq!(fd::supports_web_p(), supports_web_p);
    fd::reset_web_p_state_for_specs();
}

#[test]
#[ignore = "requires a cesium-scene context (WebGL2 probe), ported in M3"]
fn detects_webgl2_support() {}

#[test]
fn detector_parses_chrome_user_agent() {
    // Supplementary native coverage for the injectable detector (the JS
    // suite runs only inside real browsers, so user-agent parsing is never
    // exercised there).
    let detector = FeatureDetector::new(
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36",
        "5.0 (Windows)",
        "Win32",
    );
    assert!(detector.is_chrome());
    assert_eq!(
        detector.chrome_version().as_deref(),
        Some(&[126.0, 0.0, 0.0, 0.0][..])
    );
    assert!(!detector.is_safari());
    assert!(detector.is_webkit());
    assert!(detector.is_windows());
}
