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

// ---------------------------------------------------------------------------
// DOM adaptation layer.
//
// CesiumJS widgets receive DOM `Element`s (or element ids resolved through
// `getElement`) and query `document.body`. The Rust port has no DOM; the
// types below are the mock substrate the widget ViewModels operate on.
// They live in this module because `knockout.rs` is the crate's designated
// UI-adaptation module (there is no CesiumJS file mirrored by it).
// ---------------------------------------------------------------------------

use std::collections::HashMap;

/// Mock DOM element (Rust analogue of a browser `Element`).
///
/// Carries only the properties the widget ViewModels actually read:
/// identity (`id`, `tag`), layout metrics used by
/// `SelectionIndicatorViewModel.update` (`client_width`/`client_height` and
/// the parent node's client size), nothing more.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockDomElement {
    /// The element id (mirrors `element.id`).
    pub id: String,
    /// The tag name, e.g. `"div"`, `"span"`, `"body"`.
    pub tag: String,
    /// Mirrors `element.clientWidth`.
    pub client_width: i32,
    /// Mirrors `element.clientHeight`.
    pub client_height: i32,
    /// Mirrors `element.parentNode.clientWidth`.
    pub parent_client_width: i32,
    /// Mirrors `element.parentNode.clientHeight`.
    pub parent_client_height: i32,
}

impl MockDomElement {
    /// Creates an element with the given tag and no id.
    pub fn new(tag: &str) -> Self {
        Self {
            id: String::new(),
            tag: tag.to_string(),
            client_width: 0,
            client_height: 0,
            parent_client_width: 0,
            parent_client_height: 0,
        }
    }

    /// Builder-style id setter.
    #[must_use]
    pub fn with_id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    /// Builder-style client size setter.
    #[must_use]
    pub fn with_client_size(mut self, width: i32, height: i32) -> Self {
        self.client_width = width;
        self.client_height = height;
        self
    }

    /// Builder-style parent client size setter.
    #[must_use]
    pub fn with_parent_client_size(mut self, width: i32, height: i32) -> Self {
        self.parent_client_width = width;
        self.parent_client_height = height;
        self
    }
}

/// Mock document (Rust analogue of `document`): a `body` element plus a
/// registry of elements addressable by id (mirrors `getElementById`).
#[derive(Debug, Clone)]
pub struct MockDocument {
    body: MockDomElement,
    elements: HashMap<String, MockDomElement>,
}

impl MockDocument {
    /// Creates a document with a `body` element.
    pub fn new() -> Self {
        Self {
            body: MockDomElement::new("body"),
            elements: HashMap::new(),
        }
    }

    /// The `document.body` element.
    pub fn body(&self) -> &MockDomElement {
        &self.body
    }

    /// Appends an element to the document (mirrors
    /// `document.body.appendChild`).
    pub fn append(&mut self, element: MockDomElement) {
        self.elements.insert(element.id.clone(), element);
    }

    /// Removes the element with the given id (mirrors
    /// `document.body.removeChild`).
    pub fn remove(&mut self, id: &str) -> Option<MockDomElement> {
        self.elements.remove(id)
    }

    /// Mirrors `document.getElementById`.
    pub fn get_element_by_id(&self, id: &str) -> Option<&MockDomElement> {
        self.elements.get(id)
    }
}

impl Default for MockDocument {
    fn default() -> Self {
        Self::new()
    }
}

/// Argument form accepted by widget constructors that take
/// `Element|string` in CesiumJS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElementOrId {
    /// A concrete element (JS `Element` argument).
    Element(MockDomElement),
    /// An element id to resolve through the document (JS `string` argument).
    Id(String),
}

/// Rust analogue of CesiumJS Core `getElement(elementOrId)`:
/// resolves an element or an element id (looked up in `document`) to an
/// element. Returns `None` when given no argument; throws `DeveloperError`
/// when an id does not resolve, mirroring the JS behaviour.
pub fn get_element(
    document: &MockDocument,
    element_or_id: Option<&ElementOrId>,
) -> Option<MockDomElement> {
    match element_or_id {
        None => None,
        Some(ElementOrId::Element(element)) => Some(element.clone()),
        Some(ElementOrId::Id(id)) => {
            if let Some(element) = document.get_element_by_id(id) {
                return Some(element.clone());
            }
            if id == "body" {
                return Some(document.body().clone());
            }
            cesium_core::developer_error::throw_developer_error(
                "id must be a valid element.",
            )
        }
    }
}

// DEVIATION: CesiumJS widgets query `Fullscreen.enabled` /
// `Fullscreen.fullscreen` statics from `@cesium/engine`. The cesium-core
// `Fullscreen` port is currently a stub without those statics (missing Core
// API, reported to the leader for Track B), so the widget ViewModels query
// this widget-local capability surface instead. Headless Rust has no
// fullscreen API, so both report `false`.

/// Rust analogue of `Fullscreen.enabled` (headless: always `false`).
pub fn fullscreen_enabled() -> bool {
    false
}

/// Rust analogue of `Fullscreen.fullscreen` (headless: always `false`).
pub fn fullscreen_active() -> bool {
    false
}
