//! Ported from `packages/engine/Source/Scene/CreditDisplay.js`.
//!
//! Manages display of data attribution credits.

use cesium_core::credit::Credit;
use std::collections::HashSet;

/// Manages display of data attribution credits.
///
/// Collects credits from imagery providers, terrain providers, and other
/// data sources, and manages their display on screen.
pub struct CreditDisplay {
    current_credits: Vec<Credit>,
    previous_credits: Vec<Credit>,
    show_on_screen: bool,
}

impl CreditDisplay {
    /// Creates a new credit display.
    pub fn new() -> Self {
        Self {
            current_credits: Vec::new(),
            previous_credits: Vec::new(),
            show_on_screen: false,
        }
    }

    /// Adds a credit for the current frame.
    pub fn add_credit(&mut self, credit: Credit) {
        self.current_credits.push(credit);
    }

    /// Returns the current frame's credits.
    pub fn current_credits(&self) -> &[Credit] {
        &self.current_credits
    }

    /// Returns whether credits should be shown on screen.
    pub fn show_on_screen(&self) -> bool {
        self.show_on_screen
    }

    /// Sets whether credits should be shown on screen.
    pub fn set_show_on_screen(&mut self, value: bool) {
        self.show_on_screen = value;
    }

    /// Begins a new frame, moving current credits to previous.
    pub fn begin_frame(&mut self) {
        self.previous_credits.clear();
        std::mem::swap(&mut self.previous_credits, &mut self.current_credits);
    }

    /// Ends the current frame.
    pub fn end_frame(&mut self) {}
}

impl Default for CreditDisplay {
    fn default() -> Self { Self::new() }
}
