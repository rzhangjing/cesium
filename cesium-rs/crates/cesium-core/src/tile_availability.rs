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

#[derive(Clone)]
struct RectangleWithLevel {
    level: i32,
    west: f64,
    south: f64,
    east: f64,
    north: f64,
}

struct QuadtreeNodeData {
    extent: Rectangle,
    parent: Option<NodeKey>,
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

    /// The maximum level for which availability is tracked.
    ///
    /// Mirrors the JS private `_maximumLevel` field (exposed here because
    /// `CesiumTerrainProvider.getTileDataAvailable` reads it).
    pub fn maximum_level(&self) -> i32 {
        self.maximum_level
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
                    parent: None,
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
                            parent: Some(current),
                            rectangles: Vec::new(),
                        },
                    );
                }
                let child_extent = nodes[&child].extent;
                // JS: `rectangleFullyContainsRectangle(node.nw.extent, rectangle)`
                // — descend while the child extent fully contains the rectangle.
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
        // Mirrors `findMaxLevelFromNode(undefined, node, position)`: descend
        // to the deepest node containing the position, then work back up the
        // parent chain checking the rectangles stored at every visited node
        // (positions on tile boundaries may be covered by a rectangle stored
        // at an ancestor).
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
                // Position is on a boundary - use recursion for each containing
                // child. Mirrors the JS recursion with `stopNode = node`, so
                // each sub-search walks back up only to `current`.
                for idx in 0..4 {
                    let child = Self::child_key(&current, idx);
                    if let Some(child_data) = self.nodes.get(&child) {
                        if rect_contains_cartographic(&child_data.extent, position) {
                            let sub =
                                self.find_max_level_from_node_with_stop(&child, &current, position);
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

        // Work up the tree until we find a rectangle that contains this
        // point (JS `stopNode` is undefined here, so walk all the way to
        // and including the root).
        let mut node = current;
        loop {
            self.check_node_rectangles(&node, position, &mut max_level);
            let Some(parent) = self.nodes[&node].parent else {
                break;
            };
            node = parent;
        }

        max_level
    }

    /// Checks the rectangles of `node` (sorted by level, lowest first)
    /// against the position, updating `max_level`.
    fn check_node_rectangles(&self, node: &NodeKey, position: &Cartographic, max_level: &mut i32) {
        if let Some(node_data) = self.nodes.get(node) {
            for r in node_data.rectangles.iter().rev() {
                if r.level <= *max_level {
                    break;
                }
                if rectangle_with_level_contains_position(r, position) {
                    *max_level = r.level;
                }
            }
        }
    }

    /// Mirrors the JS boundary-recursion `findMaxLevelFromNode(stopNode, node, position)`.
    fn find_max_level_from_node_with_stop(
        &self,
        start_node: &NodeKey,
        stop_node: &NodeKey,
        position: &Cartographic,
    ) -> i32 {
        let mut max_level = 0i32;

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
                for idx in 0..4 {
                    let child = Self::child_key(&current, idx);
                    if let Some(child_data) = self.nodes.get(&child) {
                        if rect_contains_cartographic(&child_data.extent, position) {
                            let sub =
                                self.find_max_level_from_node_with_stop(&child, &current, position);
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

        let mut node = current;
        while node != *stop_node {
            self.check_node_rectangles(&node, position, &mut max_level);
            let Some(parent) = self.nodes[&node].parent else {
                break;
            };
            node = parent;
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

    /// Finds the most detailed level that is available _everywhere_ within a
    /// given rectangle. More detailed tiles may be available in parts of the
    /// rectangle, but not the whole thing.
    pub fn compute_best_available_level_over_rectangle(
        &self,
        rectangle: &Rectangle,
    ) -> i32 {
        let mut rectangles: Vec<Rectangle> = Vec::new();

        if rectangle.east < rectangle.west {
            // Rectangle crosses the IDL, make it two rectangles.
            rectangles.push(Rectangle::from_radians(
                -std::f64::consts::PI,
                rectangle.south,
                rectangle.east,
                rectangle.north,
            ));
            rectangles.push(Rectangle::from_radians(
                rectangle.west,
                rectangle.south,
                std::f64::consts::PI,
                rectangle.north,
            ));
        } else {
            rectangles.push(*rectangle);
        }

        // Mirrors the sparse JS `remainingToCoverByLevel` array indexed by level.
        let mut remaining_to_cover_by_level: Vec<Option<Vec<Rectangle>>> = Vec::new();

        let root_keys = self.root_nodes.clone();
        for root_key in &root_keys {
            Self::update_coverage_with_node(
                self,
                &mut remaining_to_cover_by_level,
                root_key,
                &rectangles,
            );
        }

        for i in (0..remaining_to_cover_by_level.len()).rev() {
            if let Some(list) = &remaining_to_cover_by_level[i] {
                if list.is_empty() {
                    return i as i32;
                }
            }
        }

        0
    }

    /// Test support mirroring the JS spec's `checkNodeRectanglesSorted`
    /// helper, which walks the private `_rootNodes`/`_nw`/`_ne`/`_sw`/`_se`
    /// fields to verify that every node's rectangle list stays sorted by level.
    #[doc(hidden)]
    pub fn debug_check_node_rectangles_sorted(&self) -> bool {
        for root_key in &self.root_nodes {
            if !self.debug_check_node_sorted(root_key) {
                return false;
            }
        }
        true
    }

    fn debug_check_node_sorted(&self, node_key: &NodeKey) -> bool {
        let Some(node) = self.nodes.get(node_key) else {
            return true;
        };
        let level_rectangles = &node.rectangles;
        for i in 0..level_rectangles.len() {
            for j in i..level_rectangles.len() {
                if !(level_rectangles[i].level <= level_rectangles[j].level) {
                    return false;
                }
            }
        }
        for quadrant in 0..4 {
            let child = Self::child_key(node_key, quadrant);
            if !self.debug_check_node_sorted(&child) {
                return false;
            }
        }
        true
    }

    /// Mirrors the JS `updateCoverageWithNode` helper. Only children that
    /// already exist in the quadtree are visited (JS accesses the private
    /// `_nw`/`_ne`/`_sw`/`_se` fields, which stay `undefined` until created).
    fn update_coverage_with_node(
        &self,
        remaining_to_cover_by_level: &mut Vec<Option<Vec<Rectangle>>>,
        node_key: &NodeKey,
        rectangles_to_cover: &[Rectangle],
    ) {
        let Some(node) = self.nodes.get(node_key) else {
            return;
        };

        let mut any_overlap = false;
        for rectangle in rectangles_to_cover {
            any_overlap = any_overlap || rectangles_overlap(&node.extent, rectangle);
        }

        if !any_overlap {
            // This node is not applicable to the rectangle(s).
            return;
        }

        let rectangles = node.rectangles.clone();
        for rectangle in &rectangles {
            let level = rectangle.level as usize;
            if remaining_to_cover_by_level.len() <= level {
                remaining_to_cover_by_level.resize(level + 1, None);
            }
            if remaining_to_cover_by_level[level].is_none() {
                remaining_to_cover_by_level[level] = Some(rectangles_to_cover.to_vec());
            }

            let current = remaining_to_cover_by_level[level].take().unwrap();
            remaining_to_cover_by_level[level] =
                Some(subtract_rectangle(&current, rectangle));
        }

        // Update with child nodes.
        for quadrant in 0..4 {
            let child = Self::child_key(node_key, quadrant);
            self.update_coverage_with_node(
                remaining_to_cover_by_level,
                &child,
                rectangles_to_cover,
            );
        }
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

fn rectangles_overlap(rectangle1: &Rectangle, rectangle2: &Rectangle) -> bool {
    rect_rectangles_overlap(
        rectangle1,
        rectangle2.west,
        rectangle2.south,
        rectangle2.east,
        rectangle2.north,
    )
}

/// Mirrors the JS `subtractRectangle` helper: splits each rectangle in
/// `rectangle_list` around `rectangle_to_subtract`.
fn subtract_rectangle(
    rectangle_list: &[Rectangle],
    rectangle_to_subtract: &RectangleWithLevel,
) -> Vec<Rectangle> {
    let mut result: Vec<Rectangle> = Vec::new();
    for rectangle in rectangle_list {
        let overlaps = rect_rectangles_overlap(
            rectangle,
            rectangle_to_subtract.west,
            rectangle_to_subtract.south,
            rectangle_to_subtract.east,
            rectangle_to_subtract.north,
        );
        if !overlaps {
            // Disjoint rectangles. Original rectangle is unmodified.
            result.push(*rectangle);
        } else {
            // rectangleToSubtract partially or completely overlaps rectangle.
            if rectangle.west < rectangle_to_subtract.west {
                result.push(Rectangle::new(
                    rectangle.west,
                    rectangle.south,
                    rectangle_to_subtract.west,
                    rectangle.north,
                ));
            }
            if rectangle.east > rectangle_to_subtract.east {
                result.push(Rectangle::new(
                    rectangle_to_subtract.east,
                    rectangle.south,
                    rectangle.east,
                    rectangle.north,
                ));
            }
            if rectangle.south < rectangle_to_subtract.south {
                result.push(Rectangle::new(
                    rectangle_to_subtract.west.max(rectangle.west),
                    rectangle.south,
                    rectangle_to_subtract.east.min(rectangle.east),
                    rectangle_to_subtract.south,
                ));
            }
            if rectangle.north > rectangle_to_subtract.north {
                result.push(Rectangle::new(
                    rectangle_to_subtract.west.max(rectangle.west),
                    rectangle_to_subtract.north,
                    rectangle_to_subtract.east.min(rectangle.east),
                    rectangle.north,
                ));
            }
        }
    }

    result
}

/// Mirrors JS `rectangleFullyContainsRectangle(potentialContainer, rectangleToTest)`:
/// true when the rectangle (`west..east`, `south..north`) is fully
/// contained by `container`.
fn rectangle_fully_contains_rect(
    container: &Rectangle,
    west: f64,
    south: f64,
    east: f64,
    north: f64,
) -> bool {
    west >= container.west
        && east <= container.east
        && south >= container.south
        && north <= container.north
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
