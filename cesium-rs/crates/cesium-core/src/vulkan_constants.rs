//! Ported from `packages/engine/Source/Core/VulkanConstants.js`.
//!
//! Vulkan API constants.

/// Vulkan format constants.
pub struct VulkanConstants;

impl VulkanConstants {
    pub const VK_FORMAT_UNDEFINED: u32 = 0;
    pub const VK_FORMAT_R8_UNORM: u32 = 9;
    pub const VK_FORMAT_R8G8_UNORM: u32 = 16;
    pub const VK_FORMAT_R8G8B8_UNORM: u32 = 23;
    pub const VK_FORMAT_R8G8B8A8_UNORM: u32 = 37;
    pub const VK_FORMAT_R16_SFLOAT: u32 = 70;
    pub const VK_FORMAT_R16G16_SFLOAT: u32 = 73;
    pub const VK_FORMAT_R16G16B16A16_SFLOAT: u32 = 76;
    pub const VK_FORMAT_R32_SFLOAT: u32 = 100;
    pub const VK_FORMAT_R32G32_SFLOAT: u32 = 103;
    pub const VK_FORMAT_R32G32B32_SFLOAT: u32 = 106;
    pub const VK_FORMAT_R32G32B32A32_SFLOAT: u32 = 109;
    pub const VK_FORMAT_D16_UNORM: u32 = 124;
    pub const VK_FORMAT_D32_SFLOAT: u32 = 126;
    pub const VK_FORMAT_D24_UNORM_S8_UINT: u32 = 129;
}
