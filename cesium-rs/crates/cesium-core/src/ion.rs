//! Ported from `packages/engine/Source/Core/Ion.js`.
//!
//! Default settings for accessing the Cesium ion API.

/// Default access token for evaluation purposes.
pub const DEFAULT_ACCESS_TOKEN: &str =
    "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJqdGkiOiIzNzRjZTkzNC05M2UwLTRlNDItOWU0My1hYjk5YjFiNTNhYTMiLCJpZCI6MjU5LCJzdWIiOiJDZXNpdW1KUyIsImlzcyI6Imh0dHBzOi8vYXBpLmNlc2l1bS5jb20iLCJhdWQiOiIxLjE0MyBSZWxlYXNlIC0gRGVsZXRlIG9uIFNlcHRlbWJlciAxLCAyMDI2IiwiaWF0IjoxNzgyMzY4NzY4fQ.kDcFqK7jTTloOcBbwb-epSQGd1Lu12_hRuqk1XRE_H8";

/// Default server URL for Cesium ion API.
pub const DEFAULT_SERVER: &str = "https://api.cesium.com/";

/// Gets or sets the default Cesium ion access token.
pub fn default_access_token() -> &'static str {
    DEFAULT_ACCESS_TOKEN
}

/// Gets or sets the default Cesium ion server URL.
pub fn default_server() -> &'static str {
    DEFAULT_SERVER
}
