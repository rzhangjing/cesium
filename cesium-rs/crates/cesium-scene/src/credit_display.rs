//! Ported from `packages/engine/Source/Scene/CreditDisplay.js`.
//!
//! M3/S3 materialization: the CesiumJS `CreditDisplay` (frameState's
//! `creditDisplay`, consumed by the widgets through
//! `addStaticCredit`/`removeStaticCredit`/`isDestroyed`) is ported
//! one-to-one: per-frame credits (`addCredit`), persistent static credits
//! that reappear every frame, the `beginFrame` rotation (previous ←
//! current; current ← static copy), and `destroy`/`isDestroyed`.
//!
//! DEVIATION: the JS takes a DOM `container` and renders the credit HTML
//! into it; the headless port keeps the credit accounting only (the
//! widgets read the accounting, e.g. the Geocoder's ion static credit).

use cesium_core::credit::Credit;

/// Manages display of data attribution credits.
///
/// Collects credits from imagery providers, terrain providers, and other
/// data sources, and manages their display on screen.
pub struct CreditDisplay {
    current_credits: Vec<Credit>,
    previous_credits: Vec<Credit>,
    static_credits: Vec<Credit>,
    show_on_screen: bool,
    is_destroyed: bool,
}

impl CreditDisplay {
    /// Creates a new credit display.
    pub fn new() -> Self {
        Self {
            current_credits: Vec::new(),
            previous_credits: Vec::new(),
            static_credits: Vec::new(),
            show_on_screen: false,
            is_destroyed: false,
        }
    }

    /// Adds a credit for the current frame (mirrors the JS `addCredit`;
    /// duplicate credits — same id — are only recorded once per frame).
    pub fn add_credit(&mut self, credit: Credit) {
        if !self.current_credits.iter().any(|existing| *existing == credit) {
            self.current_credits.push(credit);
        }
    }

    /// Adds a credit that is displayed every frame (mirrors the JS
    /// `addStaticCredit`).
    pub fn add_static_credit(&mut self, credit: Credit) {
        if !self.static_credits.iter().any(|existing| *existing == credit) {
            self.static_credits.push(credit);
        }
    }

    /// Removes a static credit (mirrors the JS `removeStaticCredit`).
    pub fn remove_static_credit(&mut self, credit: &Credit) {
        self.static_credits.retain(|existing| existing != credit);
    }

    /// Returns the current frame's credits.
    pub fn current_credits(&self) -> &[Credit] {
        &self.current_credits
    }

    /// Returns the previous frame's credits.
    pub fn previous_credits(&self) -> &[Credit] {
        &self.previous_credits
    }

    /// Returns the static credits (shown every frame).
    pub fn static_credits(&self) -> &[Credit] {
        &self.static_credits
    }

    /// Returns whether credits should be shown on screen.
    pub fn show_on_screen(&self) -> bool {
        self.show_on_screen
    }

    /// Sets whether credits should be shown on screen.
    pub fn set_show_on_screen(&mut self, value: bool) {
        self.show_on_screen = value;
    }

    /// Begins a new frame (mirrors the JS `beginFrame`): the current
    /// credits become the previous credits, and the new current credits
    /// start from a copy of the static credits.
    pub fn begin_frame(&mut self) {
        self.previous_credits = std::mem::take(&mut self.current_credits);
        self.current_credits = self.static_credits.clone();
    }

    /// Ends the current frame.
    pub fn end_frame(&mut self) {}

    /// Destroys the credit display (mirrors the JS `destroy`).
    pub fn destroy(&mut self) {
        self.is_destroyed = true;
    }

    /// Returns whether the credit display has been destroyed (mirrors the
    /// JS `isDestroyed`).
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }
}

impl Default for CreditDisplay {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mirrors CreditDisplaySpec: per-frame credits are deduplicated and
    /// rotated to previous on beginFrame.
    #[test]
    fn add_credit_rotates_each_frame() {
        let mut display = CreditDisplay::new();
        display.add_credit(Credit::new("a", false));
        display.add_credit(Credit::new("a", false));
        display.add_credit(Credit::new("b", false));
        assert_eq!(display.current_credits().len(), 2);

        display.begin_frame();
        assert_eq!(display.previous_credits().len(), 2);
        assert!(display.current_credits().is_empty());
    }

    /// Mirrors CreditDisplaySpec "addStaticCredit/removeStaticCredit":
    /// static credits reappear every frame until removed.
    #[test]
    fn static_credits_persist_across_frames() {
        let mut display = CreditDisplay::new();
        let credit = Credit::new("ion", true);
        display.add_static_credit(credit.clone_credit());
        // Duplicate adds are ignored.
        display.add_static_credit(credit.clone_credit());
        assert_eq!(display.static_credits().len(), 1);

        display.begin_frame();
        assert_eq!(display.current_credits().len(), 1);
        display.begin_frame();
        assert_eq!(display.current_credits().len(), 1);

        display.remove_static_credit(&credit);
        assert!(display.static_credits().is_empty());
        display.begin_frame();
        assert!(display.current_credits().is_empty());
    }

    /// Mirrors CreditDisplaySpec: destroy flips isDestroyed.
    #[test]
    fn destroy_marks_the_display_destroyed() {
        let mut display = CreditDisplay::new();
        assert!(!display.is_destroyed());
        display.destroy();
        assert!(display.is_destroyed());
    }
}
