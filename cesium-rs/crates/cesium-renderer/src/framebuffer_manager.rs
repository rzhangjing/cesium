//! Ported from `packages/engine/Source/Renderer/FramebufferManager.js`.
//!
//! Manages framebuffer resources and attachments.

use crate::renderbuffer::Renderbuffer;
use crate::texture::Texture;

/// Options for creating a `FramebufferManager`.
pub struct FramebufferManagerOptions {
    /// Number of MSAA samples.
    pub num_samples: u32,
    /// Number of color attachments.
    pub color_attachments_length: usize,
    /// Whether to use color attachments.
    pub color: bool,
    /// Whether to use a depth attachment.
    pub depth: bool,
    /// Whether to use a depth-stencil attachment.
    pub depth_stencil: bool,
}

impl Default for FramebufferManagerOptions {
    fn default() -> Self {
        Self {
            num_samples: 1,
            color_attachments_length: 1,
            color: true,
            depth: false,
            depth_stencil: false,
        }
    }
}

/// Manages framebuffer resources and their lifecycle.
///
/// Wraps a `Framebuffer` (or `MultisampleFramebuffer`) with resource
/// creation helpers for color/depth/stencil attachments.
pub struct FramebufferManager {
    num_samples: u32,
    color_attachments_length: usize,
    use_color: bool,
    use_depth: bool,
    use_depth_stencil: bool,
    is_destroyed: bool,
}

impl FramebufferManager {
    /// Creates a new framebuffer manager.
    pub fn new(options: FramebufferManagerOptions) -> Self {
        debug_assert!(
            options.color || options.depth || options.depth_stencil,
            "Must enable at least one type of framebuffer attachment"
        );
        debug_assert!(
            !(options.depth && options.depth_stencil),
            "Cannot have both a depth and depth-stencil attachment"
        );
        Self {
            num_samples: options.num_samples,
            color_attachments_length: options.color_attachments_length,
            use_color: options.color,
            use_depth: options.depth,
            use_depth_stencil: options.depth_stencil,
            is_destroyed: false,
        }
    }

    /// Returns the number of MSAA samples.
    pub fn num_samples(&self) -> u32 { self.num_samples }

    /// Returns the number of color attachments.
    pub fn color_attachments_length(&self) -> usize { self.color_attachments_length }

    /// Whether color attachments are used.
    pub fn use_color(&self) -> bool { self.use_color }

    /// Whether a depth attachment is used.
    pub fn use_depth(&self) -> bool { self.use_depth }

    /// Whether a depth-stencil attachment is used.
    pub fn use_depth_stencil(&self) -> bool { self.use_depth_stencil }

    /// Returns whether this manager has been destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys the manager and all managed resources.
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}
