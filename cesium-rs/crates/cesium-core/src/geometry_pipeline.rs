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
