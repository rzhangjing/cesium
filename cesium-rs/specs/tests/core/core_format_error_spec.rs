//! Tests for `cesium_core::format_error`.

use cesium_core::developer_error::DeveloperError;
use cesium_core::format_error::{format_error, ErrorFields};
use cesium_core::runtime_error::RuntimeError;

#[test]
fn format_developer_error() {
    let err = DeveloperError {
        name: "DeveloperError",
        message: "test message".to_string(),
        stack: None,
    };
    let result = format_error(&err);
    assert!(result.contains("DeveloperError"));
    assert!(result.contains("test message"));
}

#[test]
fn format_runtime_error_with_stack() {
    let err = RuntimeError {
        name: "RuntimeError",
        message: "something failed".to_string(),
        stack: Some("at line 42".to_string()),
    };
    let result = format_error(&err);
    assert!(result.contains("RuntimeError"));
    assert!(result.contains("something failed"));
    assert!(result.contains("at line 42"));
}
