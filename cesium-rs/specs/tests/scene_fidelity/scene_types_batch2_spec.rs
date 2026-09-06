//! Specs for Scene types substantiated from stubs (batch 2):
//! - `Particle`, `ParticleEmitter` trait
//! - `BoxEmitter`, `CircleEmitter`, `ConeEmitter`, `SphereEmitter`
//! - `FrustumCommands`
//! - `MetadataType`, `MetadataEnumValue`, `MetadataComponentType`

use cesium_core::cartesian3::Cartesian3;

use cesium_scene::box_emitter::BoxEmitter;
use cesium_scene::circle_emitter::CircleEmitter;
use cesium_scene::cone_emitter::ConeEmitter;
use cesium_scene::frustum_commands::FrustumCommands;
use cesium_scene::metadata_component_type::{MetadataComponentType, ScalarCategory};
use cesium_scene::metadata_enum_value::MetadataEnumValue;
use cesium_scene::metadata_type::MetadataType;
use cesium_scene::particle::Particle;
use cesium_scene::particle_emitter::ParticleEmitter;
use cesium_scene::sphere_emitter::SphereEmitter;

// ─── Particle ──────────────────────────────────────────────────────

#[test]
fn particle_defaults() {
    let p = Particle::new();
    assert_eq!(p.mass, 1.0);
    assert_eq!(p.position, Cartesian3::ZERO);
    assert_eq!(p.velocity, Cartesian3::ZERO);
    assert_eq!(p.life, f64::MAX);
    assert_eq!(p.start_scale, 1.0);
    assert_eq!(p.end_scale, 1.0);
    assert_eq!(p.age(), 0.0);
    assert_eq!(p.normalized_age(), 0.0);
}

#[test]
fn particle_update_advances_position() {
    let mut p = Particle::new();
    p.velocity = Cartesian3::new(1.0, 0.0, 0.0);
    p.life = 10.0;
    let alive = p.update(2.0);
    assert!(alive);
    assert!((p.position.x - 2.0).abs() < 1e-10);
    assert_eq!(p.age(), 2.0);
    assert!((p.normalized_age() - 0.2).abs() < 1e-10);
}

#[test]
fn particle_update_returns_false_when_dead() {
    let mut p = Particle::new();
    p.life = 1.0;
    let alive = p.update(2.0);
    assert!(!alive);
}

#[test]
fn particle_normalized_age_with_max_life() {
    let mut p = Particle::new();
    p.life = f64::MAX;
    p.update(100.0);
    // With MAX life, normalized_age stays 0
    assert_eq!(p.normalized_age(), 0.0);
}

// ─── BoxEmitter ────────────────────────────────────────────────────

#[test]
fn box_emitter_defaults() {
    let emitter = BoxEmitter::new(None);
    assert_eq!(emitter.dimensions().x, 1.0);
    assert_eq!(emitter.dimensions().y, 1.0);
    assert_eq!(emitter.dimensions().z, 1.0);
}

#[test]
fn box_emitter_custom_dimensions() {
    let emitter = BoxEmitter::new(Some(Cartesian3::new(2.0, 3.0, 4.0)));
    assert_eq!(emitter.dimensions().x, 2.0);
    assert_eq!(emitter.dimensions().y, 3.0);
    assert_eq!(emitter.dimensions().z, 4.0);
}

#[test]
fn box_emitter_emit_sets_position_within_bounds() {
    let emitter = BoxEmitter::new(Some(Cartesian3::new(10.0, 10.0, 10.0)));
    let mut p = Particle::new();
    for _ in 0..50 {
        emitter.emit(&mut p);
        assert!(p.position.x.abs() <= 5.0);
        assert!(p.position.y.abs() <= 5.0);
        assert!(p.position.z.abs() <= 5.0);
    }
}

#[test]
fn box_emitter_implements_particle_emitter_trait() {
    let emitter = BoxEmitter::new(None);
    let _trait_ref: &dyn ParticleEmitter = &emitter;
}

// ─── CircleEmitter ─────────────────────────────────────────────────

#[test]
fn circle_emitter_defaults() {
    let emitter = CircleEmitter::new(None);
    assert_eq!(emitter.radius(), 1.0);
}

#[test]
fn circle_emitter_emit_sets_z_zero() {
    let emitter = CircleEmitter::new(Some(5.0));
    let mut p = Particle::new();
    for _ in 0..50 {
        emitter.emit(&mut p);
        assert_eq!(p.position.z, 0.0);
        assert!(p.position.x.hypot(p.position.y) <= 5.0 + 1e-10);
        assert_eq!(p.velocity, Cartesian3::UNIT_Z);
    }
}

// ─── ConeEmitter ───────────────────────────────────────────────────

#[test]
fn cone_emitter_default_angle() {
    let emitter = ConeEmitter::new(None);
    let expected = std::f64::consts::PI / 6.0; // toRadians(30)
    assert!((emitter.angle() - expected).abs() < 1e-10);
}

#[test]
fn cone_emitter_emit_sets_position_at_origin() {
    let emitter = ConeEmitter::new(None);
    let mut p = Particle::new();
    emitter.emit(&mut p);
    assert_eq!(p.position, Cartesian3::ZERO);
    // Velocity should be normalized (unit length)
    let mag = Cartesian3::magnitude(&p.velocity);
    assert!((mag - 1.0).abs() < 1e-10);
}

// ─── SphereEmitter ─────────────────────────────────────────────────

#[test]
fn sphere_emitter_defaults() {
    let emitter = SphereEmitter::new(None);
    assert_eq!(emitter.radius(), 1.0);
}

#[test]
fn sphere_emitter_emit_within_radius() {
    let emitter = SphereEmitter::new(Some(3.0));
    let mut p = Particle::new();
    for _ in 0..50 {
        emitter.emit(&mut p);
        let dist = Cartesian3::magnitude(&p.position);
        assert!(dist <= 3.0 + 1e-10);
        // Velocity should be normalized
        let vel_mag = Cartesian3::magnitude(&p.velocity);
        assert!((vel_mag - 1.0).abs() < 1e-10);
    }
}

// ─── FrustumCommands ───────────────────────────────────────────────

#[test]
fn frustum_commands_defaults() {
    let fc = FrustumCommands::new(None, None);
    assert_eq!(fc.near, 0.0);
    assert_eq!(fc.far, 0.0);
    assert_eq!(fc.commands.len(), 14); // Pass::NumberOfPasses
    assert_eq!(fc.indices.len(), 14);
    for cmd_list in &fc.commands {
        assert!(cmd_list.is_empty());
    }
    for &idx in &fc.indices {
        assert_eq!(idx, 0);
    }
}

#[test]
fn frustum_commands_custom_near_far() {
    let fc = FrustumCommands::new(Some(10.0), Some(1000.0));
    assert_eq!(fc.near, 10.0);
    assert_eq!(fc.far, 1000.0);
}

// ─── MetadataType ──────────────────────────────────────────────────

#[test]
fn metadata_type_as_str() {
    assert_eq!(MetadataType::Scalar.as_str(), "SCALAR");
    assert_eq!(MetadataType::Vec3.as_str(), "VEC3");
    assert_eq!(MetadataType::Mat4.as_str(), "MAT4");
    assert_eq!(MetadataType::Boolean.as_str(), "BOOLEAN");
    assert_eq!(MetadataType::String.as_str(), "STRING");
    assert_eq!(MetadataType::Enum.as_str(), "ENUM");
}

#[test]
fn metadata_type_from_str() {
    assert_eq!(MetadataType::from_str("SCALAR"), Some(MetadataType::Scalar));
    assert_eq!(MetadataType::from_str("VEC4"), Some(MetadataType::Vec4));
    assert_eq!(MetadataType::from_str("INVALID"), None);
}

#[test]
fn metadata_type_is_vector_type() {
    assert!(MetadataType::Vec2.is_vector_type());
    assert!(MetadataType::Vec3.is_vector_type());
    assert!(MetadataType::Vec4.is_vector_type());
    assert!(!MetadataType::Scalar.is_vector_type());
    assert!(!MetadataType::Mat2.is_vector_type());
}

#[test]
fn metadata_type_is_matrix_type() {
    assert!(MetadataType::Mat2.is_matrix_type());
    assert!(MetadataType::Mat3.is_matrix_type());
    assert!(MetadataType::Mat4.is_matrix_type());
    assert!(!MetadataType::Vec3.is_matrix_type());
    assert!(!MetadataType::Scalar.is_matrix_type());
}

#[test]
fn metadata_type_component_count() {
    assert_eq!(MetadataType::Scalar.component_count(), 1);
    assert_eq!(MetadataType::Vec2.component_count(), 2);
    assert_eq!(MetadataType::Vec3.component_count(), 3);
    assert_eq!(MetadataType::Vec4.component_count(), 4);
    assert_eq!(MetadataType::Mat2.component_count(), 4);
    assert_eq!(MetadataType::Mat3.component_count(), 9);
    assert_eq!(MetadataType::Mat4.component_count(), 16);
    assert_eq!(MetadataType::Boolean.component_count(), 1);
}

// ─── MetadataEnumValue ─────────────────────────────────────────────

#[test]
fn metadata_enum_value_new() {
    let ev = MetadataEnumValue::new(42, "FOO".to_string(), Some("desc".to_string()), None, None);
    assert_eq!(ev.value(), 42);
    assert_eq!(ev.name(), "FOO");
    assert_eq!(ev.description(), Some("desc"));
    assert!(ev.extras().is_none());
    assert!(ev.extensions().is_none());
}

#[test]
fn metadata_enum_value_from_json() {
    let json = serde_json::json!({
        "value": 7,
        "name": "BAR",
        "description": "A bar enum value"
    });
    let ev = MetadataEnumValue::from_json(&json).unwrap();
    assert_eq!(ev.value(), 7);
    assert_eq!(ev.name(), "BAR");
    assert_eq!(ev.description(), Some("A bar enum value"));
}

#[test]
fn metadata_enum_value_from_json_missing_fields() {
    let json = serde_json::json!({
        "value": 1,
        "name": "X"
    });
    let ev = MetadataEnumValue::from_json(&json).unwrap();
    assert_eq!(ev.value(), 1);
    assert_eq!(ev.name(), "X");
    assert!(ev.description().is_none());
}

// ─── MetadataComponentType ─────────────────────────────────────────

#[test]
fn metadata_component_type_as_str() {
    assert_eq!(MetadataComponentType::Int8.as_str(), "INT8");
    assert_eq!(MetadataComponentType::Float64.as_str(), "FLOAT64");
}

#[test]
fn metadata_component_type_from_str() {
    assert_eq!(
        MetadataComponentType::from_str("INT32"),
        Some(MetadataComponentType::Int32)
    );
    assert_eq!(MetadataComponentType::from_str("INVALID"), None);
}

#[test]
fn metadata_component_type_size_in_bytes() {
    assert_eq!(MetadataComponentType::Int8.size_in_bytes(), 1);
    assert_eq!(MetadataComponentType::Uint8.size_in_bytes(), 1);
    assert_eq!(MetadataComponentType::Int16.size_in_bytes(), 2);
    assert_eq!(MetadataComponentType::Uint16.size_in_bytes(), 2);
    assert_eq!(MetadataComponentType::Int32.size_in_bytes(), 4);
    assert_eq!(MetadataComponentType::Uint32.size_in_bytes(), 4);
    assert_eq!(MetadataComponentType::Float32.size_in_bytes(), 4);
    assert_eq!(MetadataComponentType::Int64.size_in_bytes(), 8);
    assert_eq!(MetadataComponentType::Uint64.size_in_bytes(), 8);
    assert_eq!(MetadataComponentType::Float64.size_in_bytes(), 8);
}

#[test]
fn metadata_component_type_is_vector_compatible() {
    assert!(MetadataComponentType::Int8.is_vector_compatible());
    assert!(MetadataComponentType::Float32.is_vector_compatible());
    assert!(!MetadataComponentType::Int64.is_vector_compatible());
    assert!(!MetadataComponentType::Uint64.is_vector_compatible());
}

#[test]
fn metadata_component_type_category() {
    assert_eq!(MetadataComponentType::Int8.category(), ScalarCategory::Integer);
    assert_eq!(
        MetadataComponentType::Uint32.category(),
        ScalarCategory::UnsignedInteger
    );
    assert_eq!(MetadataComponentType::Float64.category(), ScalarCategory::Float);
}

#[test]
fn metadata_component_type_is_integer_type() {
    assert!(MetadataComponentType::Int8.is_integer_type());
    assert!(MetadataComponentType::Uint64.is_integer_type());
    assert!(!MetadataComponentType::Float32.is_integer_type());
}

#[test]
fn metadata_component_type_gpu_component_type() {
    assert_eq!(MetadataComponentType::Int64.gpu_component_type(), MetadataComponentType::Int32);
    assert_eq!(MetadataComponentType::Uint64.gpu_component_type(), MetadataComponentType::Uint32);
    assert_eq!(MetadataComponentType::Float64.gpu_component_type(), MetadataComponentType::Float32);
    assert_eq!(MetadataComponentType::Int8.gpu_component_type(), MetadataComponentType::Int8);
}

#[test]
fn metadata_component_type_normalize_unnormalize() {
    // Unsigned: 127.5 / 255 ≈ 0.502
    let n = MetadataComponentType::normalize(127.5, &MetadataComponentType::Uint8);
    assert!((n - 0.5).abs() < 0.01);

    // Signed: 63.5 / 127 ≈ 0.5
    let n = MetadataComponentType::normalize(63.5, &MetadataComponentType::Int8);
    assert!((n - 0.5).abs() < 0.01);

    // Unnormalize roundtrip
    let u = MetadataComponentType::unnormalize(0.5, &MetadataComponentType::Uint8);
    assert!((u - 128.0).abs() < 1.0);
}

#[test]
fn metadata_component_type_from_to_component_datatype() {
    use cesium_core::component_datatype::ComponentDatatype;

    assert_eq!(
        MetadataComponentType::from_component_datatype(ComponentDatatype::Byte),
        Some(MetadataComponentType::Int8)
    );
    assert_eq!(
        MetadataComponentType::from_component_datatype(ComponentDatatype::Float),
        Some(MetadataComponentType::Float32)
    );
    assert_eq!(
        MetadataComponentType::Int8.to_component_datatype(),
        Some(ComponentDatatype::Byte)
    );
    // Int64 has no WebGL equivalent
    assert_eq!(MetadataComponentType::Int64.to_component_datatype(), None);
}
