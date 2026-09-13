//! Widgets/TimelineSpec.js → Rust integration tests

use cesium_widgets::{Timeline, TimelineTrack, TimelineHighlightRange, TimelineTicScale};

// === Timeline ===

#[test]
fn test_timeline_default() {
    let tl = Timeline::default();
    assert_eq!(tl.start_time, 0.0);
    assert_eq!(tl.end_time, 86400.0);
    assert!(tl.tracks.is_empty());
    assert!(tl.show);
}

#[test]
fn test_timeline_add_track() {
    let mut tl = Timeline::default();
    tl.tracks.push(TimelineTrack::new("test", 0.0, 3600.0));
    assert_eq!(tl.tracks.len(), 1);
    assert_eq!(tl.tracks[0].name, "test");
}

#[test]
fn test_timeline_add_highlight() {
    let mut tl = Timeline::default();
    tl.highlight_ranges.push(TimelineHighlightRange::new(100.0, 200.0));
    assert_eq!(tl.highlight_ranges.len(), 1);
}

// === TimelineTrack ===

#[test]
fn test_timeline_track_new() {
    let track = TimelineTrack::new("availability", 0.0, 7200.0);
    assert_eq!(track.name, "availability");
    assert_eq!(track.start_time, 0.0);
    assert_eq!(track.end_time, 7200.0);
    assert_eq!(track.color, [0.5, 0.5, 1.0, 1.0]);
}

// === TimelineHighlightRange ===

#[test]
fn test_timeline_highlight_range_new() {
    let range = TimelineHighlightRange::new(100.0, 500.0);
    assert_eq!(range.start_time, 100.0);
    assert_eq!(range.end_time, 500.0);
    assert_eq!(range.color, [1.0, 1.0, 0.0, 0.3]);
}

#[test]
fn test_timeline_highlight_range_with_color() {
    let range = TimelineHighlightRange::new(0.0, 100.0).with_color([1.0, 0.0, 0.0, 0.5]);
    assert_eq!(range.color, [1.0, 0.0, 0.0, 0.5]);
}

// === TimelineTicScale ===

#[test]
fn test_timeline_tic_scale() {
    let scale = TimelineTicScale::for_span_and_width(86400.0, 1000.0, 50.0);
    assert!(scale.seconds > 0.0);
}
