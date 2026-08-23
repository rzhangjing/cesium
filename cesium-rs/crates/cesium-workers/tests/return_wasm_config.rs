//! Mock test worker: return_wasm_config.
//!
//! Ported from `Specs/TestWorkers/returnWasmConfig.js`.
//! Extracts and returns the wasm configuration from parameters.

/// A mock worker function that extracts the wasm config from parameters.
///
/// In CesiumJS, this returns `parameters.webAssemblyConfig`.
/// Used in tests to verify structured parameter passing to workers.
pub fn return_wasm_config(params: &[u8]) -> Result<Vec<u8>, String> {
    // In CesiumJS, this extracts parameters.webAssemblyConfig.
    // In Rust, we echo back the input as the "config" for testing purposes.
    // A real implementation would deserialize and extract a specific field.
    if params.is_empty() {
        Err("No wasm config provided".to_string())
    } else {
        Ok(params.to_vec())
    }
}
