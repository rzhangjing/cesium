//! Tests for Core enum types: ArcType, ClockRange, ClockStep, CornerType,
//! ArticulationStageType, ExtrapolationType, GeocodeType, GeometryType,
//! GeometryOffsetAttribute, IndexDatatype, KeyboardEventModifier,
//! PrimitiveType, RequestState, RequestType, ScreenSpaceEventType,
//! TimeStandard, Visibility, WindingOrder, HeightmapEncoding,
//! TerrainQuantization, InterpolationType, ReferenceFrame.

use cesium_core::arc_type::ArcType;
use cesium_core::articulation_stage_type::ArticulationStageType;
use cesium_core::clock_range::ClockRange;
use cesium_core::clock_step::ClockStep;
use cesium_core::corner_type::CornerType;
use cesium_core::extrapolation_type::ExtrapolationType;
use cesium_core::geocode_type::GeocodeType;
use cesium_core::geometry_offset_attribute::GeometryOffsetAttribute;
use cesium_core::geometry_type::GeometryType;
use cesium_core::heightmap_encoding::HeightmapEncoding;
use cesium_core::index_datatype::IndexDatatype;
use cesium_core::interpolation_type::InterpolationType;
use cesium_core::keyboard_event_modifier::KeyboardEventModifier;
use cesium_core::primitive_type::PrimitiveType;
use cesium_core::reference_frame::ReferenceFrame;
use cesium_core::request_state::RequestState;
use cesium_core::request_type::RequestType;
use cesium_core::screen_space_event_type::ScreenSpaceEventType;
use cesium_core::terrain_quantization::TerrainQuantization;
use cesium_core::time_standard::TimeStandard;
use cesium_core::visibility::Visibility;
use cesium_core::webgl_constants::WebGLConstants;
use cesium_core::winding_order::WindingOrder;

// --- ArcType ---
#[test]
fn arc_type_variants() {
    assert_eq!(ArcType::None as i32, 0);
    assert_eq!(ArcType::Geodesic as i32, 1);
    assert_eq!(ArcType::Rhumb as i32, 2);
}

// --- ClockRange ---
#[test]
fn clock_range_variants() {
    assert_eq!(ClockRange::Unbounded as i32, 0);
    assert_eq!(ClockRange::Clamped as i32, 1);
    assert_eq!(ClockRange::LoopStop as i32, 2);
}

// --- ClockStep ---
#[test]
fn clock_step_variants() {
    assert_eq!(ClockStep::TickDependent as i32, 0);
    assert_eq!(ClockStep::SystemClockMultiplier as i32, 1);
    assert_eq!(ClockStep::SystemClock as i32, 2);
}

// --- CornerType ---
#[test]
fn corner_type_variants() {
    assert_eq!(CornerType::Rounded as i32, 0);
    assert_eq!(CornerType::Mitered as i32, 1);
    assert_eq!(CornerType::Beveled as i32, 2);
}

// --- ArticulationStageType ---
#[test]
fn articulation_stage_type_as_str() {
    assert_eq!(ArticulationStageType::XTranslate.as_str(), "xTranslate");
    assert_eq!(ArticulationStageType::YRotate.as_str(), "yRotate");
    assert_eq!(ArticulationStageType::UniformScale.as_str(), "uniformScale");
}

// --- ExtrapolationType ---
#[test]
fn extrapolation_type_variants() {
    assert_eq!(ExtrapolationType::None as i32, 0);
    assert_eq!(ExtrapolationType::Hold as i32, 1);
    assert_eq!(ExtrapolationType::Extrapolate as i32, 2);
}

// --- GeocodeType ---
#[test]
fn geocode_type_variants() {
    assert_eq!(GeocodeType::Search as i32, 0);
    assert_eq!(GeocodeType::Autocomplete as i32, 1);
}

// --- GeometryType ---
#[test]
fn geometry_type_variants() {
    assert_eq!(GeometryType::None as u32, 0);
    assert_eq!(GeometryType::Triangles as u32, 1);
    assert_eq!(GeometryType::Lines as u32, 2);
    assert_eq!(GeometryType::Polylines as u32, 3);
}

// --- GeometryOffsetAttribute ---
#[test]
fn geometry_offset_attribute_validate() {
    assert!(GeometryOffsetAttribute::validate(0));
    assert!(GeometryOffsetAttribute::validate(1));
    assert!(GeometryOffsetAttribute::validate(2));
    assert!(!GeometryOffsetAttribute::validate(3));
}

#[test]
fn geometry_offset_attribute_try_from_u32() {
    assert_eq!(GeometryOffsetAttribute::try_from_u32(0), Some(GeometryOffsetAttribute::None));
    assert_eq!(GeometryOffsetAttribute::try_from_u32(1), Some(GeometryOffsetAttribute::Top));
    assert_eq!(GeometryOffsetAttribute::try_from_u32(2), Some(GeometryOffsetAttribute::All));
    assert_eq!(GeometryOffsetAttribute::try_from_u32(99), None);
}

// --- IndexDatatype ---
#[test]
fn index_datatype_size_in_bytes() {
    assert_eq!(IndexDatatype::UnsignedByte.size_in_bytes(), 1);
    assert_eq!(IndexDatatype::UnsignedShort.size_in_bytes(), 2);
    assert_eq!(IndexDatatype::UnsignedInt.size_in_bytes(), 4);
}

#[test]
fn index_datatype_from_size_in_bytes() {
    assert_eq!(IndexDatatype::from_size_in_bytes(1), IndexDatatype::UnsignedByte);
    assert_eq!(IndexDatatype::from_size_in_bytes(2), IndexDatatype::UnsignedShort);
    assert_eq!(IndexDatatype::from_size_in_bytes(4), IndexDatatype::UnsignedInt);
}

#[test]
fn index_datatype_validate() {
    assert!(IndexDatatype::validate(WebGLConstants::UNSIGNED_BYTE));
    assert!(IndexDatatype::validate(WebGLConstants::UNSIGNED_SHORT));
    assert!(IndexDatatype::validate(WebGLConstants::UNSIGNED_INT));
    assert!(!IndexDatatype::validate(999));
}

#[test]
fn index_datatype_create_typed_array() {
    let small = IndexDatatype::create_typed_array(100, 5);
    assert_eq!(small.len(), 5);
    let large = IndexDatatype::create_typed_array(100000, 5);
    assert_eq!(large.len(), 5);
}

#[test]
fn index_datatype_try_from_u32() {
    assert_eq!(IndexDatatype::try_from_u32(WebGLConstants::UNSIGNED_BYTE), Some(IndexDatatype::UnsignedByte));
    assert_eq!(IndexDatatype::try_from_u32(999), None);
}

// --- KeyboardEventModifier ---
#[test]
fn keyboard_event_modifier_variants() {
    assert_eq!(KeyboardEventModifier::Shift as i32, 0);
    assert_eq!(KeyboardEventModifier::Ctrl as i32, 1);
    assert_eq!(KeyboardEventModifier::Alt as i32, 2);
}

// --- PrimitiveType ---
#[test]
fn primitive_type_is_lines() {
    assert!(PrimitiveType::Lines.is_lines());
    assert!(PrimitiveType::LineLoop.is_lines());
    assert!(PrimitiveType::LineStrip.is_lines());
    assert!(!PrimitiveType::Triangles.is_lines());
    assert!(!PrimitiveType::Points.is_lines());
}

#[test]
fn primitive_type_is_triangles() {
    assert!(PrimitiveType::Triangles.is_triangles());
    assert!(PrimitiveType::TriangleStrip.is_triangles());
    assert!(PrimitiveType::TriangleFan.is_triangles());
    assert!(!PrimitiveType::Lines.is_triangles());
}

#[test]
fn primitive_type_validate() {
    assert!(PrimitiveType::validate(WebGLConstants::POINTS));
    assert!(PrimitiveType::validate(WebGLConstants::TRIANGLES));
    assert!(!PrimitiveType::validate(99999));
}

#[test]
fn primitive_type_try_from_u32() {
    assert_eq!(PrimitiveType::try_from_u32(WebGLConstants::POINTS), Some(PrimitiveType::Points));
    assert_eq!(PrimitiveType::try_from_u32(99999), None);
}

// --- RequestState ---
#[test]
fn request_state_variants() {
    assert_eq!(RequestState::Unissued as i32, 0);
    assert_eq!(RequestState::Issued as i32, 1);
    assert_eq!(RequestState::Active as i32, 2);
    assert_eq!(RequestState::Received as i32, 3);
    assert_eq!(RequestState::Cancelled as i32, 4);
    assert_eq!(RequestState::Failed as i32, 5);
}

// --- RequestType ---
#[test]
fn request_type_variants() {
    assert_eq!(RequestType::Terrain as i32, 0);
    assert_eq!(RequestType::Imagery as i32, 1);
    assert_eq!(RequestType::Tiles3D as i32, 2);
    assert_eq!(RequestType::Other as i32, 3);
}

// --- ScreenSpaceEventType ---
#[test]
fn screen_space_event_type_variants() {
    assert_eq!(ScreenSpaceEventType::LeftDown as i32, 0);
    assert_eq!(ScreenSpaceEventType::MouseMove as i32, 15);
    assert_eq!(ScreenSpaceEventType::Wheel as i32, 16);
}

// --- TimeStandard ---
#[test]
fn time_standard_variants() {
    assert_eq!(TimeStandard::UTC as u8, 0);
    assert_eq!(TimeStandard::TAI as u8, 1);
}

// --- Visibility ---
#[test]
fn visibility_variants() {
    assert_eq!(Visibility::None as i32, -1);
    assert_eq!(Visibility::Partial as i32, 0);
    assert_eq!(Visibility::Full as i32, 1);
}

// --- WindingOrder ---
#[test]
fn winding_order_validate() {
    assert!(WindingOrder::validate(WebGLConstants::CW));
    assert!(WindingOrder::validate(WebGLConstants::CCW));
    assert!(!WindingOrder::validate(999));
}

#[test]
fn winding_order_try_from_u32() {
    assert_eq!(WindingOrder::try_from_u32(WebGLConstants::CW), Some(WindingOrder::Clockwise));
    assert_eq!(WindingOrder::try_from_u32(WebGLConstants::CCW), Some(WindingOrder::CounterClockwise));
    assert_eq!(WindingOrder::try_from_u32(0), None);
}

// --- HeightmapEncoding ---
#[test]
fn heightmap_encoding_variants() {
    assert_eq!(HeightmapEncoding::None as i32, 0);
    assert_eq!(HeightmapEncoding::Lerc as i32, 1);
}

// --- TerrainQuantization ---
#[test]
fn terrain_quantization_variants() {
    assert_eq!(TerrainQuantization::None as i32, 0);
    assert_eq!(TerrainQuantization::Bits12 as i32, 1);
}

// --- InterpolationType ---
#[test]
fn interpolation_type_variants() {
    assert_eq!(InterpolationType::Step as i32, 0);
    assert_eq!(InterpolationType::Linear as i32, 1);
    assert_eq!(InterpolationType::CubicSpline as i32, 2);
}

// --- ReferenceFrame ---
#[test]
fn reference_frame_variants() {
    assert_eq!(ReferenceFrame::Fixed as i32, 0);
    assert_eq!(ReferenceFrame::Inertial as i32, 1);
}
