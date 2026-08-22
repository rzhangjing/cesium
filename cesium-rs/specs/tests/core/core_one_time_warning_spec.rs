//! Mirrors packages/engine/Specs/Core/oneTimeWarningSpec.js
//!
//! DEVIATION: the JS spec spies on `console.warn`; the Rust port captures
//! warnings through a replaceable sink
//! (`one_time_warning::set_warning_sink_for_specs`). Identifiers are made
//! unique per run because the dedup registry is process-wide. See
//! docs/deviations.md.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use cesium_core::one_time_warning::{one_time_warning, set_warning_sink_for_specs};
use cesium_specs::spec_serial_guard;
use cesium_test_utils::expect_to_throw_dev_error;

static RUN_COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique(prefix: &str) -> String {
    format!(
        "{prefix}-{}-{}",
        std::process::id(),
        RUN_COUNTER.fetch_add(1, Ordering::SeqCst)
    )
}

// describe("Core/oneTimeWarning")

#[test]
fn logs_a_warning() {
    let _serial = spec_serial_guard();

    let captured: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let sink_target = captured.clone();
    set_warning_sink_for_specs(Box::new(move |message: &str| {
        sink_target.lock().unwrap().push(message.to_owned());
    }));

    let identifier = unique("oneTime-identifier");
    let another_identifier = unique("another oneTime-identifier");

    one_time_warning(Some(&identifier), Some("message"));
    one_time_warning(Some(&identifier), None);
    one_time_warning(Some(&another_identifier), None);

    let calls = captured.lock().unwrap();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0], "message");
    assert_eq!(calls[1], another_identifier);
}

#[test]
fn throws_without_identifier() {
    expect_to_throw_dev_error(|| {
        one_time_warning(None, None);
    });
}
