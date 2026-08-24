//! Ported from the CZML geometry updaters of
//! `packages/engine/Source/DataSources/CzmlDataSource.js`: `processBox`,
//! `processCorridor`, `processCylinder`, `processEllipse`,
//! `processEllipsoid`, `processModel` (+ `processNodeTransformations` /
//! `processArticulations`), `processPath`, `processPolylineVolume`,
//! `processRectangle`, `processTileset`, `processWall`, plus the
//! `processPolygon` hierarchy supplement (`_positions`/`_holes`, mirror of
//! `PolygonHierarchyProperty`) and the `processPolyline` legacy
//! `followSurface` → `arcType` adapter.
//!
//! DEVIATION (storage): CesiumJS stores these values on the `*Graphics`
//! objects hanging off the entity. The Rust `Entity` only carries the
//! constant value model (handled by [`crate::czml_data_source`]), so the
//! full time-dynamic values live in the sidecar [`CzmlGeometryStore`] owned
//! by the data source. Constant packets are still mirrored onto the entity
//! by the existing updaters.

use std::collections::{BTreeMap, HashMap};

use cesium_core::julian_date::JulianDate;
use cesium_core::time_interval::TimeInterval;
use serde_json::Value;

use crate::czml_processing::{
    compute_combined_interval, process_array, process_material_packet_data, process_packet_data,
    process_position_array, process_position_array_of_arrays, process_shape, CzmlMaterialProperty,
};
use crate::czml_property::{interval_from_string, CzmlProperty, CzmlPropertyType, CzmlValue};

// ============================================================================
// Storage
// ============================================================================

/// The time-dynamic fields of one CZML geometry packet family (a `*Graphics`
/// object in CesiumJS).
#[derive(Debug, Default)]
pub struct CzmlGeometry {
    /// Regular properties keyed by field name (`show`, `outlineWidth`,
    /// `positions`, `minimumHeights`, ...).
    pub properties: BTreeMap<String, Option<CzmlProperty>>,
    /// Material properties keyed by field name (`material`,
    /// `depthFailMaterial`).
    pub materials: BTreeMap<String, Option<CzmlMaterialProperty>>,
    /// Model `nodeTransformations`: node name -> field name -> property
    /// (mirror of the `PropertyBag` of `NodeTransformationProperty`).
    pub node_transformations: BTreeMap<String, BTreeMap<String, Option<CzmlProperty>>>,
    /// Model `articulations`: articulation stage key -> number property
    /// (mirror of the `PropertyBag` of articulation values).
    pub articulations: BTreeMap<String, Option<CzmlProperty>>,
    /// Mirrors the presence of a `PolygonHierarchyProperty` on the polygon
    /// (set when `_positions` or `_holes` carry data).
    pub has_hierarchy: bool,
}

impl CzmlGeometry {
    fn property_slot(&mut self, name: &str) -> &mut Option<CzmlProperty> {
        self.properties.entry(name.to_string()).or_insert(None)
    }

    fn material_slot(&mut self, name: &str) -> &mut Option<CzmlMaterialProperty> {
        self.materials.entry(name.to_string()).or_insert(None)
    }

    /// Evaluates the named property at `time` (mirror of
    /// `graphics[name].getValue(time)`).
    pub fn get_value(&self, name: &str, time: &JulianDate) -> Option<CzmlValue> {
        self.properties.get(name)?.as_ref()?.get_value(time)
    }

    /// Returns the material in effect at `time` for the named material slot.
    pub fn get_material(
        &self,
        name: &str,
        time: &JulianDate,
    ) -> Option<&crate::czml_processing::CzmlMaterial> {
        self.materials.get(name)?.as_ref()?.get_material(time)
    }
}

/// All geometry slots of one entity.
#[derive(Debug, Default)]
pub struct CzmlEntityGeometry {
    pub r#box: CzmlGeometry,
    pub corridor: CzmlGeometry,
    pub cylinder: CzmlGeometry,
    pub ellipse: CzmlGeometry,
    pub ellipsoid: CzmlGeometry,
    pub model: CzmlGeometry,
    pub path: CzmlGeometry,
    pub polyline_volume: CzmlGeometry,
    pub rectangle: CzmlGeometry,
    pub tileset: CzmlGeometry,
    pub wall: CzmlGeometry,
    /// Polygon supplement (`_positions`/`_holes`/hierarchy); the constant
    /// polygon fields are mirrored onto the entity by the existing updater.
    pub polygon: CzmlGeometry,
    /// Polyline supplement (legacy `followSurface` → `arcType` adapter); the
    /// constant polyline fields are mirrored onto the entity by the existing
    /// updater.
    pub polyline: CzmlGeometry,
}

/// The sidecar store of CZML geometry data, keyed by entity id.
#[derive(Debug, Default)]
pub struct CzmlGeometryStore {
    entities: HashMap<String, CzmlEntityGeometry>,
}

impl CzmlGeometryStore {
    /// Returns the geometry slots of `id`, creating them on first access
    /// (mirror of `entity.box = box = new BoxGraphics()` and friends).
    pub fn get_or_create(&mut self, id: &str) -> &mut CzmlEntityGeometry {
        self.entities.entry(id.to_string()).or_default()
    }

    /// Returns the geometry slots of `id`, if present.
    pub fn get(&self, id: &str) -> Option<&CzmlEntityGeometry> {
        self.entities.get(id)
    }

    /// Removes the geometry slots of `id` (the `delete` packet path).
    pub fn remove(&mut self, id: &str) -> Option<CzmlEntityGeometry> {
        self.entities.remove(id)
    }

    /// Removes all stored geometry data.
    pub fn clear(&mut self) {
        self.entities.clear();
    }
}

// ============================================================================
// Field processing helpers
// ============================================================================

macro_rules! field {
    ($geometry:expr, $data:expr, $name:expr, $type:expr, $interval:expr, $source_uri:expr, $current_id:expr) => {
        if let Some(packet_data) = $data.get($name) {
            let slot = $geometry.property_slot($name);
            process_packet_data(
                slot,
                $type,
                Some(packet_data),
                $interval,
                $source_uri,
                $current_id,
            );
        }
    };
}

macro_rules! material_field {
    ($geometry:expr, $data:expr, $name:expr, $interval:expr, $source_uri:expr, $current_id:expr) => {
        if let Some(packet_data) = $data.get($name) {
            let slot = $geometry.material_slot($name);
            process_material_packet_data(
                slot,
                Some(packet_data),
                $interval,
                $source_uri,
                $current_id,
            );
        }
    };
}

fn interval_of(data: &Value) -> Option<TimeInterval> {
    interval_from_string(data.get("interval").and_then(|v| v.as_str()))
}

// ============================================================================
// processBox
// ============================================================================

/// Mirror of `processBox`.
pub fn process_box(
    geometry: &mut CzmlGeometry,
    packet: &Value,
    source_uri: Option<&str>,
    current_id: Option<&str>,
) {
    let Some(box_data) = packet.get("box") else {
        return;
    };
    let interval_storage = interval_of(box_data);
    let interval = interval_storage.as_ref();

    field!(geometry, box_data, "show", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    field!(geometry, box_data, "dimensions", CzmlPropertyType::Cartesian3, interval, source_uri, current_id);
    field!(geometry, box_data, "heightReference", CzmlPropertyType::HeightReference, interval, source_uri, current_id);
    field!(geometry, box_data, "fill", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    material_field!(geometry, box_data, "material", interval, source_uri, current_id);
    field!(geometry, box_data, "outline", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    field!(geometry, box_data, "outlineColor", CzmlPropertyType::Color, interval, source_uri, current_id);
    field!(geometry, box_data, "outlineWidth", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, box_data, "shadows", CzmlPropertyType::ShadowMode, interval, source_uri, current_id);
    field!(geometry, box_data, "distanceDisplayCondition", CzmlPropertyType::DistanceDisplayCondition, interval, source_uri, current_id);
}

// ============================================================================
// processCorridor
// ============================================================================

/// Mirror of `processCorridor`.
pub fn process_corridor(
    geometry: &mut CzmlGeometry,
    packet: &Value,
    source_uri: Option<&str>,
    current_id: Option<&str>,
) {
    let Some(corridor_data) = packet.get("corridor") else {
        return;
    };
    let interval_storage = interval_of(corridor_data);
    let interval = interval_storage.as_ref();

    field!(geometry, corridor_data, "show", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    if let Some(positions) = corridor_data.get("positions") {
        let slot = geometry.property_slot("positions");
        process_position_array(slot, Some(positions), current_id);
    }
    field!(geometry, corridor_data, "width", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, corridor_data, "height", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, corridor_data, "heightReference", CzmlPropertyType::HeightReference, interval, source_uri, current_id);
    field!(geometry, corridor_data, "extrudedHeight", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, corridor_data, "extrudedHeightReference", CzmlPropertyType::HeightReference, interval, source_uri, current_id);
    field!(geometry, corridor_data, "cornerType", CzmlPropertyType::CornerType, interval, source_uri, current_id);
    field!(geometry, corridor_data, "granularity", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, corridor_data, "fill", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    material_field!(geometry, corridor_data, "material", interval, source_uri, current_id);
    field!(geometry, corridor_data, "outline", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    field!(geometry, corridor_data, "outlineColor", CzmlPropertyType::Color, interval, source_uri, current_id);
    field!(geometry, corridor_data, "outlineWidth", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, corridor_data, "shadows", CzmlPropertyType::ShadowMode, interval, source_uri, current_id);
    field!(geometry, corridor_data, "distanceDisplayCondition", CzmlPropertyType::DistanceDisplayCondition, interval, source_uri, current_id);
    field!(geometry, corridor_data, "classificationType", CzmlPropertyType::ClassificationType, interval, source_uri, current_id);
    field!(geometry, corridor_data, "zIndex", CzmlPropertyType::Number, interval, source_uri, current_id);
}

// ============================================================================
// processCylinder
// ============================================================================

/// Mirror of `processCylinder`.
pub fn process_cylinder(
    geometry: &mut CzmlGeometry,
    packet: &Value,
    source_uri: Option<&str>,
    current_id: Option<&str>,
) {
    let Some(cylinder_data) = packet.get("cylinder") else {
        return;
    };
    let interval_storage = interval_of(cylinder_data);
    let interval = interval_storage.as_ref();

    field!(geometry, cylinder_data, "show", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    field!(geometry, cylinder_data, "length", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, cylinder_data, "topRadius", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, cylinder_data, "bottomRadius", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, cylinder_data, "heightReference", CzmlPropertyType::HeightReference, interval, source_uri, current_id);
    field!(geometry, cylinder_data, "fill", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    material_field!(geometry, cylinder_data, "material", interval, source_uri, current_id);
    field!(geometry, cylinder_data, "outline", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    field!(geometry, cylinder_data, "outlineColor", CzmlPropertyType::Color, interval, source_uri, current_id);
    field!(geometry, cylinder_data, "outlineWidth", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, cylinder_data, "numberOfVerticalLines", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, cylinder_data, "slices", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, cylinder_data, "shadows", CzmlPropertyType::ShadowMode, interval, source_uri, current_id);
    field!(geometry, cylinder_data, "distanceDisplayCondition", CzmlPropertyType::DistanceDisplayCondition, interval, source_uri, current_id);
}

// ============================================================================
// processEllipse
// ============================================================================

/// Mirror of `processEllipse`.
pub fn process_ellipse(
    geometry: &mut CzmlGeometry,
    packet: &Value,
    source_uri: Option<&str>,
    current_id: Option<&str>,
) {
    let Some(ellipse_data) = packet.get("ellipse") else {
        return;
    };
    let interval_storage = interval_of(ellipse_data);
    let interval = interval_storage.as_ref();

    field!(geometry, ellipse_data, "show", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    field!(geometry, ellipse_data, "semiMajorAxis", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, ellipse_data, "semiMinorAxis", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, ellipse_data, "height", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, ellipse_data, "heightReference", CzmlPropertyType::HeightReference, interval, source_uri, current_id);
    field!(geometry, ellipse_data, "extrudedHeight", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, ellipse_data, "extrudedHeightReference", CzmlPropertyType::HeightReference, interval, source_uri, current_id);
    field!(geometry, ellipse_data, "rotation", CzmlPropertyType::Rotation, interval, source_uri, current_id);
    field!(geometry, ellipse_data, "stRotation", CzmlPropertyType::Rotation, interval, source_uri, current_id);
    field!(geometry, ellipse_data, "granularity", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, ellipse_data, "fill", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    material_field!(geometry, ellipse_data, "material", interval, source_uri, current_id);
    field!(geometry, ellipse_data, "outline", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    field!(geometry, ellipse_data, "outlineColor", CzmlPropertyType::Color, interval, source_uri, current_id);
    field!(geometry, ellipse_data, "outlineWidth", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, ellipse_data, "numberOfVerticalLines", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, ellipse_data, "shadows", CzmlPropertyType::ShadowMode, interval, source_uri, current_id);
    field!(geometry, ellipse_data, "distanceDisplayCondition", CzmlPropertyType::DistanceDisplayCondition, interval, source_uri, current_id);
    field!(geometry, ellipse_data, "classificationType", CzmlPropertyType::ClassificationType, interval, source_uri, current_id);
    field!(geometry, ellipse_data, "zIndex", CzmlPropertyType::Number, interval, source_uri, current_id);
}

// ============================================================================
// processEllipsoid
// ============================================================================

/// Mirror of `processEllipsoid`.
pub fn process_ellipsoid(
    geometry: &mut CzmlGeometry,
    packet: &Value,
    source_uri: Option<&str>,
    current_id: Option<&str>,
) {
    let Some(ellipsoid_data) = packet.get("ellipsoid") else {
        return;
    };
    let interval_storage = interval_of(ellipsoid_data);
    let interval = interval_storage.as_ref();

    field!(geometry, ellipsoid_data, "show", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    field!(geometry, ellipsoid_data, "radii", CzmlPropertyType::Cartesian3, interval, source_uri, current_id);
    field!(geometry, ellipsoid_data, "innerRadii", CzmlPropertyType::Cartesian3, interval, source_uri, current_id);
    field!(geometry, ellipsoid_data, "minimumClock", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, ellipsoid_data, "maximumClock", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, ellipsoid_data, "minimumCone", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, ellipsoid_data, "maximumCone", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, ellipsoid_data, "heightReference", CzmlPropertyType::HeightReference, interval, source_uri, current_id);
    field!(geometry, ellipsoid_data, "fill", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    material_field!(geometry, ellipsoid_data, "material", interval, source_uri, current_id);
    field!(geometry, ellipsoid_data, "outline", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    field!(geometry, ellipsoid_data, "outlineColor", CzmlPropertyType::Color, interval, source_uri, current_id);
    field!(geometry, ellipsoid_data, "outlineWidth", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, ellipsoid_data, "stackPartitions", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, ellipsoid_data, "slicePartitions", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, ellipsoid_data, "subdivisions", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, ellipsoid_data, "shadows", CzmlPropertyType::ShadowMode, interval, source_uri, current_id);
    field!(geometry, ellipsoid_data, "distanceDisplayCondition", CzmlPropertyType::DistanceDisplayCondition, interval, source_uri, current_id);
}

// ============================================================================
// processModel (+ nodeTransformations / articulations)
// ============================================================================

/// Mirror of `processNodeTransformations`.
pub fn process_node_transformations(
    geometry: &mut CzmlGeometry,
    node_transformations_data: &Value,
    constrained_interval: Option<&TimeInterval>,
    source_uri: Option<&str>,
    current_id: Option<&str>,
) {
    let combined_storage = compute_combined_interval(node_transformations_data, constrained_interval);
    let combined = combined_storage.as_ref();

    let Some(map) = node_transformations_data.as_object() else {
        return;
    };
    let node_names: Vec<String> = map.keys().cloned().collect();
    for node_name in node_names {
        if node_name == "interval" {
            continue;
        }
        let node_data = &map[&node_name];
        if node_data.is_null() {
            continue;
        }

        let node = geometry
            .node_transformations
            .entry(node_name.clone())
            .or_default();

        for (name, r#type) in [
            ("translation", CzmlPropertyType::Cartesian3),
            ("rotation", CzmlPropertyType::Quaternion),
            ("scale", CzmlPropertyType::Cartesian3),
        ] {
            if let Some(packet_data) = node_data.get(name) {
                let slot = node.entry(name.to_string()).or_insert(None);
                process_packet_data(
                    slot,
                    r#type,
                    Some(packet_data),
                    combined,
                    source_uri,
                    current_id,
                );
            }
        }
    }
}

/// Mirror of `processArticulations`.
pub fn process_articulations(
    geometry: &mut CzmlGeometry,
    articulations_data: &Value,
    constrained_interval: Option<&TimeInterval>,
    source_uri: Option<&str>,
    current_id: Option<&str>,
) {
    let combined_storage = compute_combined_interval(articulations_data, constrained_interval);
    let combined = combined_storage.as_ref();

    let Some(map) = articulations_data.as_object() else {
        return;
    };
    let keys: Vec<String> = map.keys().cloned().collect();
    for key in keys {
        if key == "interval" {
            continue;
        }
        let stage_data = &map[&key];
        if stage_data.is_null() {
            continue;
        }

        let slot = geometry.articulations.entry(key.clone()).or_insert(None);
        process_packet_data(
            slot,
            CzmlPropertyType::Number,
            Some(stage_data),
            combined,
            source_uri,
            current_id,
        );
    }
}

/// Mirror of `processModel`.
pub fn process_model(
    geometry: &mut CzmlGeometry,
    packet: &Value,
    source_uri: Option<&str>,
    current_id: Option<&str>,
) {
    let Some(model_data) = packet.get("model") else {
        return;
    };
    let interval_storage = interval_of(model_data);
    let interval = interval_storage.as_ref();

    field!(geometry, model_data, "show", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    // The model uri is carried by the `gltf` key in CZML.
    if let Some(packet_data) = model_data.get("gltf") {
        let slot = geometry.property_slot("uri");
        process_packet_data(
            slot,
            CzmlPropertyType::Uri,
            Some(packet_data),
            interval,
            source_uri,
            current_id,
        );
    }
    field!(geometry, model_data, "scale", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, model_data, "minimumPixelSize", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, model_data, "maximumScale", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, model_data, "incrementallyLoadTextures", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    field!(geometry, model_data, "runAnimations", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    field!(geometry, model_data, "clampAnimations", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    field!(geometry, model_data, "shadows", CzmlPropertyType::ShadowMode, interval, source_uri, current_id);
    field!(geometry, model_data, "heightReference", CzmlPropertyType::HeightReference, interval, source_uri, current_id);
    field!(geometry, model_data, "silhouetteColor", CzmlPropertyType::Color, interval, source_uri, current_id);
    field!(geometry, model_data, "silhouetteSize", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, model_data, "color", CzmlPropertyType::Color, interval, source_uri, current_id);
    field!(geometry, model_data, "colorBlendMode", CzmlPropertyType::ColorBlendMode, interval, source_uri, current_id);
    field!(geometry, model_data, "colorBlendAmount", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, model_data, "distanceDisplayCondition", CzmlPropertyType::DistanceDisplayCondition, interval, source_uri, current_id);

    if let Some(node_transformations_data) = model_data.get("nodeTransformations") {
        if let Some(packets) = node_transformations_data.as_array() {
            for packet in packets {
                process_node_transformations(geometry, packet, interval, source_uri, current_id);
            }
        } else {
            process_node_transformations(
                geometry,
                node_transformations_data,
                interval,
                source_uri,
                current_id,
            );
        }
    }

    if let Some(articulations_data) = model_data.get("articulations") {
        if let Some(packets) = articulations_data.as_array() {
            for packet in packets {
                process_articulations(geometry, packet, interval, source_uri, current_id);
            }
        } else {
            process_articulations(geometry, articulations_data, interval, source_uri, current_id);
        }
    }
}

// ============================================================================
// processPath
// ============================================================================

/// Mirror of `processPath`.
pub fn process_path(
    geometry: &mut CzmlGeometry,
    packet: &Value,
    source_uri: Option<&str>,
    current_id: Option<&str>,
) {
    let Some(path_data) = packet.get("path") else {
        return;
    };
    let interval_storage = interval_of(path_data);
    let interval = interval_storage.as_ref();

    field!(geometry, path_data, "show", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    field!(geometry, path_data, "leadTime", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, path_data, "trailTime", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, path_data, "width", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, path_data, "resolution", CzmlPropertyType::Number, interval, source_uri, current_id);
    material_field!(geometry, path_data, "material", interval, source_uri, current_id);
    field!(geometry, path_data, "distanceDisplayCondition", CzmlPropertyType::DistanceDisplayCondition, interval, source_uri, current_id);
    field!(geometry, path_data, "relativeTo", CzmlPropertyType::String, interval, source_uri, current_id);
    field!(geometry, path_data, "materialMode", CzmlPropertyType::PathMode, interval, source_uri, current_id);
}

// ============================================================================
// processPolylineVolume
// ============================================================================

/// Mirror of `processPolylineVolume`.
pub fn process_polyline_volume(
    geometry: &mut CzmlGeometry,
    packet: &Value,
    source_uri: Option<&str>,
    current_id: Option<&str>,
) {
    let Some(polyline_volume_data) = packet.get("polylineVolume") else {
        return;
    };
    let interval_storage = interval_of(polyline_volume_data);
    let interval = interval_storage.as_ref();

    if let Some(positions) = polyline_volume_data.get("positions") {
        let slot = geometry.property_slot("positions");
        process_position_array(slot, Some(positions), current_id);
    }
    if let Some(shape) = polyline_volume_data.get("shape") {
        let slot = geometry.property_slot("shape");
        process_shape(slot, Some(shape), current_id);
    }
    field!(geometry, polyline_volume_data, "show", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    field!(geometry, polyline_volume_data, "cornerType", CzmlPropertyType::CornerType, interval, source_uri, current_id);
    field!(geometry, polyline_volume_data, "fill", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    material_field!(geometry, polyline_volume_data, "material", interval, source_uri, current_id);
    field!(geometry, polyline_volume_data, "outline", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    field!(geometry, polyline_volume_data, "outlineColor", CzmlPropertyType::Color, interval, source_uri, current_id);
    field!(geometry, polyline_volume_data, "outlineWidth", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, polyline_volume_data, "granularity", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, polyline_volume_data, "shadows", CzmlPropertyType::ShadowMode, interval, source_uri, current_id);
    field!(geometry, polyline_volume_data, "distanceDisplayCondition", CzmlPropertyType::DistanceDisplayCondition, interval, source_uri, current_id);
}

// ============================================================================
// processRectangle
// ============================================================================

/// Mirror of `processRectangle`.
pub fn process_rectangle(
    geometry: &mut CzmlGeometry,
    packet: &Value,
    source_uri: Option<&str>,
    current_id: Option<&str>,
) {
    let Some(rectangle_data) = packet.get("rectangle") else {
        return;
    };
    let interval_storage = interval_of(rectangle_data);
    let interval = interval_storage.as_ref();

    field!(geometry, rectangle_data, "show", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    field!(geometry, rectangle_data, "coordinates", CzmlPropertyType::Rectangle, interval, source_uri, current_id);
    field!(geometry, rectangle_data, "height", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, rectangle_data, "heightReference", CzmlPropertyType::HeightReference, interval, source_uri, current_id);
    field!(geometry, rectangle_data, "extrudedHeight", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, rectangle_data, "extrudedHeightReference", CzmlPropertyType::HeightReference, interval, source_uri, current_id);
    field!(geometry, rectangle_data, "rotation", CzmlPropertyType::Rotation, interval, source_uri, current_id);
    field!(geometry, rectangle_data, "stRotation", CzmlPropertyType::Rotation, interval, source_uri, current_id);
    field!(geometry, rectangle_data, "granularity", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, rectangle_data, "fill", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    material_field!(geometry, rectangle_data, "material", interval, source_uri, current_id);
    field!(geometry, rectangle_data, "outline", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    field!(geometry, rectangle_data, "outlineColor", CzmlPropertyType::Color, interval, source_uri, current_id);
    field!(geometry, rectangle_data, "outlineWidth", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, rectangle_data, "shadows", CzmlPropertyType::ShadowMode, interval, source_uri, current_id);
    field!(geometry, rectangle_data, "distanceDisplayCondition", CzmlPropertyType::DistanceDisplayCondition, interval, source_uri, current_id);
    field!(geometry, rectangle_data, "classificationType", CzmlPropertyType::ClassificationType, interval, source_uri, current_id);
    field!(geometry, rectangle_data, "zIndex", CzmlPropertyType::Number, interval, source_uri, current_id);
}

// ============================================================================
// processTileset
// ============================================================================

/// Mirror of `processTileset`.
pub fn process_tileset(
    geometry: &mut CzmlGeometry,
    packet: &Value,
    source_uri: Option<&str>,
    current_id: Option<&str>,
) {
    let Some(tileset_data) = packet.get("tileset") else {
        return;
    };
    let interval_storage = interval_of(tileset_data);
    let interval = interval_storage.as_ref();

    field!(geometry, tileset_data, "show", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    field!(geometry, tileset_data, "uri", CzmlPropertyType::Uri, interval, source_uri, current_id);
    field!(geometry, tileset_data, "maximumScreenSpaceError", CzmlPropertyType::Number, interval, source_uri, current_id);
}

// ============================================================================
// processWall
// ============================================================================

/// Mirror of `processWall`.
pub fn process_wall(
    geometry: &mut CzmlGeometry,
    packet: &Value,
    source_uri: Option<&str>,
    current_id: Option<&str>,
) {
    let Some(wall_data) = packet.get("wall") else {
        return;
    };
    let interval_storage = interval_of(wall_data);
    let interval = interval_storage.as_ref();

    field!(geometry, wall_data, "show", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    if let Some(positions) = wall_data.get("positions") {
        let slot = geometry.property_slot("positions");
        process_position_array(slot, Some(positions), current_id);
    }
    if let Some(minimum_heights) = wall_data.get("minimumHeights") {
        let slot = geometry.property_slot("minimumHeights");
        process_array(slot, Some(minimum_heights), current_id);
    }
    if let Some(maximum_heights) = wall_data.get("maximumHeights") {
        let slot = geometry.property_slot("maximumHeights");
        process_array(slot, Some(maximum_heights), current_id);
    }
    field!(geometry, wall_data, "granularity", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, wall_data, "fill", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    material_field!(geometry, wall_data, "material", interval, source_uri, current_id);
    field!(geometry, wall_data, "outline", CzmlPropertyType::Boolean, interval, source_uri, current_id);
    field!(geometry, wall_data, "outlineColor", CzmlPropertyType::Color, interval, source_uri, current_id);
    field!(geometry, wall_data, "outlineWidth", CzmlPropertyType::Number, interval, source_uri, current_id);
    field!(geometry, wall_data, "shadows", CzmlPropertyType::ShadowMode, interval, source_uri, current_id);
    field!(geometry, wall_data, "distanceDisplayCondition", CzmlPropertyType::DistanceDisplayCondition, interval, source_uri, current_id);
}

// ============================================================================
// Polygon hierarchy supplement (PolygonHierarchyProperty)
// ============================================================================

/// Processes the `_positions`/`_holes` halves of `processPolygon` into the
/// sidecar store; `has_hierarchy` mirrors the
/// `polygon.hierarchy = new PolygonHierarchyProperty(polygon)` assignment.
pub fn process_polygon_hierarchy(
    geometry: &mut CzmlGeometry,
    packet: &Value,
    current_id: Option<&str>,
) {
    let Some(polygon_data) = packet.get("polygon") else {
        return;
    };

    if let Some(positions_data) = polygon_data.get("positions") {
        let slot = geometry.property_slot("_positions");
        process_position_array(slot, Some(positions_data), current_id);
    }
    if let Some(holes_data) = polygon_data.get("holes") {
        let slot = geometry.property_slot("_holes");
        process_position_array_of_arrays(slot, Some(holes_data), current_id);
    }

    let has_positions = geometry
        .properties
        .get("_positions")
        .is_some_and(|property| property.is_some());
    let has_holes = geometry
        .properties
        .get("_holes")
        .is_some_and(|property| property.is_some());
    if has_positions || has_holes {
        geometry.has_hierarchy = true;
    }
}

// ============================================================================
// Polyline followSurface supplement (createAdapterProperty)
// ============================================================================

/// Processes the legacy `followSurface` → `arcType` adapter half of
/// `processPolyline` into the sidecar store.
pub fn process_polyline_follow_surface(
    geometry: &mut CzmlGeometry,
    packet: &Value,
    source_uri: Option<&str>,
    current_id: Option<&str>,
) {
    let Some(polyline_data) = packet.get("polyline") else {
        return;
    };

    // For backwards compatibility, adapt CZML followSurface to arcType.
    if polyline_data.get("followSurface").is_some() && polyline_data.get("arcType").is_none() {
        let interval_storage = interval_of(polyline_data);
        let interval = interval_storage.as_ref();

        let mut follow_surface: Option<CzmlProperty> = None;
        if let Some(packet_data) = polyline_data.get("followSurface") {
            process_packet_data(
                &mut follow_surface,
                CzmlPropertyType::Boolean,
                Some(packet_data),
                interval,
                source_uri,
                current_id,
            );
        }
        if let Some(property) = follow_surface {
            geometry.properties.insert(
                "arcType".to_string(),
                Some(CzmlProperty::FollowSurfaceAdapter(Box::new(property))),
            );
        }
    }
}

// ============================================================================
// Packet dispatch (the geometry half of processCzmlPacket)
// ============================================================================

/// Processes every geometry family of one packet into the store (the
/// sidecar half of the JS updater loop).
pub fn process_geometry_packet(
    store: &mut CzmlGeometryStore,
    object_id: &str,
    packet: &Value,
    source_uri: Option<&str>,
) {
    let current_id = Some(object_id);
    let geometry = store.get_or_create(object_id);

    process_wall(&mut geometry.wall, packet, source_uri, current_id);
    process_tileset(&mut geometry.tileset, packet, source_uri, current_id);
    process_rectangle(&mut geometry.rectangle, packet, source_uri, current_id);
    process_polyline_volume(&mut geometry.polyline_volume, packet, source_uri, current_id);
    process_polyline_follow_surface(&mut geometry.polyline, packet, source_uri, current_id);
    process_polygon_hierarchy(&mut geometry.polygon, packet, current_id);
    process_path(&mut geometry.path, packet, source_uri, current_id);
    process_model(&mut geometry.model, packet, source_uri, current_id);
    process_ellipsoid(&mut geometry.ellipsoid, packet, source_uri, current_id);
    process_ellipse(&mut geometry.ellipse, packet, source_uri, current_id);
    process_cylinder(&mut geometry.cylinder, packet, source_uri, current_id);
    process_corridor(&mut geometry.corridor, packet, source_uri, current_id);
    process_box(&mut geometry.r#box, packet, source_uri, current_id);
}
