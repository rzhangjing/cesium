//! Ported from `packages/engine/Source/Core/TileProviderError.js`.

/// Provides details about an error that occurred in an imagery or terrain provider.
pub struct TileProviderError {
    /// The message describing the error.
    pub message: String,
    /// The X coordinate of the tile that experienced the error.
    pub x: Option<i32>,
    /// The Y coordinate of the tile that experienced the error.
    pub y: Option<i32>,
    /// The level-of-detail of the tile that experienced the error.
    pub level: Option<i32>,
    /// The number of times this operation has been retried.
    pub times_retried: i32,
    /// Whether the failed operation should be retried.
    pub retry: bool,
}

impl TileProviderError {
    /// Creates a new TileProviderError.
    pub fn new(
        message: String,
        x: Option<i32>,
        y: Option<i32>,
        level: Option<i32>,
        times_retried: Option<i32>,
    ) -> Self {
        Self {
            message,
            x,
            y,
            level,
            times_retried: times_retried.unwrap_or(0),
            retry: false,
        }
    }

    /// Reports an error, creating or updating a TileProviderError.
    pub fn report_error(
        previous_error: Option<TileProviderError>,
        message: String,
        x: Option<i32>,
        y: Option<i32>,
        level: Option<i32>,
    ) -> TileProviderError {
        match previous_error {
            Some(mut err) => {
                err.message = message;
                err.x = x;
                err.y = y;
                err.level = level;
                err.retry = false;
                err.times_retried += 1;
                err
            }
            None => Self::new(message, x, y, level, Some(0)),
        }
    }
}
