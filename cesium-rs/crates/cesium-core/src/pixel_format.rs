//! Ported from `packages/engine/Source/Core/PixelFormat.js`.
//!
//! The format of a pixel, i.e., the number of components and what they represent.

/// Pixel format constants matching WebGL values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum PixelFormat {
    DepthComponent = 0x1902,
    DepthStencil = 0x84F9,
    Alpha = 0x1906,
    Red = 0x1903,
    Rg = 0x8227,
    Rgb = 0x1907,
    Rgba = 0x1908,
    RedInteger = 0x8D94,
    RgInteger = 0x8228,
    RgbInteger = 0x8D98,
    RgbaInteger = 0x8D99,
    Luminance = 0x1909,
    LuminanceAlpha = 0x190A,
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
}
