//! cesium-voxel：用于体数据渲染的体素形状系统
//!
//! 本 crate 划分以下子模块：
//! - shape：形状接口与类型枚举、OBB/包围球等公共几何类型；
//! - box_shape / cylinder_shape / ellipsoid_shape：三种具体体素形状；
//! - cell：体素单元的元数据访问与拾取；
//! - traversal：带屏幕空间误差（SSE）的 LOD 遍历。
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
