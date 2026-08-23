//! Integration tests for mock test workers.
//!
//! These mirror the CesiumJS `Specs/TestWorkers` and verify the worker
//! function contract (input → output semantics).

mod create_bad_geometry;
mod return_byte_length;
mod return_non_cloneable;
mod return_parameters;
mod return_wasm_config;
mod throw_error;
mod transfer_array_buffer;

#[test]
fn create_bad_geometry_returns_error() {
    let result = create_bad_geometry::create_bad_geometry(&[]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "BadGeometry.createGeometry");
}

#[test]
fn return_byte_length_returns_input_size() {
    let input = vec![1u8, 2, 3, 4, 5];
    let result = return_byte_length::return_byte_length(&input).unwrap();
    let len = u64::from_le_bytes(result.try_into().unwrap());
    assert_eq!(len, 5);
}

#[test]
fn return_non_cloneable_returns_error() {
    let result = return_non_cloneable::return_non_cloneable(&[]);
    assert!(result.is_err());
}

#[test]
fn return_parameters_echoes_input() {
    let input = vec![10u8, 20, 30, 40];
    let result = return_parameters::return_parameters(&input).unwrap();
    assert_eq!(result, input);
}

#[test]
fn return_wasm_config_returns_config() {
    let input = vec![1u8, 2, 3];
    let result = return_wasm_config::return_wasm_config(&input).unwrap();
    assert_eq!(result, input);
}

#[test]
fn return_wasm_config_empty_returns_error() {
    let result = return_wasm_config::return_wasm_config(&[]);
    assert!(result.is_err());
}

#[test]
fn throw_error_returns_error_with_message() {
    let message = b"test failure";
    let result = throw_error::throw_error(message);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("test failure"));
}

#[test]
fn transfer_array_buffer_creates_zero_buffer() {
    let input = vec![1u8, 2, 3, 4, 5];
    let result = transfer_array_buffer::transfer_array_buffer(&input).unwrap();
    assert_eq!(result.len(), 5);
    assert!(result.iter().all(|&b| b == 0));
}
