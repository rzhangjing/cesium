//! Ported from `packages/engine/Source/Core/Credit.js`.
//!
//! DEVIATION: JS Credit uses DOMPurify + DOM element creation for HTML sanitization.
//! In Rust we store the HTML string and show_on_screen flag without DOM rendering.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_CREDIT_ID: AtomicU64 = AtomicU64::new(0);

fn credit_to_id() -> &'static std::sync::Mutex<HashMap<String, u64>> {
    static MAP: std::sync::OnceLock<std::sync::Mutex<HashMap<String, u64>>> =
        std::sync::OnceLock::new();
    MAP.get_or_init(|| std::sync::Mutex::new(HashMap::new()))
}

/// A credit contains data pertaining to how to display attributions/credits.
#[derive(Debug, Clone)]
pub struct Credit {
    id: u64,
    html: String,
    show_on_screen: bool,
}

impl Credit {
    pub fn new(html: &str, show_on_screen: bool) -> Self {
        let mut map = credit_to_id().lock().unwrap();
        let id = if let Some(&existing) = map.get(html) {
            existing
        } else {
            let id = NEXT_CREDIT_ID.fetch_add(1, Ordering::Relaxed);
            map.insert(html.to_string(), id);
            id
        };

        Self {
            id,
            html: html.to_string(),
            show_on_screen,
        }
    }

    /// The credit content (HTML string).
    pub fn html(&self) -> &str {
        &self.html
    }

    /// The unique id for this credit content.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Whether the credit should be displayed on screen or in a lightbox.
    pub fn show_on_screen(&self) -> bool {
        self.show_on_screen
    }

    /// Sets whether the credit should be displayed on screen.
    pub fn set_show_on_screen(&mut self, value: bool) {
        self.show_on_screen = value;
    }

    /// Returns true if the credits are equal.
    pub fn equals(left: &Self, right: &Self) -> bool {
        left.id == right.id && left.show_on_screen == right.show_on_screen
    }

    /// Duplicates a Credit instance.
    pub fn clone_credit(&self) -> Self {
        Self::new(&self.html, self.show_on_screen)
    }

    /// Creates a Credit from a geocoder result attribution.
    ///
    /// Port of `Credit.getIonCredit`: `showOnScreen` is true only when
    /// `collapsible` is defined and false.
    pub fn get_ion_credit(attribution: &crate::geocoder_service::GeocoderAttribution) -> Self {
        let show_on_screen =
            attribution.collapsible.is_some() && !attribution.collapsible.unwrap();
        Self::new(&attribution.html, show_on_screen)
    }
}

impl PartialEq for Credit {
    fn eq(&self, other: &Self) -> bool {
        Self::equals(self, other)
    }
}
