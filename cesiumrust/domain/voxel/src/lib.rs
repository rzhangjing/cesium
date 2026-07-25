//! cesium-voxel: Voxel shape system for volumetric data rendering
//!
//! Maps to CesiumJS:
//! - `Scene/VoxelShape.js` — shape interface
//! - `Scene/VoxelBoxShape.js` — box shape
//! - `Scene/VoxelCylinderShape.js` — cylinder shape
//! - `Scene/VoxelEllipsoidShape.js` — ellipsoid shape
//! - `Scene/VoxelShapeType.js` — shape type enum
//! - `Scene/VoxelCell.js` — cell metadata access
//! - `Scene/VoxelTraversal.js` — LOD traversal
//!
//! # Features
//! - Three shape types: Box, Cylinder, Ellipsoid
//! - Bounds clipping and render bounds computation
//! - UV space transformations for texture mapping
//! - Tile and sample OBB computation for LOD
//! - Cell metadata access and picking
//! - LOD traversal with screen-space error

pub mod shape;
pub mod box_shape;
pub mod cylinder_shape;
pub mod ellipsoid_shape;
pub mod cell;
pub mod traversal;

pub use shape::{VoxelShapeType, VoxelShape, OrientedBoundingBox, BoundingSphere};
pub use box_shape::VoxelBoxShape;
pub use cylinder_shape::VoxelCylinderShape;
pub use ellipsoid_shape::VoxelEllipsoidShape;
pub use cell::VoxelCell;
pub use traversal::{VoxelTraversal, TraversalResult, SpatialNode};
