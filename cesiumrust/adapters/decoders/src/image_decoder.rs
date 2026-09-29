//! 面向 PNG、JPEG、WebP 格式的图像解码。
//!
//! 使用 `image` crate，它从魔数字节自动检测格式。

use cesium_ports_driven::{DecodedImage, PortError, PortResult};
use image::GenericImageView;

pub fn decode_image(data: &[u8]) -> PortResult<DecodedImage> {
    let img = image::load_from_memory(data)
        .map_err(|e| PortError::Decode(format!("failed to decode image: {e}")))?;

    let (width, height) = img.dimensions();
    let rgba = img.to_rgba8();

    Ok(DecodedImage {
        width,
        height,
        channels: 4,
        data: rgba.into_raw(),
    })
}
