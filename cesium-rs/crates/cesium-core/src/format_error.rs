//! Ported from packages/engine/Source/Core/formatError.js

use crate::developer_error::DeveloperError;
use crate::runtime_error::RuntimeError;

/// Duck-typed view of a JavaScript error object (`name`, `message`, `stack`).
///
/// DEVIATION: JS `formatError` reads dynamic properties off any value; the
/// Rust port reads them through this trait. See docs/deviations.md.
pub trait ErrorFields {
    /// The `name` property, if present.
    fn js_name(&self) -> Option<&str>;
    /// The `message` property, if present.
    fn js_message(&self) -> Option<&str>;
    /// The `stack` property, if present.
    fn js_stack(&self) -> Option<&str>;
    /// Fallback when name/message are unavailable (`object.toString()`).
    fn js_to_string(&self) -> String;
}

impl ErrorFields for DeveloperError {
    fn js_name(&self) -> Option<&str> {
        Some(self.name)
    }
    fn js_message(&self) -> Option<&str> {
        Some(&self.message)
    }
    fn js_stack(&self) -> Option<&str> {
        self.stack.as_deref()
    }
    fn js_to_string(&self) -> String {
        self.to_string()
    }
}

impl ErrorFields for RuntimeError {
    fn js_name(&self) -> Option<&str> {
        Some(self.name)
    }
    fn js_message(&self) -> Option<&str> {
        Some(&self.message)
    }
    fn js_stack(&self) -> Option<&str> {
        self.stack.as_deref()
    }
    fn js_to_string(&self) -> String {
        self.to_string()
    }
}

/// Formats an error object into a String. If available, uses name, message,
/// and stack properties, otherwise, falls back on toString().
///
/// Port of CesiumJS `formatError(object)`.
#[must_use]
pub fn format_error(object: &dyn ErrorFields) -> String {
    let mut result;

    let name = object.js_name();
    let message = object.js_message();
    if name.is_some() && message.is_some() {
        result = format!("{}: {}", name.unwrap(), message.unwrap());
    } else {
        result = object.js_to_string();
    }

    if let Some(stack) = object.js_stack() {
        result.push('\n');
        result.push_str(stack);
    }

    result
}
