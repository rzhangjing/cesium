//! Ported from `packages/widgets/Source/InspectorShared.js`.

/// Shared utilities for inspector widgets.

/// Creates an HTML section element.
pub fn create_section(title: &str) -> String {
    format!("<div class=\"cesium-inspector-section\"><h3>{}</h3></div>", title)
}

/// Creates an HTML checkbox element.
pub fn create_checkbox(label: &str, checked: bool) -> String {
    let checked_attr = if checked { " checked" } else { "" };
    format!("<label><input type=\"checkbox\"{}> {}</label>", checked_attr, label)
}
