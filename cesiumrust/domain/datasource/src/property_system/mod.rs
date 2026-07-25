//! The complete CesiumJS-compatible Property system.
//!
//! Maps to CesiumJS `DataSources/Property.js` and its ~25 concrete
//! implementations (ConstantProperty, SampledProperty,
//! TimeIntervalCollectionProperty, CompositeProperty, CallbackProperty,
//! ReferenceProperty, PositionProperty family, MaterialProperty family, ...).
//!
//! Unlike the legacy `property` module (a simple enum kept for backward
//! compatibility with the GeoJSON/CZML parsers), this module implements the
//! full trait-object based, time-dynamic property system with type-erased
//! values, packed-array interpolation and reference-frame aware positions.

pub mod interpolation;
pub mod material;
pub mod position;
pub mod property;
pub mod reference;
pub mod value;

pub use interpolation::{
    ExtrapolationType, HermitePolynomialApproximation, InterpolationAlgorithm,
    InterpolationAlgorithmKind, LagrangePolynomialApproximation, LinearApproximation,
};
pub use material::{
    arc_material_property_equals, CheckerboardMaterialProperty, ColorMaterialProperty,
    CompositeMaterialProperty, GridMaterialProperty, ImageMaterialProperty, MaterialProperty,
    MaterialUniforms, PolylineArrowMaterialProperty, PolylineDashMaterialProperty,
    PolylineGlowMaterialProperty, PolylineOutlineMaterialProperty, StripeMaterialProperty,
    StripeOrientation, COLOR_BLACK, COLOR_TRANSPARENT, COLOR_WHITE,
};
pub use position::{
    convert_to_reference_frame, CallbackPositionProperty, CompositePositionProperty,
    ConstantPositionProperty, PositionCallbackFn, SampledPositionProperty,
    TimeIntervalCollectionPositionProperty,
};
pub use property::{
    arc_property_equals, property_get_value_or_undefined, property_is_constant, CallbackFn,
    CallbackProperty, CompositeProperty, ConstantProperty, DynProperty, SampledProperty,
    TimeIntervalCollectionProperty,
};
pub use reference::{MapPropertyResolver, PropertyResolver, ReferenceProperty};
pub use value::{PackableType, PropertyValue, ReferenceFrame};
