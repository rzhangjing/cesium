//! Ported from Knockout.js data-binding used in CesiumJS widgets.
//!
//! In CesiumJS, widgets use Knockout.js for MVVM data binding between
//! ViewModels and DOM elements. In Rust, we replace this with a
//! `DomSurface` trait that abstracts the UI surface.

/// Trait for abstracting the DOM/UI surface.
///
/// In CesiumJS, widgets bind to DOM elements via Knockout.js observables.
/// In Rust, this trait provides the abstraction layer for UI backends:
/// - `winit` for native desktop windows
/// - `web-sys` for WASM/browser
/// - Mock implementations for testing
///
/// Each widget's ViewModel interacts with the DomSurface to:
/// - Read/write element properties (visibility, text, class)
/// - Register event handlers (click, input, resize)
/// - Create child elements
pub trait DomSurface {
    /// Returns the width of the surface in logical pixels.
    fn width(&self) -> u32;

    /// Returns the height of the surface in logical pixels.
    fn height(&self) -> u32;

    /// Returns the device pixel ratio.
    fn pixel_ratio(&self) -> f64;

    /// Returns whether the surface is visible.
    fn is_visible(&self) -> bool;

    /// Sets the visibility of the surface.
    fn set_visible(&mut self, visible: bool);

    /// Requests a redraw of the surface.
    fn request_redraw(&mut self);
}

/// A mock DOM surface for testing.
pub struct MockDomSurface {
    width: u32,
    height: u32,
    pixel_ratio: f64,
    visible: bool,
}

impl MockDomSurface {
    /// Creates a new mock surface with the given dimensions.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixel_ratio: 1.0,
            visible: true,
        }
    }
}

impl DomSurface for MockDomSurface {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn pixel_ratio(&self) -> f64 {
        self.pixel_ratio
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    fn request_redraw(&mut self) {
        // No-op for mock
    }
}

impl Default for MockDomSurface {
    fn default() -> Self {
        Self::new(800, 600)
    }
}

/// Trait for observable properties (replaces Knockout observables).
///
/// In CesiumJS, Knockout observables automatically notify subscribers
/// when values change. In Rust, we use this trait to model that pattern.
pub trait Observable<T> {
    /// Gets the current value.
    fn get(&self) -> &T;

    /// Sets the value and notifies subscribers.
    fn set(&mut self, value: T);

    /// Subscribes to value changes.
    ///
    /// Returns a subscription ID that can be used to unsubscribe.
    fn subscribe(&mut self, callback: Box<dyn Fn(&T) + Send + Sync>) -> u64;

    /// Unsubscribes from value changes.
    fn unsubscribe(&mut self, subscription_id: u64);
}

/// A simple observable implementation using a callback list.
pub struct SimpleObservable<T: Clone> {
    value: T,
    callbacks: Vec<(u64, Box<dyn Fn(&T) + Send + Sync>)>,
    next_id: u64,
}

impl<T: Clone> SimpleObservable<T> {
    /// Creates a new observable with the given initial value.
    pub fn new(initial: T) -> Self {
        Self {
            value: initial,
            callbacks: Vec::new(),
            next_id: 0,
        }
    }
}

impl<T: Clone + Send + Sync + 'static> Observable<T> for SimpleObservable<T> {
    fn get(&self) -> &T {
        &self.value
    }

    fn set(&mut self, value: T) {
        self.value = value.clone();
        for (_, callback) in &self.callbacks {
            callback(&self.value);
        }
    }

    fn subscribe(&mut self, callback: Box<dyn Fn(&T) + Send + Sync>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.callbacks.push((id, callback));
        id
    }

    fn unsubscribe(&mut self, subscription_id: u64) {
        self.callbacks.retain(|(id, _)| *id != subscription_id);
    }
}
