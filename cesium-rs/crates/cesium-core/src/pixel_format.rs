//! Ported from `packages/engine/Source/Core/PixelFormat.js`.
//!
//! The format of a pixel, i.e., the number of components and what they represent.

use crate::webgl_constants::WebGLConstants;

/// Pixel format constants matching WebGL values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum PixelFormat {
    /// A pixel format containing a depth value.
    DepthComponent = 0x1902, // WebGLConstants.DEPTH_COMPONENT
    /// A pixel format containing a depth and stencil value.
    DepthStencil = 0x84F9, // WebGLConstants.DEPTH_STENCIL
    /// A pixel format containing an alpha channel.
    Alpha = 0x1906, // WebGLConstants.ALPHA
    /// A pixel format containing a red channel.
    Red = 0x1903, // WebGLConstants.RED
    /// A pixel format containing red and green channels.
    Rg = 0x8227, // WebGLConstants.RG
    /// A pixel format containing red, green, and blue channels.
    Rgb = 0x1907, // WebGLConstants.RGB
    /// A pixel format containing red, green, blue, and alpha channels.
    Rgba = 0x1908, // WebGLConstants.RGBA
    /// A pixel format containing a red channel as an integer.
    RedInteger = 0x8D94, // WebGLConstants.RED_INTEGER
    /// A pixel format containing red and green channels as integers.
    RgInteger = 0x8228, // WebGLConstants.RG_INTEGER
    /// A pixel format containing red, green, and blue channels as integers.
    RgbInteger = 0x8D98, // WebGLConstants.RGB_INTEGER
    /// A pixel format containing red, green, blue, and alpha channels as integers.
    RgbaInteger = 0x8D99, // WebGLConstants.RGBA_INTEGER
    /// A pixel format containing a luminance (intensity) channel.
    Luminance = 0x1909, // WebGLConstants.LUMINANCE
    /// A pixel format containing luminance (intensity) and alpha channels.
    LuminanceAlpha = 0x190A, // WebGLConstants.LUMINANCE_ALPHA
    /// A pixel format containing red, green, and blue channels that is DXT1 compressed.
    RgbDxt1 = 0x83F0, // WebGLConstants.COMPRESSED_RGB_S3TC_DXT1_EXT
    /// A pixel format containing red, green, blue, and alpha channels that is DXT1 compressed.
    RgbaDxt1 = 0x83F1, // WebGLConstants.COMPRESSED_RGBA_S3TC_DXT1_EXT
    /// A pixel format containing red, green, blue, and alpha channels that is DXT3 compressed.
    RgbaDxt3 = 0x83F2, // WebGLConstants.COMPRESSED_RGBA_S3TC_DXT3_EXT
    /// A pixel format containing red, green, blue, and alpha channels that is DXT5 compressed.
    RgbaDxt5 = 0x83F3, // WebGLConstants.COMPRESSED_RGBA_S3TC_DXT5_EXT
    /// A pixel format containing red, green, and blue channels that is PVR 4bpp compressed.
    RgbPvrtc4Bppv1 = 0x8C00, // WebGLConstants.COMPRESSED_RGB_PVRTC_4BPPV1_IMG
    /// A pixel format containing red, green, and blue channels that is PVR 2bpp compressed.
    RgbPvrtc2Bppv1 = 0x8C01, // WebGLConstants.COMPRESSED_RGB_PVRTC_2BPPV1_IMG
    /// A pixel format containing red, green, blue, and alpha channels that is PVR 4bpp compressed.
    RgbaPvrtc4Bppv1 = 0x8C02, // WebGLConstants.COMPRESSED_RGBA_PVRTC_4BPPV1_IMG
    /// A pixel format containing red, green, blue, and alpha channels that is PVR 2bpp compressed.
    RgbaPvrtc2Bppv1 = 0x8C03, // WebGLConstants.COMPRESSED_RGBA_PVRTC_2BPPV1_IMG
    /// A pixel format containing red, green, blue, and alpha channels that is ASTC compressed.
    RgbaAstc = 0x93B0, // WebGLConstants.COMPRESSED_RGBA_ASTC_4x4_WEBGL
    /// A pixel format containing red, green, and blue channels that is ETC1 compressed.
    RgbEtc1 = 0x8D64, // WebGLConstants.COMPRESSED_RGB_ETC1_WEBGL
    /// A pixel format containing red, green, and blue channels that is ETC2 compressed.
    Rgb8Etc2 = 0x9274, // WebGLConstants.COMPRESSED_RGB8_ETC2
    /// A pixel format containing red, green, blue, and alpha channels that is ETC2 compressed.
    Rgba8Etc2Eac = 0x9278, // WebGLConstants.COMPRESSED_RGBA8_ETC2_EAC
    /// A pixel format containing red, green, blue, and alpha channels that is BC7 compressed.
    RgbaBc7 = 0x8E8C, // WebGLConstants.COMPRESSED_RGBA_BPTC_UNORM
}

/// A typed array mirroring the JS `TypedArray` family used by pixel buffers.
#[derive(Debug, Clone, PartialEq)]
pub enum TypedArray {
    /// Mirrors `Uint8Array`.
    U8(Vec<u8>),
    /// Mirrors `Uint16Array`.
    U16(Vec<u16>),
    /// Mirrors `Uint32Array`.
    U32(Vec<u32>),
    /// Mirrors `Float32Array`.
    F32(Vec<f32>),
}

impl TypedArray {
    /// Returns the number of elements in the typed array.
    pub fn len(&self) -> usize {
        match self {
            Self::U8(v) => v.len(),
            Self::U16(v) => v.len(),
            Self::U32(v) => v.len(),
            Self::F32(v) => v.len(),
        }
    }

    /// Returns true if the typed array contains no elements.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// ---- PixelDatatype helpers (mirrors `packages/engine/Source/Renderer/PixelDatatype.js`) ----
//
// DEVIATION: cesium-core cannot depend on cesium-renderer, so pixel datatypes are
// accepted as raw WebGL constant values (`u32`). The values below mirror the JS
// `PixelDatatype` enum exactly, including `HALF_FLOAT = HALF_FLOAT_OES (0x8D61)`.

/// Mirrors `PixelDatatype.HALF_FLOAT` (`WebGLConstants.HALF_FLOAT_OES`).
pub const PIXEL_DATATYPE_HALF_FLOAT: u32 = WebGLConstants::HALF_FLOAT_OES;

fn datatype_size_in_bytes(pixel_datatype: u32) -> usize {
    match pixel_datatype {
        WebGLConstants::UNSIGNED_BYTE => 1,
        WebGLConstants::UNSIGNED_SHORT
        | WebGLConstants::UNSIGNED_SHORT_4_4_4_4
        | WebGLConstants::UNSIGNED_SHORT_5_5_5_1
        | WebGLConstants::UNSIGNED_SHORT_5_6_5
        | PIXEL_DATATYPE_HALF_FLOAT => 2,
        WebGLConstants::UNSIGNED_INT | WebGLConstants::FLOAT | WebGLConstants::UNSIGNED_INT_24_8 => {
            4
        }
        // JS `sizeInBytes` has no default and returns `undefined` for other values;
        // treat unknown values as 1 byte so arithmetic stays defined.
        _ => 1,
    }
}

fn datatype_is_packed(pixel_datatype: u32) -> bool {
    matches!(
        pixel_datatype,
        WebGLConstants::UNSIGNED_INT_24_8
            | WebGLConstants::UNSIGNED_SHORT_4_4_4_4
            | WebGLConstants::UNSIGNED_SHORT_5_5_5_1
            | WebGLConstants::UNSIGNED_SHORT_5_6_5
    )
}

/// Mirrors `PixelDatatype.getTypedArrayConstructor`; returns a zeroed array of `size` elements.
fn create_typed_array_for_datatype(pixel_datatype: u32, size: usize) -> TypedArray {
    let size_in_bytes = datatype_size_in_bytes(pixel_datatype);
    if size_in_bytes == 1 {
        TypedArray::U8(vec![0; size])
    } else if size_in_bytes == 2 {
        TypedArray::U16(vec![0; size])
    } else if size_in_bytes == 4 && pixel_datatype == WebGLConstants::FLOAT {
        TypedArray::F32(vec![0.0; size])
    } else {
        TypedArray::U32(vec![0; size])
    }
}

impl PixelFormat {
    /// Returns the number of components for the format.
    pub fn components_length(&self) -> usize {
        match self {
            Self::Rgb | Self::RgbInteger => 3,
            Self::Rgba | Self::RgbaInteger => 4,
            Self::LuminanceAlpha | Self::Rg | Self::RgInteger => 2,
            _ => 1,
        }
    }

    /// Mirrors `PixelFormat.validate`; accepts a raw constant value.
    pub fn validate(pixel_format: u32) -> bool {
        matches!(
            pixel_format,
            0x1902 | 0x84F9
                | 0x1906
                | 0x1903
                | 0x8227
                | 0x1907
                | 0x1908
                | 0x8D94
                | 0x8228
                | 0x8D98
                | 0x8D99
                | 0x1909
                | 0x190A
                | 0x83F0
                | 0x83F1
                | 0x83F2
                | 0x83F3
                | 0x8C00
                | 0x8C01
                | 0x8C02
                | 0x8C03
                | 0x93B0
                | 0x8D64
                | 0x9274
                | 0x9278
                | 0x8E8C
        )
    }

    /// Returns true if this is a color format.
    pub fn is_color_format(&self) -> bool {
        matches!(
            self,
            Self::Red
                | Self::Alpha
                | Self::Rgb
                | Self::Rgba
                | Self::Luminance
                | Self::LuminanceAlpha
        )
    }

    /// Returns true if this is a depth format.
    pub fn is_depth_format(&self) -> bool {
        matches!(self, Self::DepthComponent | Self::DepthStencil)
    }

    /// Returns true if this is a compressed format.
    pub fn is_compressed_format(&self) -> bool {
        matches!(
            self,
            Self::RgbDxt1
                | Self::RgbaDxt1
                | Self::RgbaDxt3
                | Self::RgbaDxt5
                | Self::RgbPvrtc4Bppv1
                | Self::RgbPvrtc2Bppv1
                | Self::RgbaPvrtc4Bppv1
                | Self::RgbaPvrtc2Bppv1
                | Self::RgbaAstc
                | Self::RgbEtc1
                | Self::Rgb8Etc2
                | Self::Rgba8Etc2Eac
                | Self::RgbaBc7
        )
    }

    /// Returns true if this is a DXT format.
    pub fn is_dxt_format(&self) -> bool {
        matches!(
            self,
            Self::RgbDxt1 | Self::RgbaDxt1 | Self::RgbaDxt3 | Self::RgbaDxt5
        )
    }

    /// Returns true if this is a PVRTC format.
    pub fn is_pvrtc_format(&self) -> bool {
        matches!(
            self,
            Self::RgbPvrtc4Bppv1 | Self::RgbPvrtc2Bppv1 | Self::RgbaPvrtc4Bppv1 | Self::RgbaPvrtc2Bppv1
        )
    }

    /// Returns true if this is an ASTC format.
    pub fn is_astc_format(&self) -> bool {
        matches!(self, Self::RgbaAstc)
    }

    /// Returns true if this is an ETC1 format.
    pub fn is_etc1_format(&self) -> bool {
        matches!(self, Self::RgbEtc1)
    }

    /// Returns true if this is an ETC2 format.
    pub fn is_etc2_format(&self) -> bool {
        matches!(self, Self::Rgb8Etc2 | Self::Rgba8Etc2Eac)
    }

    /// Returns true if this is a BC7 format.
    pub fn is_bc7_format(&self) -> bool {
        matches!(self, Self::RgbaBc7)
    }

    /// Returns the size in bytes of a compressed texture of the given dimensions.
    pub fn compressed_texture_size_in_bytes(&self, width: usize, height: usize) -> usize {
        match self {
            Self::RgbDxt1 | Self::RgbaDxt1 | Self::RgbEtc1 | Self::Rgb8Etc2 => {
                ((width + 3) / 4) * ((height + 3) / 4) * 8
            }
            Self::RgbaDxt3 | Self::RgbaDxt5 | Self::RgbaAstc | Self::Rgba8Etc2Eac => {
                ((width + 3) / 4) * ((height + 3) / 4) * 16
            }
            Self::RgbPvrtc4Bppv1 | Self::RgbaPvrtc4Bppv1 => {
                (width.max(8) * height.max(8) * 4 + 7) / 8
            }
            Self::RgbPvrtc2Bppv1 | Self::RgbaPvrtc2Bppv1 => {
                (width.max(16) * height.max(8) * 2 + 7) / 8
            }
            Self::RgbaBc7 => {
                // Mirrors `Math.ceil(width / 4) * Math.ceil(height / 4) * 16`.
                ((width + 3) / 4) * ((height + 3) / 4) * 16
            }
            _ => 0,
        }
    }

    /// Returns the size in bytes of an uncompressed texture.
    pub fn texture_size_in_bytes(
        &self,
        pixel_datatype: u32,
        width: usize,
        height: usize,
    ) -> usize {
        let mut components_length = self.components_length();
        if datatype_is_packed(pixel_datatype) {
            components_length = 1;
        }
        components_length * datatype_size_in_bytes(pixel_datatype) * width * height
    }

    /// Returns the size in bytes of an uncompressed 3D texture.
    pub fn texture_3d_size_in_bytes(
        &self,
        pixel_datatype: u32,
        width: usize,
        height: usize,
        depth: usize,
    ) -> usize {
        let mut components_length = self.components_length();
        if datatype_is_packed(pixel_datatype) {
            components_length = 1;
        }
        components_length * datatype_size_in_bytes(pixel_datatype) * width * height * depth
    }

    /// Returns the alignment in bytes for a row of the given width.
    pub fn alignment_in_bytes(&self, pixel_datatype: u32, width: usize) -> usize {
        let modulo = self.texture_size_in_bytes(pixel_datatype, width, 1) % 4;
        if modulo == 0 {
            4
        } else if modulo == 2 {
            2
        } else {
            1
        }
    }

    /// Creates a zeroed typed array large enough to hold the texture data.
    pub fn create_typed_array(
        &self,
        pixel_datatype: u32,
        width: usize,
        height: usize,
    ) -> TypedArray {
        let size = self.components_length() * width * height;
        create_typed_array_for_datatype(pixel_datatype, size)
    }

    /// Flips the rows of a texture buffer vertically.
    ///
    /// When `height` is 1 the input view is returned unchanged (mirrors the JS
    /// early return of the original `bufferView`).
    pub fn flip_y(
        buffer_view: &TypedArray,
        pixel_format: PixelFormat,
        pixel_datatype: u32,
        width: usize,
        height: usize,
    ) -> TypedArray {
        if height == 1 {
            return buffer_view.clone();
        }
        let mut flipped = pixel_format.create_typed_array(pixel_datatype, width, height);
        let number_of_components = pixel_format.components_length();
        let texture_width = width * number_of_components;

        for i in 0..height {
            let row = i * width * number_of_components;
            let flipped_row = (height - i - 1) * width * number_of_components;
            for j in 0..texture_width {
                match (buffer_view, &mut flipped) {
                    (TypedArray::U8(src), TypedArray::U8(dst)) => {
                        dst[flipped_row + j] = src[row + j];
                    }
                    (TypedArray::U16(src), TypedArray::U16(dst)) => {
                        dst[flipped_row + j] = src[row + j];
                    }
                    (TypedArray::U32(src), TypedArray::U32(dst)) => {
                        dst[flipped_row + j] = src[row + j];
                    }
                    (TypedArray::F32(src), TypedArray::F32(dst)) => {
                        dst[flipped_row + j] = src[row + j];
                    }
                    // A typed-array/datatype mismatch cannot occur when both are
                    // produced through `create_typed_array` for the same datatype.
                    _ => {}
                }
            }
        }
        flipped
    }

    /// Converts a pixel format/datatype pair to the WebGL2 internal format.
    ///
    /// `webgl2` mirrors `context.webgl2`; WebGL1 requires the internal format to
    /// equal the pixel format.
    pub fn to_internal_format(&self, pixel_datatype: u32, webgl2: bool) -> u32 {
        let pixel_format = *self as u32;

        // WebGL 1 requires internalFormat to be the same as PixelFormat
        if !webgl2 {
            return pixel_format;
        }

        // Convert pixelFormat to correct internalFormat for WebGL 2
        if *self == PixelFormat::DepthStencil {
            return WebGLConstants::DEPTH24_STENCIL8;
        }

        if *self == PixelFormat::DepthComponent {
            if pixel_datatype == WebGLConstants::UNSIGNED_SHORT {
                return WebGLConstants::DEPTH_COMPONENT16;
            } else if pixel_datatype == WebGLConstants::UNSIGNED_INT {
                return WebGLConstants::DEPTH_COMPONENT24;
            }
        }

        if pixel_datatype == WebGLConstants::FLOAT {
            match self {
                Self::Rgba => return WebGLConstants::RGBA32F,
                Self::Rgb => return WebGLConstants::RGB32F,
                Self::Rg => return WebGLConstants::RG32F,
                Self::Red => return WebGLConstants::R32F,
                _ => {}
            }
        }

        if pixel_datatype == PIXEL_DATATYPE_HALF_FLOAT {
            match self {
                Self::Rgba => return WebGLConstants::RGBA16F,
                Self::Rgb => return WebGLConstants::RGB16F,
                Self::Rg => return WebGLConstants::RG16F,
                Self::Red => return WebGLConstants::R16F,
                _ => {}
            }
        }

        if pixel_datatype == WebGLConstants::UNSIGNED_BYTE {
            match self {
                Self::Rgba => return WebGLConstants::RGBA8,
                Self::Rgb => return WebGLConstants::RGB8,
                Self::Rg => return WebGLConstants::RG8,
                Self::Red => return WebGLConstants::R8,
                _ => {}
            }
        }

        if pixel_datatype == WebGLConstants::INT {
            match self {
                Self::RgbaInteger => return WebGLConstants::RGBA32I,
                Self::RgbInteger => return WebGLConstants::RGB32I,
                Self::RgInteger => return WebGLConstants::RG32I,
                Self::RedInteger => return WebGLConstants::R32I,
                _ => {}
            }
        }

        if pixel_datatype == WebGLConstants::UNSIGNED_INT {
            match self {
                Self::RgbaInteger => return WebGLConstants::RGBA32UI,
                Self::RgbInteger => return WebGLConstants::RGB32UI,
                Self::RgInteger => return WebGLConstants::RG32UI,
                Self::RedInteger => return WebGLConstants::R32UI,
                _ => {}
            }
        }

        pixel_format
    }
}
