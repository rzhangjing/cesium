//! Port of `Renderer/DrawCommandSpec.js`.

use cesium_renderer::draw_command::{DrawCommand, DrawCommandFlags};

#[test]
fn constructs_with_defaults() {
    let c = DrawCommand::new();
    assert!(c.bounding_volume.is_none());
    assert!(c.oriented_bounding_box.is_none());
    assert!(c.cull());
    assert!(c.occlude());
    assert!(c.model_matrix.is_none());
    assert_eq!(c.primitive_type, 4); // TRIANGLES
    assert!(c.vertex_array.is_none());
    assert!(c.count.is_none());
    assert_eq!(c.offset, 0);
    assert_eq!(c.instance_count, 0);
    assert!(c.shader_program.is_none());
    assert!(c.uniform_map.is_none());
    assert!(c.framebuffer.is_none());
    assert!(c.pass.is_none());
    assert!(!c.execute_in_closest_frustum());
    assert!(c.owner.is_none());
    assert!(!c.debug_show_bounding_volume());
    assert!(!c.cast_shadows());
    assert!(!c.receive_shadows());
    assert!(c.pick_id.is_none());
    assert!(!c.pick_only());
}

#[test]
fn flag_setters_work() {
    let mut c = DrawCommand::new();

    c.set_cull(false);
    assert!(!c.cull());

    c.set_occlude(false);
    assert!(!c.occlude());

    c.set_cast_shadows(true);
    assert!(c.cast_shadows());

    c.set_receive_shadows(true);
    assert!(c.receive_shadows());

    c.set_pick_only(true);
    assert!(c.pick_only());

    c.set_execute_in_closest_frustum(true);
    assert!(c.execute_in_closest_frustum());

    c.set_debug_show_bounding_volume(true);
    assert!(c.debug_show_bounding_volume());
}

#[test]
fn shallow_clone_preserves_fields() {
    let mut c = DrawCommand::new();
    c.primitive_type = 3; // TRIANGLE_FAN
    c.count = Some(3);
    c.offset = 3;
    c.instance_count = 2;
    c.set_cull(false);
    c.set_occlude(false);
    c.set_cast_shadows(true);
    c.set_receive_shadows(true);
    c.set_pick_only(true);

    let clone = c.shallow_clone();
    assert_eq!(clone.primitive_type, c.primitive_type);
    assert_eq!(clone.count, c.count);
    assert_eq!(clone.offset, c.offset);
    assert_eq!(clone.instance_count, c.instance_count);
    assert_eq!(clone.cull(), c.cull());
    assert_eq!(clone.occlude(), c.occlude());
    assert_eq!(clone.cast_shadows(), c.cast_shadows());
    assert_eq!(clone.receive_shadows(), c.receive_shadows());
    assert_eq!(clone.pick_only(), c.pick_only());
}

#[test]
fn flags_default_is_zero() {
    let f = DrawCommandFlags::new();
    assert!(!f.has(DrawCommandFlags::CULL));
    assert!(!f.has(DrawCommandFlags::OCCLUDE));
    assert!(!f.has(DrawCommandFlags::PICK_ONLY));
}

#[test]
fn flags_set_and_clear() {
    let mut f = DrawCommandFlags::new();
    f.set(DrawCommandFlags::CULL, true);
    assert!(f.has(DrawCommandFlags::CULL));
    f.set(DrawCommandFlags::CULL, false);
    assert!(!f.has(DrawCommandFlags::CULL));
}
