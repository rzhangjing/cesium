//! Draco 网格解码。
//!
//! TODO：通过对 Google draco 库的 C FFI 绑定实现 Draco 解码
//! （<https://github.com/google/draco>），或在 `draco-rs` crate 成熟后改用它。
//! Draco 为 3D 网格与点云提供有损压缩，被广泛用于
//! 3D Tiles 与 glTF 中，以实现高效的几何传输。

use cesium_geospatial::GeometryData;
use cesium_ports_driven::{PortError, PortResult};

/// 解码一段 Draco 压缩网格。目前为未实现的占位：始终返回
/// 一个 [`PortError::Decode`]，直到接入真正的 Draco 后端。
pub fn decode_draco(_data: &[u8]) -> PortResult<GeometryData> {
    Err(PortError::Decode(
        "Draco decoding not yet implemented".to_string(),
    ))
}
