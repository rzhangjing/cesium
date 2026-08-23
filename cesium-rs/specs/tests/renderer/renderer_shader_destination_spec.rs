//! Port of `Renderer/ShaderDestinationSpec.js`.

use cesium_renderer::shader_destination::ShaderDestination;

#[test]
fn includes_vertex_shader_works() {
    assert!(ShaderDestination::Vertex.includes_vertex_shader());
    assert!(!ShaderDestination::Fragment.includes_vertex_shader());
    assert!(ShaderDestination::Both.includes_vertex_shader());
    assert!(!ShaderDestination::None.includes_vertex_shader());
}

#[test]
fn includes_fragment_shader_works() {
    assert!(!ShaderDestination::Vertex.includes_fragment_shader());
    assert!(ShaderDestination::Fragment.includes_fragment_shader());
    assert!(ShaderDestination::Both.includes_fragment_shader());
    assert!(!ShaderDestination::None.includes_fragment_shader());
}

#[test]
fn enum_values_match_cesiumjs() {
    // CesiumJS: ShaderDestination.NONE = 0, VERTEX = 1, FRAGMENT = 2, BOTH = 3
    assert_eq!(ShaderDestination::None as u8, 0);
    assert_eq!(ShaderDestination::Vertex as u8, 1);
    assert_eq!(ShaderDestination::Fragment as u8, 2);
    assert_eq!(ShaderDestination::Both as u8, 3);
}
