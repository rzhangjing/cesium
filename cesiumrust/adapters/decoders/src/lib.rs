//! cesium-decoders: Binary format decoders for terrain and 3D tiles
//!
//! Maps to CesiumJS:
//! - Quantized-mesh terrain format parsing
//! - Draco mesh decoding (future)
//! - KTX2 texture decoding (future)

pub mod quantized_mesh_decoder;

pub use quantized_mesh_decoder::{decode_quantized_mesh, QuantizedMeshError};
