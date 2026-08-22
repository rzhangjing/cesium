//! Ported from packages/engine/Source/Core/oneTimeWarning.js

use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};

use crate::developer_error::throw_developer_error;

static WARNINGS: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

type WarningSink = Box<dyn Fn(&str) + Send>;

/// Sink used to emit warnings (`console.warn` in CesiumJS, `eprintln!` in
/// the native port). Replaceable for spec verification.
static WARN_SINK: LazyLock<Mutex<WarningSink>> = LazyLock::new(|| {
    Mutex::new(Box::new(|message: &str| eprintln!("{message}")) as WarningSink)
});

/// Replaces the warning sink (spec helper; the CesiumJS specs spy on
/// `console.warn` instead).
pub fn set_warning_sink_for_specs(sink: WarningSink) {
    *WARN_SINK.lock().expect("warning sink poisoned") = sink;
}

/// Restores the default warning sink (spec helper).
pub fn reset_warning_sink_for_specs() {
    *WARN_SINK.lock().expect("warning sink poisoned") =
        Box::new(|message: &str| eprintln!("{message}")) as WarningSink;
}

/// Logs a one time message to the console. Use this function instead of
/// logging directly since this does not log duplicate messages unless it is
/// called from multiple workers.
///
/// Port of CesiumJS `oneTimeWarning(identifier, message)`; `message`
/// defaults to `identifier` (JS `message ?? identifier`).
///
/// # Panics
/// In debug builds, panics with `DeveloperError` when `identifier` is
/// `None`.
pub fn one_time_warning(identifier: Option<&str>, message: Option<&str>) {
    // >>includeStart('debug', pragmas.debug)
    if cfg!(debug_assertions) && identifier.is_none() {
        throw_developer_error("identifier is required.");
    }
    // >>includeEnd('debug')
    let Some(identifier) = identifier else {
        return;
    };

    let mut warnings = WARNINGS.lock().expect("warnings registry poisoned");
    if !warnings.contains(identifier) {
        warnings.insert(identifier.to_owned());
        drop(warnings);
        let sink = WARN_SINK.lock().expect("warning sink poisoned");
        sink(message.unwrap_or(identifier));
    }
}

/// Entity geometry outlines are unsupported on terrain. Outlines will be
/// disabled. To enable outlines, disable geometry terrain clamping by
/// explicitly setting height to 0.
pub const GEOMETRY_OUTLINES: &str =
    "Entity geometry outlines are unsupported on terrain. Outlines will be disabled. To enable outlines, disable geometry terrain clamping by explicitly setting height to 0.";

/// Entity geometry with zIndex are unsupported when height or extrudedHeight
/// are defined. zIndex will be ignored.
pub const GEOMETRY_Z_INDEX: &str =
    "Entity geometry with zIndex are unsupported when height or extrudedHeight are defined.  zIndex will be ignored";

/// Entity corridor, ellipse, polygon or rectangle with heightReference must
/// also have a defined height. heightReference will be ignored.
pub const GEOMETRY_HEIGHT_REFERENCE: &str =
    "Entity corridor, ellipse, polygon or rectangle with heightReference must also have a defined height.  heightReference will be ignored";

/// Entity corridor, ellipse, polygon or rectangle with
/// extrudedHeightReference must also have a defined extrudedHeight.
/// extrudedHeightReference will be ignored.
pub const GEOMETRY_EXTRUDED_HEIGHT_REFERENCE: &str =
    "Entity corridor, ellipse, polygon or rectangle with extrudedHeightReference must also have a defined extrudedHeight.  extrudedHeightReference will be ignored";
