//! Port of `Renderer/PassStateSpec.js`.

use cesium_renderer::pass_state::PassState;

#[test]
fn creates_default_pass_state() {
    let ps = PassState::new();
    assert!(ps.blending_enabled.is_none());
    assert!(ps.scissor_test.is_none());
    assert!(ps.viewport.is_none());
}

#[test]
fn default_trait_matches_new() {
    let ps = PassState::default();
    assert!(ps.blending_enabled.is_none());
    assert!(ps.scissor_test.is_none());
    assert!(ps.viewport.is_none());
}
