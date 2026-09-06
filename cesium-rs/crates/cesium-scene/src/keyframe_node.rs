//! Ported from `packages/engine/Source/Scene/KeyframeNode.js`.
//!
//! A keyframe node within a spatial keyframe data structure.
//! Tracks loading state, content, priority, and megatexture placement.

/// Load state of a keyframe node (mirrors CesiumJS `LoadState` enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum KeyframeLoadState {
    /// Has no data and is in dormant state.
    Unloaded = 0,
    /// Is waiting on data from the provider.
    Receiving = 1,
    /// Data received; contents are being processed for rendering.
    Processing = 2,
    /// Processed data from provider.
    Loaded = 3,
    /// Failed to receive data from the provider.
    Failed = 4,
    /// No data available for this tile.
    Unavailable = 5,
}

impl KeyframeLoadState {
    /// Converts from the integer representation.
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Unloaded),
            1 => Some(Self::Receiving),
            2 => Some(Self::Processing),
            3 => Some(Self::Loaded),
            4 => Some(Self::Failed),
            5 => Some(Self::Unavailable),
            _ => None,
        }
    }
}

/// A keyframe node within a spatial keyframe data structure.
///
/// Mirrors CesiumJS `KeyframeNode` (62 lines):
/// - `spatial_node_index`: index into the parent spatial node array
/// - `keyframe`: the keyframe index
/// - `state`: current loading state
/// - `content`: opaque reference to loaded content
/// - `megatexture_index`: placement index in the megatexture (-1 = not placed)
/// - `priority`: scheduling priority
/// - `high_priority_frame_number`: frame at which this node was last high-priority
#[derive(Debug, Clone)]
pub struct KeyframeNode {
    /// Index of the spatial node this keyframe belongs to.
    pub spatial_node_index: usize,
    /// The keyframe index.
    pub keyframe: u32,
    /// Current load state.
    pub state: KeyframeLoadState,
    /// Opaque content reference (DEVIATION: JS uses `undefined`; Rust uses `None`).
    pub content: Option<u64>,
    /// Index in the megatexture (-1 = not yet placed).
    pub megatexture_index: i32,
    /// Scheduling priority.
    pub priority: f64,
    /// Frame number at which this node was last assigned high priority.
    pub high_priority_frame_number: i64,
}

impl KeyframeNode {
    /// Creates a new `KeyframeNode` in the `Unloaded` state.
    pub fn new(spatial_node_index: usize, keyframe: u32) -> Self {
        Self {
            spatial_node_index,
            keyframe,
            state: KeyframeLoadState::Unloaded,
            content: None,
            megatexture_index: -1,
            priority: f64::MIN,
            high_priority_frame_number: -1,
        }
    }

    /// Frees resources and resets to the `Unloaded` state
    /// (mirrors CesiumJS `KeyframeNode#unload`).
    pub fn unload(&mut self) {
        self.content = None;
        self.state = KeyframeLoadState::Unloaded;
        self.megatexture_index = -1;
        self.priority = f64::MIN;
        self.high_priority_frame_number = -1;
    }

    /// Compares two nodes by priority (ascending) for scheduling
    /// (mirrors CesiumJS `KeyframeNode.priorityComparator`).
    pub fn priority_compare(a: &KeyframeNode, b: &KeyframeNode) -> std::cmp::Ordering {
        a.priority.partial_cmp(&b.priority).unwrap_or(std::cmp::Ordering::Equal)
    }

    /// Compares two nodes by keyframe (ascending) for search
    /// (mirrors CesiumJS `KeyframeNode.searchComparator`).
    pub fn search_compare(a: &KeyframeNode, b: &KeyframeNode) -> std::cmp::Ordering {
        a.keyframe.cmp(&b.keyframe)
    }
}

impl Default for KeyframeNode {
    fn default() -> Self {
        Self::new(0, 0)
    }
}
