//! Ported from `packages/engine/Source/Scene/GroundPrimitive.js`.
//!
//! A ground primitive draped onto terrain.
//!
//! M3/S3 status — DEVIATION: CesiumJS `GroundPrimitive` drapes geometry
//! instances onto terrain/3D Tiles through the shadow-volume classification
//! pipeline (`BatchTable`-aware `ClassificationType`, terrain extent
//! clipping, per-tile volume extrusion via the `createGeometry` worker
//! path). The terrain-draping dependency chain (classification volumes +
//! per-tile re-batching against the quadtree) is not yet ported, so
//! `update` registers no draw commands and the primitive never becomes
//! ready; the collection/API semantics (options, classification type,
//! shadow mode, destroy) are kept faithful so dependents can construct and
//! manage ground primitives. See `docs/MAPPING.md` (GroundPrimitive row)
//! and the ignored GPU test below for the unlock conditions.

use cesium_core::geometry_instance::GeometryInstance;
use cesium_renderer::context::Context;

use crate::classification_type::ClassificationType;
use crate::frame_state::FrameState;
use crate::primitive_collection::ScenePrimitive;
use crate::shadow_mode::ShadowMode;

/// A ground primitive that drapes geometry onto terrain or 3D Tiles.
///
/// Mirrors CesiumJS `GroundPrimitive` (1047 lines).
pub struct GroundPrimitive {
    /// Whether this primitive is shown.
    pub show: bool,
    /// Whether to allow picking.
    pub allow_picking: bool,
    /// Whether to compress geometry.
    pub compress_geometry: bool,
    /// The shadow mode.
    pub shadows: ShadowMode,
    /// What this primitive classifies (terrain, 3D Tiles, or both).
    pub classification_type: ClassificationType,
    /// The geometry instances to drape (held until the draping pipeline is
    /// available; mirrors the JS constructor input).
    geometry_instances: Vec<GeometryInstance>,
    /// Whether this primitive is ready.
    ready: bool,
    /// Whether this primitive has been destroyed.
    is_destroyed: bool,
}

/// The constructor options of [`GroundPrimitive`], mirroring the JS
/// `options` object.
#[derive(Default)]
pub struct GroundPrimitiveOptions {
    pub geometry_instances: Vec<GeometryInstance>,
    pub show: Option<bool>,
    pub allow_picking: Option<bool>,
    pub compress_geometry: Option<bool>,
    pub shadows: Option<ShadowMode>,
    pub classification_type: Option<ClassificationType>,
}

impl GroundPrimitive {
    /// Creates a new GroundPrimitive.
    pub fn new() -> Self {
        Self::with_options(GroundPrimitiveOptions::default())
    }

    /// Creates a new GroundPrimitive from explicit options (mirrors the JS
    /// `options` object of the constructor).
    pub fn with_options(options: GroundPrimitiveOptions) -> Self {
        Self {
            show: options.show.unwrap_or(true),
            allow_picking: options.allow_picking.unwrap_or(true),
            compress_geometry: options.compress_geometry.unwrap_or(true),
            shadows: options.shadows.unwrap_or(ShadowMode::Disabled),
            classification_type: options
                .classification_type
                .unwrap_or(ClassificationType::Both),
            geometry_instances: options.geometry_instances,
            ready: false,
            is_destroyed: false,
        }
    }

    /// Adds a geometry instance (mirrors the JS `addInstance`).
    pub fn add_instance(&mut self, instance: GeometryInstance) {
        self.geometry_instances.push(instance);
        self.ready = false;
    }

    /// Returns the number of geometry instances (mirrors the JS
    /// `getGeometryInstanceCount`).
    pub fn instance_count(&self) -> usize {
        self.geometry_instances.len()
    }

    /// Updates the primitive for the current frame.
    ///
    /// DEVIATION (module docs): the terrain draping pipeline is not yet
    /// ported, so no draw commands are generated.
    pub fn update(&mut self, _frame_state: &FrameState, _context: &mut Context) {
        // Intentionally empty — see the module-level DEVIATION.
    }

    /// Returns whether this primitive is ready.
    pub fn is_ready(&self) -> bool { self.ready }

    /// Returns whether this primitive has been destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys this primitive.
    pub fn destroy(&mut self) {
        self.geometry_instances.clear();
        self.is_destroyed = true;
    }
}

impl Default for GroundPrimitive {
    fn default() -> Self { Self::new() }
}

impl ScenePrimitive for GroundPrimitive {
    fn update(&mut self, frame_state: &FrameState, context: &mut Context) {
        GroundPrimitive::update(self, frame_state, context);
    }
    fn show(&self) -> bool { self.show }
    fn set_show(&mut self, show: bool) { self.show = show; }
    fn is_destroyed(&self) -> bool { GroundPrimitive::is_destroyed(self) }
    fn destroy(&mut self) { GroundPrimitive::destroy(self); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_core::geometry_instance::GeometryInstanceGeometry;

    /// Mirrors GroundPrimitiveSpec constructor defaults (the non-WebGL
    /// subset): show/picking/compress/shadows/classification defaults.
    #[test]
    fn constructor_defaults() {
        let primitive = GroundPrimitive::new();
        assert!(primitive.show);
        assert!(primitive.allow_picking);
        assert!(primitive.compress_geometry);
        assert!(matches!(primitive.shadows, ShadowMode::Disabled));
        assert!(matches!(primitive.classification_type, ClassificationType::Both));
        assert!(!primitive.is_ready());
        assert!(!primitive.is_destroyed());
    }

    /// Mirrors GroundPrimitiveSpec: options override the defaults and the
    /// instances are held for the draping pipeline.
    #[test]
    fn options_and_instances() {
        let mut primitive = GroundPrimitive::with_options(GroundPrimitiveOptions {
            show: Some(false),
            classification_type: Some(ClassificationType::Terrain),
            ..Default::default()
        });
        assert!(!primitive.show);
        assert!(matches!(primitive.classification_type, ClassificationType::Terrain));
        primitive.add_instance(GeometryInstance::new(
            GeometryInstanceGeometry::Placeholder,
            None,
            None,
            None,
        ));
        assert_eq!(primitive.instance_count(), 1);
    }

    /// Mirrors GroundPrimitiveSpec: "destroys".
    #[test]
    fn destroys() {
        let mut primitive = GroundPrimitive::new();
        primitive.destroy();
        assert!(primitive.is_destroyed());
    }

    /// Drape render path (terrain classification volumes).
    ///
    /// UNLOCK CONDITION: requires the shadow-volume classification pipeline
    /// plus terrain height queries (M4 scope); then this becomes a GPU
    /// smoke test asserting the primitive becomes ready and classifies the
    /// globe surface.
    #[ignore = "terrain draping/classification pipeline not yet ported (M4)"]
    #[test]
    fn drapes_geometry_onto_terrain() {
        // Placeholder — see the module-level DEVIATION.
    }
}
