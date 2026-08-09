//! cesium-decoders: Binary format decoders for terrain and 3D tiles
//!
//! Maps to CesiumJS:
//! - Quantized-mesh terrain format parsing
//! - Image decoding (PNG/JPEG/WebP)
//! - Gzip decompression
//! - Draco mesh decoding (future)
//! - KTX2 texture decoding (future)

pub mod quantized_mesh_decoder;
pub mod image_decoder;
pub mod gzip_decoder;
pub mod content_type;

#[cfg(feature = "draco")]
pub mod draco_decoder;

pub use quantized_mesh_decoder::{decode_quantized_mesh, QuantizedMeshError};

use cesium_ports_driven::{DecodedImage, Decoder, PortError, PortResult};

pub struct DecoderImpl;

impl Decoder for DecoderImpl {
    fn decode_draco(&self, _data: &[u8]) -> PortResult<cesium_geospatial::GeometryData> {
        #[cfg(feature = "draco")]
        {
            draco_decoder::decode_draco(_data)
        }
        #[cfg(not(feature = "draco"))]
        {
            Err(PortError::Decode(
                "Draco decoding not yet implemented".to_string(),
            ))
        }
    }

    fn decode_image(&self, data: &[u8]) -> PortResult<DecodedImage> {
        image_decoder::decode_image(data)
    }

    fn decode_gzip(&self, data: &[u8]) -> PortResult<Vec<u8>> {
        gzip_decoder::decode_gzip(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use image::ImageEncoder;
    use std::io::Write;

    #[test]
    fn test_decode_png() {
        let png_data = create_test_png();
        let decoder = DecoderImpl;
        let result = decoder.decode_image(&png_data).unwrap();
        assert_eq!(result.width, 4);
        assert_eq!(result.height, 4);
        assert_eq!(result.channels, 4);
        assert_eq!(result.data.len(), 4 * 4 * 4);
    }

    #[test]
    fn test_decode_jpeg() {
        let jpeg_data = create_test_jpeg();
        let decoder = DecoderImpl;
        let result = decoder.decode_image(&jpeg_data).unwrap();
        assert_eq!(result.width, 4);
        assert_eq!(result.height, 4);
        assert_eq!(result.channels, 4);
    }

    #[test]
    fn test_decode_gzip_roundtrip() {
        let original = b"Hello, quantized mesh! This is test data for gzip roundtrip verification.";
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(original).unwrap();
        let compressed = encoder.finish().unwrap();

        let decoder = DecoderImpl;
        let decompressed = decoder.decode_gzip(&compressed).unwrap();
        assert_eq!(decompressed, original);
    }

    #[test]
    fn test_decode_gzip_empty() {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"").unwrap();
        let compressed = encoder.finish().unwrap();

        let decoder = DecoderImpl;
        let decompressed = decoder.decode_gzip(&compressed).unwrap();
        assert!(decompressed.is_empty());
    }

    #[test]
    fn test_content_type_detection() {
        use cesium_tileset::content_decoder::{detect_content_type, TileContentType};

        assert!(matches!(
            detect_content_type(b"b3dm data here..."),
            TileContentType::Batched3DModel
        ));
        assert!(matches!(
            detect_content_type(b"i3dm data here..."),
            TileContentType::Instanced3DModel
        ));
        assert!(matches!(
            detect_content_type(b"pnts data here..."),
            TileContentType::PointCloud
        ));
        assert!(matches!(
            detect_content_type(b"cmpt data here..."),
            TileContentType::Composite
        ));
        assert!(matches!(
            detect_content_type(b"glTF data here..."),
            TileContentType::GltfBinary
        ));
        assert!(matches!(
            detect_content_type(b"xxxx data here..."),
            TileContentType::Unknown
        ));
    }

    #[test]
    fn test_draco_stub() {
        let decoder = DecoderImpl;
        let result = decoder.decode_draco(b"fake draco data");
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("not yet implemented"));
    }

    fn create_test_png() -> Vec<u8> {
        let mut pixels = Vec::with_capacity(4 * 4 * 4);
        for i in 0..16 {
            let v = (i * 16) as u8;
            pixels.extend_from_slice(&[v, 255 - v, v, 255]);
        }
        let mut buf = Vec::new();
        {
            let encoder = image::codecs::png::PngEncoder::new(&mut buf);
            encoder
                .write_image(
                    &pixels,
                    4,
                    4,
                    image::ExtendedColorType::Rgba8,
                )
                .unwrap();
        }
        buf
    }

    fn create_test_jpeg() -> Vec<u8> {
        let mut pixels = Vec::with_capacity(4 * 4 * 3);
        for i in 0..16 {
            let v = (i * 16) as u8;
            pixels.extend_from_slice(&[v, 128, 255 - v]);
        }
        let mut buf = Vec::new();
        {
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 90);
            encoder
                .write_image(
                    &pixels,
                    4,
                    4,
                    image::ExtendedColorType::Rgb8,
                )
                .unwrap();
        }
        buf
    }
}
