//! Ported from `packages/engine/Source/Renderer/MultisampleFramebuffer.js`.
//!
//! A framebuffer with multisample anti-aliasing support.

use crate::renderbuffer::Renderbuffer;
use crate::texture::Texture;

/// Options for creating a [`MultisampleFramebuffer`].
pub struct MultisampleFramebufferOptions {
    /// Number of MSAA samples.
    pub num_samples: u32,
    /// Color texture attachments.
    pub color_textures: Option<Vec<Texture>>,
    /// Color renderbuffer attachments.
    pub color_renderbuffers: Option<Vec<Renderbuffer>>,
    /// Depth renderbuffer attachment.
    pub depth_renderbuffer: Option<Renderbuffer>,
    /// Depth-stencil renderbuffer attachment.
    pub depth_stencil_renderbuffer: Option<Renderbuffer>,
}

/// A framebuffer with multisample anti-aliasing.
///
/// DEVIATION: In wgpu, MSAA is handled via sample_count on texture and pipeline
/// descriptors rather than a separate framebuffer type. This struct manages
/// the MSAA resolve target and render pass configuration.
pub struct MultisampleFramebuffer {
    num_samples: u32,
    color_textures: Vec<Texture>,
    color_renderbuffers: Vec<Renderbuffer>,
    depth_renderbuffer: Option<Renderbuffer>,
    depth_stencil_renderbuffer: Option<Renderbuffer>,
    is_destroyed: bool,
}

impl MultisampleFramebuffer {
    /// Creates a new multisample framebuffer.
    pub fn new(options: MultisampleFramebufferOptions) -> Self {
        debug_assert!(
            options.num_samples > 0,
            "num_samples must be greater than zero"
        );
        Self {
            num_samples: options.num_samples,
            color_textures: options.color_textures.unwrap_or_default(),
            color_renderbuffers: options.color_renderbuffers.unwrap_or_default(),
            depth_renderbuffer: options.depth_renderbuffer,
            depth_stencil_renderbuffer: options.depth_stencil_renderbuffer,
            is_destroyed: false,
        }
    }

    /// Returns the number of MSAA samples.
    pub fn num_samples(&self) -> u32 {
        self.num_samples
    }

    /// Returns the number of color attachments.
    pub fn number_of_color_attachments(&self) -> usize {
        if !self.color_textures.is_empty() {
            self.color_textures.len()
        } else {
            self.color_renderbuffers.len()
        }
    }

    /// Returns whether this framebuffer has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys the multisample framebuffer.
    pub fn destroy(&mut self) {
        for tex in &mut self.color_textures {
            tex.destroy();
        }
        for rb in &mut self.color_renderbuffers {
            rb.destroy();
        }
        if let Some(ref mut r) = self.depth_renderbuffer {
            r.destroy();
        }
        if let Some(ref mut r) = self.depth_stencil_renderbuffer {
            r.destroy();
        }
        self.is_destroyed = true;
    }
}
