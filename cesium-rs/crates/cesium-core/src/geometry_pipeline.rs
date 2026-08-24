//! Ported from `packages/engine/Source/Core/GeometryPipeline.js`.
//!
//! Content pipeline functions for geometries.
//!
//! This module is split into sub-modules for maintainability:
//! - `wireframe` – toWireframe, createLineSegmentsForVectors
//! - `attribute_locations` – createAttributeLocations
//! - `reorder` – reorderForPreVertexCache, reorderForPostVertexCache
//! - `fit_indices` – fitToUnsignedShortIndices
//! - `project` – projectTo2D
//! - `encode` – encodeAttribute
//! - `transform` – transformToWorldCoordinates
//! - `combine` – combineInstances
//! - `normals` – computeNormal, computeTangentAndBitangent
//! - `compress` – compressVertices
//! - `split` – splitLongitude

pub mod wireframe;
pub mod attribute_locations;
pub mod reorder;
pub mod fit_indices;
pub mod project;
pub mod encode;
pub mod transform;
pub mod combine;
pub mod normals;
pub mod compress;
pub mod split;

use std::collections::HashMap;

use crate::geometry::Geometry;
use crate::geometry_instance::GeometryInstance;

/// Content pipeline functions for geometries.
///
/// In CesiumJS, `GeometryPipeline` is a plain object whose properties are
/// static functions. The Rust port mirrors this API by collecting all the
/// static geometry-processing functions behind a single struct so that
/// callers can write `GeometryPipeline::compute_normal(&mut geometry)` just
/// like the JS `GeometryPipeline.computeNormal(geometry)`.
pub struct GeometryPipeline;

impl GeometryPipeline {
    /// Computes per-vertex normals for a geometry containing `TRIANGLES`.
    ///
    /// Port of `GeometryPipeline.computeNormal(geometry)`.
    pub fn compute_normal(geometry: &mut Geometry) {
        normals::compute_normal(geometry);
    }

    /// Computes per-vertex tangents and bitangents for a geometry containing
    /// `TRIANGLES`.
    ///
    /// Port of `GeometryPipeline.computeTangentAndBitangent(geometry)`.
    pub fn compute_tangent_and_bitangent(geometry: &mut Geometry) {
        normals::compute_tangent_and_bitangent(geometry);
    }

    /// Combines geometry from several `GeometryInstance` objects into one
    /// geometry.
    ///
    /// Port of `GeometryPipeline.combineInstances(instances)`.
    pub fn combine_instances(instances: &[GeometryInstance]) -> Vec<Geometry> {
        combine::combine_instances(instances)
    }

    /// Compresses and packs geometry normal attribute values to save memory.
    ///
    /// Port of `GeometryPipeline.compressVertices(geometry)`.
    pub fn compress_vertices(geometry: &mut Geometry) {
        compress::compress_vertices(geometry);
    }

    /// Encodes floating-point geometry attribute values as two separate
    /// attributes to improve rendering precision.
    ///
    /// Port of `GeometryPipeline.encodeAttribute(geometry, attributeName,
    /// attributeHighName, attributeLowName)`.
    pub fn encode_attribute(
        geometry: &mut Geometry,
        attribute_name: &str,
        attribute_high_name: &str,
        attribute_low_name: &str,
    ) {
        encode::encode_attribute(geometry, attribute_name, attribute_high_name, attribute_low_name);
    }

    /// Projects a geometry's 3D `position` attribute to 2D.
    ///
    /// Port of `GeometryPipeline.projectTo2D(geometry, attributeName,
    /// attributeName3D, attributeName2D, projection)`.
    pub fn project_to_2d(
        geometry: &mut Geometry,
        attribute_name: &str,
        attribute_name_3d: &str,
        attribute_name_2d: &str,
        projection: Option<&crate::geographic_projection::GeographicProjection>,
    ) {
        project::project_to_2d(
            geometry,
            attribute_name,
            attribute_name_3d,
            attribute_name_2d,
            projection,
        );
    }

    /// Transforms a geometry instance to world coordinates.
    ///
    /// Port of `GeometryPipeline.transformToWorldCoordinates(instance)`.
    pub fn transform_to_world_coordinates(instance: &mut GeometryInstance) {
        transform::transform_to_world_coordinates(instance);
    }

    /// Converts triangle indices to line indices for wireframe rendering.
    ///
    /// Port of `GeometryPipeline.toWireframe(geometry)`.
    pub fn to_wireframe(geometry: &mut Geometry) {
        wireframe::to_wireframe(geometry);
    }

    /// Creates an object that maps attribute names to unique locations.
    ///
    /// Port of `GeometryPipeline.createAttributeLocations(geometry)`.
    pub fn create_attribute_locations(geometry: &Geometry) -> HashMap<String, u32> {
        attribute_locations::create_attribute_locations(geometry)
    }

    /// Reorders a geometry's attributes and `indices` to achieve better
    /// performance from the GPU's pre-vertex-shader cache.
    ///
    /// Port of `GeometryPipeline.reorderForPreVertexCache(geometry)`.
    pub fn reorder_for_pre_vertex_cache(geometry: &mut Geometry) {
        reorder::reorder_for_pre_vertex_cache(geometry);
    }

    /// Reorders a geometry's `indices` to achieve better performance from the
    /// GPU's post vertex-shader cache by using the Tipsify algorithm.
    ///
    /// Port of `GeometryPipeline.reorderForPostVertexCache(geometry,
    /// cacheCapacity)`.
    pub fn reorder_for_post_vertex_cache(geometry: &mut Geometry, cache_capacity: Option<u32>) {
        reorder::reorder_for_post_vertex_cache(geometry, cache_capacity);
    }

    /// Splits a geometry into multiple geometries, if necessary, to ensure
    /// that indices fit into unsigned shorts.
    ///
    /// Port of `GeometryPipeline.fitToUnsignedShortIndices(geometry)`.
    pub fn fit_to_unsigned_short_indices(geometry: &Geometry) -> Vec<Geometry> {
        fit_indices::fit_to_unsigned_short_indices(geometry)
    }

    /// Splits a `Geometry` at the international date line, producing two
    /// geometries (west and east hemispheres).
    ///
    /// Port of `GeometryPipeline.splitLongitude(instance)`.
    pub fn split_longitude(instance: &mut GeometryInstance) {
        split::split_longitude(instance);
    }
}
