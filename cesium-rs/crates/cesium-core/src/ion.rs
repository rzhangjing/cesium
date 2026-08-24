//! Ported from `packages/engine/Source/Core/Ion.js`.
//!
//! Default settings for accessing the Cesium ion API.

use std::sync::{Mutex, OnceLock};

/// Default access token for evaluation purposes.
pub const DEFAULT_ACCESS_TOKEN: &str =
    "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJqdGkiOiIzNzRjZTkzNC05M2UwLTRlNDItOWU0My1hYjk5YjFiNTNhYTMiLCJpZCI6MjU5LCJzdWIiOiJDZXNpdW1KUyIsImlzcyI6Imh0dHBzOi8vYXBpLmNlc2l1bS5jb20iLCJhdWQiOiIxLjE0MyBSZWxlYXNlIC0gRGVsZXRlIG9uIFNlcHRlbWJlciAxLCAyMDI2IiwiaWF0IjoxNzgyMzY4NzY4fQ.kDcFqK7jTTloOcBbwb-epSQGd1Lu12_hRuqk1XRE_H8";

/// Default server URL for Cesium ion API.
pub const DEFAULT_SERVER: &str = "https://api.cesium.com/";

static ACCESS_TOKEN: OnceLock<Mutex<String>> = OnceLock::new();
static SERVER: OnceLock<Mutex<String>> = OnceLock::new();

fn access_token_state() -> &'static Mutex<String> {
    ACCESS_TOKEN.get_or_init(|| Mutex::new(DEFAULT_ACCESS_TOKEN.to_string()))
}

fn server_state() -> &'static Mutex<String> {
    SERVER.get_or_init(|| Mutex::new(DEFAULT_SERVER.to_string()))
}

/// The default Cesium ion access token (mutable global in JS).
///
/// Mirrors reading `Ion.defaultAccessToken`.
pub fn default_access_token() -> String {
    access_token_state().lock().unwrap().clone()
}

/// Sets the default Cesium ion access token.
///
/// Mirrors assigning `Ion.defaultAccessToken = value`.
pub fn set_default_access_token(token: &str) {
    *access_token_state().lock().unwrap() = token.to_string();
}

/// The default Cesium ion server URL.
///
/// Mirrors reading `Ion.defaultServer` (a Resource in JS; DEVIATION: stored
/// as the url string).
pub fn default_server() -> String {
    server_state().lock().unwrap().clone()
}

/// Sets the default Cesium ion server URL.
///
/// Mirrors assigning `Ion.defaultServer = value`.
pub fn set_default_server(server: &str) {
    *server_state().lock().unwrap() = server.to_string();
}

// DEVIATION: `Ion.getDefaultTokenCredit` (Credit rendering for the default
// token) requires the Credit/attribution pipeline; not ported in this batch.
