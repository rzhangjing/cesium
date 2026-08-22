//! Ported from `packages/engine/Source/Core/TileAvailability.js`.

use std::collections::HashMap;

use crate::binary_search;
use crate::cartographic::Cartographic;
use crate::rectangle::Rectangle;
use crate::tiling_scheme::TilingScheme;

/// Reports the availability of tiles in a [`TilingScheme`].
pub struct TileAvailability {
    tiling_scheme: Box<dyn TilingScheme>,
    maximum_level: i32,
    root_nodes: Vec<NodeKey>,
    nodes: HashMap<NodeKey, QuadtreeNodeData>,
}

/// A key identifying a quadtree node by its level and tile coordinates.
type NodeKey = (i32, i32, i32); // (level, x, y)

struct RectangleWithLevel {
    level: i32,
    west: f64,
    south: f64,
    east: f64,
    north: f64,
}

struct QuadtreeNodeData {
    extent: Rectangle,
    rectangles: Vec<RectangleWithLevel>,
}

impl TileAvailability {
    /// Creates a new TileAvailability.
    pub fn new(tiling_scheme: Box<dyn TilingScheme>, maximum_level: i32) -> Self {
        Self {
            tiling_scheme,
            maximum_level,
            root_nodes: Vec::new(),
            nodes: HashMap::new(),
        }
    }

    fn get_or_create_node(&mut self, level: i32, x: i32, y: i32) -> NodeKey {
        let key = (level, x, y);
        if !self.nodes.contains_key(&key) {
            let mut extent = Rectangle::from_radians(0.0, 0.0, 0.0, 0.0);
            self.tiling_scheme
                .tile_xy_to_rectangle(x, y, level, &mut extent);
            self.nodes.insert(
                key,
                QuadtreeNodeData {
                    extent,
                    rectangles: Vec::new(),
                },
            );
        }
        key
    }

    fn child_key(parent: &NodeKey, quadrant: usize) -> (i32, i32, i32) {
        let (level, x, y) = *parent;
        match quadrant {
            0 => (level + 1, x * 2, y * 2),         // nw
            1 => (level + 1, x * 2 + 1, y * 2),     // ne
            2 => (level + 1, x * 2, y * 2 + 1),     // sw
            3 => (level + 1, x * 2 + 1, y * 2 + 1), // se
            _ => unreachable!(),
        }
    }

    /// Marks a rectangular range of tiles at a particular level as being available.
    pub fn add_available_tile_range(
        &mut self,
        level: i32,
        start_x: i32,
        start_y: i32,
        end_x: i32,
        end_y: i32,
    ) {
        if level == 0 {
            for y in start_y..=end_y {
                for x in start_x..=end_x {
                    let key = (0, x, y);
                    if !self.root_nodes.contains(&key) {
                        self.get_or_create_node(0, x, y);
                        self.root_nodes.push(key);
                    }
                }
            }
        }

        let mut rect_scratch = Rectangle::from_radians(0.0, 0.0, 0.0, 0.0);
        self.tiling_scheme
            .tile_xy_to_rectangle(start_x, start_y, level, &mut rect_scratch);
        let west = rect_scratch.west;
        let north = rect_scratch.north;

        self.tiling_scheme
            .tile_xy_to_rectangle(end_x, end_y, level, &mut rect_scratch);
        let east = rect_scratch.east;
        let south = rect_scratch.south;

        let rectangle_with_level = RectangleWithLevel {
            level,
            west,
            south,
            east,
            north,
        };

        let max_level = self.maximum_level;
        let root_keys = self.root_nodes.clone();
        for root_key in &root_keys {
            let root_extent = self.nodes[root_key].extent;
            if rect_rectangles_overlap(
                &root_extent,
                west,
                south,
                east,
                north,
            ) {
                Self::put_rectangle_in_quadtree_inner(
                    &mut self.nodes,
                    max_level,
                    root_key,
                    &mut self.tiling_scheme,
                    rectangle_with_level.level,
                    rectangle_with_level.west,
                    rectangle_with_level.south,
                    rectangle_with_level.east,
                    rectangle_with_level.north,
                );
            }
        }
    }

    fn put_rectangle_in_quadtree_inner(
        nodes: &mut HashMap<NodeKey, QuadtreeNodeData>,
        max_depth: i32,
        start_node: &NodeKey,
        tiling_scheme: &mut Box<dyn TilingScheme>,
        level: i32,
        west: f64,
        south: f64,
        east: f64,
        north: f64,
    ) {
        let mut current = *start_node;
        while current.0 < max_depth {
            let mut placed = false;
            for idx in 0..4 {
                let child = Self::child_key(&current, idx);
                // Ensure child exists.
                if !nodes.contains_key(&child) {
                    let mut extent = Rectangle::from_radians(0.0, 0.0, 0.0, 0.0);
                    tiling_scheme.tile_xy_to_rectangle(child.1, child.2, child.0, &mut extent);
                    nodes.insert(
                        child,
                        QuadtreeNodeData {
                            extent,
                            rectangles: Vec::new(),
                        },
                    );
                }
                let child_extent = nodes[&child].extent;
                if rectangle_fully_contains_rect(&child_extent, west, south, east, north) {
                    current = child;
                    placed = true;
                    break;
                }
            }
            if !placed {
                break;
            }
        }

        let node = nodes.get_mut(&current).unwrap();
        if node.rectangles.is_empty() || node.rectangles.last().unwrap().level <= level {
            node.rectangles.push(RectangleWithLevel {
                level,
                west,
                south,
                east,
                north,
            });
        } else {
            let index = binary_search::binary_search(
                &node.rectangles,
                &level,
                |a: &RectangleWithLevel, b: &i32| a.level as f64 - *b as f64,
            );
            let insert_idx = if index < 0 {
                (!index) as usize
            } else {
                index as usize
            };
            node.rectangles.insert(
                insert_idx,
                RectangleWithLevel {
                    level,
                    west,
                    south,
                    east,
                    north,
                },
            );
        }
    }

    /// Determines the level of the most detailed tile covering the position.
    pub fn compute_maximum_level_at_position(&self, position: &Cartographic) -> i32 {
        for root_key in &self.root_nodes {
            let root_extent = &self.nodes[root_key].extent;
            if rect_contains_cartographic(root_extent, position) {
                return self.find_max_level_from_node(root_key, position);
            }
        }
        -1
    }

    fn find_max_level_from_node(&self, start_node: &NodeKey, position: &Cartographic) -> i32 {
        let mut max_level = 0i32;

        // Find the deepest quadtree node containing this point.
        let mut current = *start_node;
        loop {
            let mut found_count = 0u32;
            let mut found_child = current;

            for idx in 0..4 {
                let child = Self::child_key(&current, idx);
                if let Some(child_data) = self.nodes.get(&child) {
                    if rect_contains_cartographic(&child_data.extent, position) {
                        found_count += 1;
                        found_child = child;
                    }
                }
            }

            if found_count > 1 {
                // Position is on a boundary - use recursion for each containing child.
                for idx in 0..4 {
                    let child = Self::child_key(&current, idx);
                    if let Some(child_data) = self.nodes.get(&child) {
                        if rect_contains_cartographic(&child_data.extent, position) {
                            let sub = self.find_max_level_from_node(&child, position);
                            max_level = max_level.max(sub);
                        }
                    }
                }
                break;
            } else if found_count == 1 {
                current = found_child;
            } else {
                break;
            }
        }

        // Check rectangles at the final node.
        if let Some(node_data) = self.nodes.get(&current) {
            for r in node_data.rectangles.iter().rev() {
                if r.level > max_level && rectangle_with_level_contains_position(r, position) {
                    max_level = r.level;
                }
            }
        }

        max_level
    }

    /// Determines if a particular tile is available.
    pub fn is_tile_available(&self, level: i32, x: i32, y: i32) -> bool {
        let mut rect_scratch = Rectangle::from_radians(0.0, 0.0, 0.0, 0.0);
        self.tiling_scheme
            .tile_xy_to_rectangle(x, y, level, &mut rect_scratch);
        let center = Rectangle::center(&rect_scratch);
        self.compute_maximum_level_at_position(&center) >= level
    }

    /// Computes a bit mask indicating which of a tile's four children exist.
    pub fn compute_child_mask_for_tile(&self, level: i32, x: i32, y: i32) -> u8 {
        let child_level = level + 1;
        if child_level >= self.maximum_level {
            return 0;
        }

        let mut mask = 0u8;
        if self.is_tile_available(child_level, 2 * x, 2 * y + 1) {
            mask |= 1; // SW
        }
        if self.is_tile_available(child_level, 2 * x + 1, 2 * y + 1) {
            mask |= 2; // SE
        }
        if self.is_tile_available(child_level, 2 * x, 2 * y) {
            mask |= 4; // NW
        }
        if self.is_tile_available(child_level, 2 * x + 1, 2 * y) {
            mask |= 8; // NE
        }
        mask
    }
}

fn rect_rectangles_overlap(
    rect: &Rectangle,
    other_west: f64,
    other_south: f64,
    other_east: f64,
    other_north: f64,
) -> bool {
    let west = rect.west.max(other_west);
    let south = rect.south.max(other_south);
    let east = rect.east.min(other_east);
    let north = rect.north.min(other_north);
    south < north && west < east
}

fn rectangle_fully_contains_rect(
    container: &Rectangle,
    west: f64,
    south: f64,
    east: f64,
    north: f64,
) -> bool {
    west >= container.west && east <= container.east && south >= container.south && north <= container.north
}

fn rect_contains_cartographic(rect: &Rectangle, pos: &Cartographic) -> bool {
    pos.longitude >= rect.west
        && pos.longitude <= rect.east
        && pos.latitude >= rect.south
        && pos.latitude <= rect.north
}

fn rectangle_with_level_contains_position(
    container: &RectangleWithLevel,
    pos: &Cartographic,
) -> bool {
    pos.longitude >= container.west
        && pos.longitude <= container.east
        && pos.latitude >= container.south
        && pos.latitude <= container.north
}
