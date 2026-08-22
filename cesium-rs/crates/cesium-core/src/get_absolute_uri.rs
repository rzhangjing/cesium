//! Ported from packages/engine/Source/Core/getAbsoluteUri.js
//!
//! DEVIATION: the JS version resolves against `document.baseURI` /
//! `document.location.href` when no base is given; native builds have no
//! document, so the relative URI is returned unchanged in that case.
//! See docs/deviations.md.

use crate::developer_error::throw_developer_error;

/// Duck-typed view of the browser `document` used by `getAbsoluteUri`
/// (`baseURI` / `location.href`).
pub trait DocumentLike {
    /// `document.baseURI`.
    fn base_uri(&self) -> Option<String>;
    /// `document.location.href`.
    fn location_href(&self) -> Option<String>;
}

/// Given a relative Uri and a base Uri, returns the absolute Uri of the
/// relative Uri.
///
/// Port of CesiumJS `getAbsoluteUri(relative, base)`; the native build has
/// no document, equivalent to `_implementation(relative, base, undefined)`.
#[must_use]
pub fn get_absolute_uri(relative: Option<&str>, base: Option<&str>) -> String {
    get_absolute_uri_implementation::<NoDocument>(relative, base, None)
}

/// Marker type: absence of a DOM document.
pub enum NoDocument {}

impl DocumentLike for NoDocument {
    fn base_uri(&self) -> Option<String> {
        match *self {}
    }
    fn location_href(&self) -> Option<String> {
        match *self {}
    }
}

/// Port of `getAbsoluteUri._implementation(relative, base, documentObject)`.
///
/// # Panics
/// Panics with `DeveloperError` when `relative` is `None`.
pub fn get_absolute_uri_implementation<D: DocumentLike>(
    relative: Option<&str>,
    base: Option<&str>,
    document_object: Option<&D>,
) -> String {
    let relative = match relative {
        Some(relative) => relative,
        None => {
            // >>includeStart('debug', pragmas.debug)
            if cfg!(debug_assertions) {
                throw_developer_error("relative uri is required.");
            }
            // >>includeEnd('debug')
            return String::new();
        }
    };

    let owned_base: Option<String> = match base {
        Some(base) => Some(base.to_owned()),
        None => match document_object {
            Some(document) => match document.base_uri().or_else(|| document.location_href()) {
                Some(b) => Some(b),
                None => return relative.to_owned(),
            },
            None => return relative.to_owned(),
        },
    };
    let base = owned_base.as_deref().expect("base resolved above");

    // If the relative URI already has a scheme it is already absolute.
    if let Ok(relative_uri) = url::Url::parse(relative) {
        if !relative_uri.scheme().is_empty() {
            return relative.to_owned();
        }
    }

    // absoluteTo(base): RFC 3986 reference resolution.
    match url::Url::parse(base) {
        Ok(base_uri) => match base_uri.join(relative) {
            Ok(resolved) => resolved.to_string(),
            Err(_) => relative.to_owned(),
        },
        // DEVIATION: urijs can merge relative-against-relative bases; the
        // native port returns the relative URI unchanged. See
        // docs/deviations.md.
        Err(_) => relative.to_owned(),
    }
}
