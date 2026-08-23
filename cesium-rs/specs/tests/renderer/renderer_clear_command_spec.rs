//! Port of `Renderer/ClearCommandSpec.js`.

use cesium_renderer::clear_command::ClearCommand;

#[test]
fn constructs_with_defaults() {
    let c = ClearCommand::default();
    assert!(c.color.is_none());
    assert!(c.depth.is_none());
    assert!(c.stencil.is_none());
    assert!(c.framebuffer.is_none());
}

#[test]
fn constructs_with_options() {
    let c = ClearCommand {
        color: Some([1.0, 2.0, 3.0, 4.0]),
        depth: Some(1.0),
        stencil: Some(2),
        ..ClearCommand::default()
    };
    assert_eq!(c.color, Some([1.0, 2.0, 3.0, 4.0]));
    assert_eq!(c.depth, Some(1.0));
    assert_eq!(c.stencil, Some(2));
}

#[test]
fn clear_all_has_expected_defaults() {
    let all = ClearCommand::all();
    assert_eq!(all.color, Some([0.0, 0.0, 0.0, 0.0]));
    assert_eq!(all.depth, Some(1.0));
    assert_eq!(all.stencil, Some(0));
}
