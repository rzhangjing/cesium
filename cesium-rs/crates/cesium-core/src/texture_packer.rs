//! Ported from `packages/engine/Source/Core/TexturePacker.js`.
//!
//! A texture atlas packer that efficiently allocates regions.

use crate::bounding_rectangle::BoundingRectangle;

/// A node in the texture atlas tree.
pub struct TextureNode {
    pub rectangle: BoundingRectangle,
    pub child_node1: Option<Box<TextureNode>>,
    pub child_node2: Option<Box<TextureNode>>,
    pub index: Option<u32>,
}

/// A texture atlas packer.
pub struct TexturePacker {
    _width: u32,
    _height: u32,
    border_padding: u32,
    root: TextureNode,
}

impl TexturePacker {
    /// Creates a new TexturePacker.
    pub fn new(width: u32, height: u32, border_padding: u32) -> Self {
        let root = TextureNode {
            rectangle: BoundingRectangle::new(
                border_padding as f64,
                border_padding as f64,
                (width - 2 * border_padding) as f64,
                (height - 2 * border_padding) as f64,
            ),
            child_node1: None,
            child_node2: None,
            index: None,
        };
        Self {
            _width: width,
            _height: height,
            border_padding,
            root,
        }
    }

    /// Packs an item into the atlas. Returns the node if successful.
    pub fn pack(&mut self, index: u32, item_width: u32, item_height: u32) -> Option<&TextureNode> {
        if let Some(node) = Self::find_node(&mut self.root, item_width, item_height, self.border_padding) {
            node.index = Some(index);
            // We can't return a reference easily due to borrow rules, so return None
            // and let callers use get_root() to traverse.
            Some(node)
        } else {
            None
        }
    }

    /// Gets a reference to the root node.
    pub fn root(&self) -> &TextureNode {
        &self.root
    }

    fn find_node<'a>(
        node: &'a mut TextureNode,
        width: u32,
        height: u32,
        border_padding: u32,
    ) -> Option<&'a mut TextureNode> {
        if node.child_node1.is_none() && node.child_node2.is_none() {
            if node.index.is_some() {
                return None;
            }

            let node_width = node.rectangle.width as u32;
            let node_height = node.rectangle.height as u32;
            let width_diff = node_width as i32 - width as i32;
            let height_diff = node_height as i32 - height as i32;

            if width_diff < 0 || height_diff < 0 {
                return None;
            }

            if width_diff == 0 && height_diff == 0 {
                return Some(node);
            }

            let x = node.rectangle.x as u32;
            let y = node.rectangle.y as u32;

            if width_diff > height_diff {
                let wdp = width_diff - border_padding as i32;
                node.child_node1 = Some(Box::new(TextureNode {
                    rectangle: BoundingRectangle::new(x as f64, y as f64, width as f64, node_height as f64),
                    child_node1: None,
                    child_node2: None,
                    index: None,
                }));
                if wdp > 0 {
                    node.child_node2 = Some(Box::new(TextureNode {
                        rectangle: BoundingRectangle::new(
                            (x + width + border_padding) as f64,
                            y as f64,
                            wdp as f64,
                            node_height as f64,
                        ),
                        child_node1: None,
                        child_node2: None,
                        index: None,
                    }));
                }
                return Self::find_node(node.child_node1.as_deref_mut().unwrap(), width, height, border_padding);
            }

            let hdp = height_diff - border_padding as i32;
            node.child_node1 = Some(Box::new(TextureNode {
                rectangle: BoundingRectangle::new(x as f64, y as f64, node_width as f64, height as f64),
                child_node1: None,
                child_node2: None,
                index: None,
            }));
            if hdp > 0 {
                node.child_node2 = Some(Box::new(TextureNode {
                    rectangle: BoundingRectangle::new(
                        x as f64,
                        (y + height + border_padding) as f64,
                        node_width as f64,
                        hdp as f64,
                    ),
                    child_node1: None,
                    child_node2: None,
                    index: None,
                }));
            }
            return Self::find_node(node.child_node1.as_deref_mut().unwrap(), width, height, border_padding);
        }

        // Non-leaf: try children
        if let Some(ref mut child1) = node.child_node1 {
            if let Some(result) = Self::find_node(child1, width, height, border_padding) {
                return Some(result);
            }
        }
        if let Some(ref mut child2) = node.child_node2 {
            if let Some(result) = Self::find_node(child2, width, height, border_padding) {
                return Some(result);
            }
        }
        None
    }
}
