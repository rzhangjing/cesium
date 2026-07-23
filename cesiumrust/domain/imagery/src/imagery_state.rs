//! Imagery state machine.
//! Maps to CesiumJS `Scene/ImageryState.js`

use serde::{Deserialize, Serialize};

/// The state of an imagery tile.
/// Maps to CesiumJS `ImageryState`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ImageryState {
    /// Imagery has not been requested yet.
    #[default]
    Unloaded,
    /// Imagery request is in progress.
    Transitioning,
    /// Imagery data has been received but not yet processed.
    Received,
    /// Texture has been loaded but not yet ready.
    TextureLoaded,
    /// Imagery is ready for rendering.
    Ready,
    /// Imagery request failed.
    Failed,
    /// Imagery is invalid (e.g., wrong format).
    Invalid,
    /// Placeholder imagery (used while loading).
    Placeholder,
}

impl ImageryState {
    /// Returns true if the imagery is in a terminal state (Ready, Failed, Invalid).
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Ready | Self::Failed | Self::Invalid)
    }

    /// Returns true if the imagery can be rendered.
    pub fn is_renderable(&self) -> bool {
        matches!(self, Self::Ready | Self::Placeholder)
    }

    /// Returns true if a request should be made for this state.
    pub fn should_request(&self) -> bool {
        matches!(self, Self::Unloaded | Self::Failed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_terminal() {
        assert!(ImageryState::Ready.is_terminal());
        assert!(ImageryState::Failed.is_terminal());
        assert!(ImageryState::Invalid.is_terminal());
        assert!(!ImageryState::Unloaded.is_terminal());
        assert!(!ImageryState::Transitioning.is_terminal());
    }

    #[test]
    fn test_is_renderable() {
        assert!(ImageryState::Ready.is_renderable());
        assert!(ImageryState::Placeholder.is_renderable());
        assert!(!ImageryState::Unloaded.is_renderable());
        assert!(!ImageryState::Failed.is_renderable());
    }

    #[test]
    fn test_should_request() {
        assert!(ImageryState::Unloaded.should_request());
        assert!(ImageryState::Failed.should_request());
        assert!(!ImageryState::Ready.should_request());
        assert!(!ImageryState::Transitioning.should_request());
    }
}
