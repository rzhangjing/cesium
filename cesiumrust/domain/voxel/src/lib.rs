//! cesium-voxel：用于体数据渲染的体素形状系统
//!
//! 映射到 CesiumJS：
//! - `Scene/VoxelShape.js` — 形状接口
//! - `Scene/VoxelBoxShape.js` — 长方体形状
//! - `Scene/VoxelCylinderShape.js` — 圆柱体形状
//! - `Scene/VoxelEllipsoidShape.js` — 椭球体形状
//! - `Scene/VoxelShapeType.js` — 形状类型枚举
//! - `Scene/VoxelCell.js` — 单元元数据访问
//! - `Scene/VoxelTraversal.js` — LOD 遍历
//!
//! # 特性
//! - 三种形状类型：Box、Cylinder、Ellipsoid
//! - 边界裁剪与渲染边界计算
//! - 用于纹理映射的 UV 空间变换
//! - 用于 LOD 的瓦片与采样 OBB 计算
//! - 单元元数据访问与拾取
//! - 带屏幕空间误差的 LOD 遍历

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
