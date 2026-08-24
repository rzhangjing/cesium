//! Ported from `packages/engine/Source/Renderer/Framebuffer.js`.
//!
//! A framebuffer for render-to-texture effects.

use crate::renderbuffer::Renderbuffer;
use crate::texture::Texture;

use std::sync::Arc;

/// Options for creating a [`Framebuffer`].
pub struct FramebufferOptions {
    /// Color texture attachments.
    ///
    /// DEVIATION (B4-3): attachments are `Arc<Texture>` so callers (e.g. the
    /// globe pass blit) can keep a shareable handle to the color texture
    /// outside the framebuffer.
    pub color_textures: Option<Vec<Arc<Texture>>>,
    /// Color renderbuffer attachments.
    pub color_renderbuffers: Option<Vec<Renderbuffer>>,
    /// Depth texture attachment.
    pub depth_texture: Option<Arc<Texture>>,
    /// Depth renderbuffer attachment.
    pub depth_renderbuffer: Option<Renderbuffer>,
    /// Stencil renderbuffer attachment.
    pub stencil_renderbuffer: Option<Renderbuffer>,
    /// Combined depth-stencil texture attachment.
    pub depth_stencil_texture: Option<Arc<Texture>>,
    /// Combined depth-stencil renderbuffer attachment.
    pub depth_stencil_renderbuffer: Option<Renderbuffer>,
    /// Whether the framebuffer owns its attachments. Defaults to `true`.
    pub destroy_attachments: bool,
}

impl Default for FramebufferOptions {
    fn default() -> Self {
        Self {
            color_textures: None,
            color_renderbuffers: None,
            depth_texture: None,
            depth_renderbuffer: None,
            stencil_renderbuffer: None,
            depth_stencil_texture: None,
            depth_stencil_renderbuffer: None,
            destroy_attachments: true,
        }
    }
}

/// A framebuffer for off-screen rendering (render-to-texture).
///
/// DEVIATION: In WebGL, framebuffers are persistent GPU objects. In wgpu,
/// render passes reference texture views imperatively. This struct manages
/// the attachment set and creates `RenderPassDescriptor`-compatible views
/// on demand.
pub struct Framebuffer {
    color_textures: Vec<Arc<Texture>>,
    color_renderbuffers: Vec<Renderbuffer>,
    depth_texture: Option<Arc<Texture>>,
    depth_renderbuffer: Option<Renderbuffer>,
    stencil_renderbuffer: Option<Renderbuffer>,
    depth_stencil_texture: Option<Arc<Texture>>,
    depth_stencil_renderbuffer: Option<Renderbuffer>,
    /// When true, the framebuffer owns its attachments.
    pub destroy_attachments: bool,
    is_destroyed: bool,
}

impl Framebuffer {
    /// Creates a new framebuffer with the given options.
    ///
    /// Mirrors the JS constructor `new Framebuffer(options)`.
    pub fn new(options: FramebufferOptions) -> Self {
        debug_assert!(
            !(options.color_textures.is_some() && options.color_renderbuffers.is_some()),
            "Cannot have both color texture and color renderbuffer attachments"
        );
        debug_assert!(
            !(options.depth_texture.is_some() && options.depth_renderbuffer.is_some()),
            "Cannot have both a depth texture and depth renderbuffer attachment"
        );
        debug_assert!(
            !(options.depth_stencil_texture.is_some()
                && options.depth_stencil_renderbuffer.is_some()),
            "Cannot have both a depth-stencil texture and depth-stencil renderbuffer attachment"
        );

        Self {
            color_textures: options.color_textures.unwrap_or_default(),
            color_renderbuffers: options.color_renderbuffers.unwrap_or_default(),
            depth_texture: options.depth_texture,
            depth_renderbuffer: options.depth_renderbuffer,
            stencil_renderbuffer: options.stencil_renderbuffer,
            depth_stencil_texture: options.depth_stencil_texture,
            depth_stencil_renderbuffer: options.depth_stencil_renderbuffer,
            destroy_attachments: options.destroy_attachments,
            is_destroyed: false,
        }
    }

    /// Returns the number of active color attachments.
    pub fn number_of_color_attachments(&self) -> usize {
        if !self.color_textures.is_empty() {
            self.color_textures.len()
        } else {
            self.color_renderbuffers.len()
        }
    }

    /// Returns whether this framebuffer has a depth attachment.
    pub fn has_depth_attachment(&self) -> bool {
        self.depth_texture.is_some()
            || self.depth_renderbuffer.is_some()
            || self.depth_stencil_texture.is_some()
            || self.depth_stencil_renderbuffer.is_some()
    }

    /// Returns the color texture at the given index.
    pub fn get_color_texture(&self, index: usize) -> Option<&Arc<Texture>> {
        self.color_textures.get(index)
    }

    /// Returns the color renderbuffer at the given index.
    pub fn get_color_renderbuffer(&self, index: usize) -> Option<&Renderbuffer> {
        self.color_renderbuffers.get(index)
    }

    /// Returns the depth texture, if any.
    pub fn depth_texture(&self) -> Option<&Arc<Texture>> {
        self.depth_texture.as_ref()
    }

    /// Returns the depth renderbuffer, if any.
    pub fn depth_renderbuffer(&self) -> Option<&Renderbuffer> {
        self.depth_renderbuffer.as_ref()
    }

    /// Returns the stencil renderbuffer, if any.
    pub fn stencil_renderbuffer(&self) -> Option<&Renderbuffer> {
        self.stencil_renderbuffer.as_ref()
    }

    /// Returns the depth-stencil texture, if any.
    pub fn depth_stencil_texture(&self) -> Option<&Arc<Texture>> {
        self.depth_stencil_texture.as_ref()
    }

    /// Returns the depth-stencil renderbuffer, if any.
    pub fn depth_stencil_renderbuffer(&self) -> Option<&Renderbuffer> {
        self.depth_stencil_renderbuffer.as_ref()
    }

    // ---- Attachment views (wgpu render pass inputs) ----

    /// Creates a view of the color attachment at `index` (texture or
    /// renderbuffer), for use in a `RenderPassColorAttachment`.
    pub fn color_attachment_view(&self, index: usize) -> Option<wgpu::TextureView> {
        if let Some(texture) = self.color_textures.get(index) {
            return Some(texture.create_view());
        }
        self.color_renderbuffers.get(index).map(|rb| rb.create_view())
    }

    /// Returns the wgpu format of the first color attachment, if any.
    pub fn color_attachment_format(&self) -> Option<wgpu::TextureFormat> {
        if let Some(texture) = self.color_textures.first() {
            return Some(texture.wgpu_format());
        }
        self.color_renderbuffers
            .first()
            .map(|rb| rb.wgpu_texture().format())
    }

    /// Creates a view of the depth(-stencil) attachment, if any.
    /// Priority mirrors CesiumJS: depth-stencil texture, depth texture,
    /// depth-stencil renderbuffer, depth renderbuffer.
    pub fn depth_stencil_attachment_view(&self) -> Option<wgpu::TextureView> {
        if let Some(texture) = &self.depth_stencil_texture {
            return Some(texture.create_view());
        }
        if let Some(texture) = &self.depth_texture {
            return Some(texture.create_view());
        }
        if let Some(renderbuffer) = &self.depth_stencil_renderbuffer {
            return Some(renderbuffer.create_view());
        }
        self.depth_renderbuffer.as_ref().map(|rb| rb.create_view())
    }

    /// Returns the wgpu format of the depth(-stencil) attachment, if any.
    pub fn depth_stencil_format(&self) -> Option<wgpu::TextureFormat> {
        if let Some(texture) = &self.depth_stencil_texture {
            return Some(texture.wgpu_format());
        }
        if let Some(texture) = &self.depth_texture {
            return Some(texture.wgpu_format());
        }
        if let Some(renderbuffer) = &self.depth_stencil_renderbuffer {
            return Some(renderbuffer.wgpu_texture().format());
        }
        self.depth_renderbuffer
            .as_ref()
            .map(|rb| rb.wgpu_texture().format())
    }

    /// Returns the width of the first color attachment, if any.
    pub fn width(&self) -> Option<u32> {
        self.color_textures
            .first()
            .map(|t| t.width())
            .or_else(|| self.color_renderbuffers.first().map(|rb| rb.width()))
    }

    /// Returns the height of the first color attachment, if any.
    pub fn height(&self) -> Option<u32> {
        self.color_textures
            .first()
            .map(|t| t.height())
            .or_else(|| self.color_renderbuffers.first().map(|rb| rb.height()))
    }

    /// Returns whether this framebuffer has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys the framebuffer and optionally its attachments.
    pub fn destroy(&mut self) {
        if self.destroy_attachments {
            for tex in &mut self.color_textures {
                // Shared attachments (Arc still referenced elsewhere, e.g.
                // by a scene-owned globe framebuffer) are left to their
                // owner; only uniquely-held textures are destroyed here.
                if let Some(tex) = Arc::get_mut(tex) {
                    tex.destroy();
                }
            }
            for rb in &mut self.color_renderbuffers {
                rb.destroy();
            }
            if let Some(ref mut t) = self.depth_texture {
                if let Some(t) = Arc::get_mut(t) {
                    t.destroy();
                }
            }
            if let Some(ref mut r) = self.depth_renderbuffer {
                r.destroy();
            }
            if let Some(ref mut r) = self.stencil_renderbuffer {
                r.destroy();
            }
            if let Some(ref mut t) = self.depth_stencil_texture {
                if let Some(t) = Arc::get_mut(t) {
                    t.destroy();
                }
            }
            if let Some(ref mut r) = self.depth_stencil_renderbuffer {
                r.destroy();
            }
        }
        self.is_destroyed = true;
    }
}
