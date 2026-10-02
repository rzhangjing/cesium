//! 用于 glTF up-axis 处理的坐标轴转换矩阵。

use glam::DMat4;

/// 描述 x、y、z 轴及辅助转换函数的枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Axis {
    /// 表示 x 轴。
    X = 0,
    /// 表示 y 轴。
    Y = 1,
    /// 表示 z 轴。
    Z = 2,
}

impl Axis {
    /// 按名称获取坐标轴（"X"/"Y"/"Z"，大小写敏感）。
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "X" => Some(Axis::X),
            "Y" => Some(Axis::Y),
            "Z" => Some(Axis::Z),
            _ => None,
        }
    }
}

/// 用于从 y-up 转换到 z-up 的矩阵。
/// 绕 X 轴旋转 PI/2。
pub const Y_UP_TO_Z_UP: DMat4 = DMat4::from_cols_array(&[
    1.0, 0.0, 0.0, 0.0, // 第 0 列
    0.0, 0.0, 1.0, 0.0, // 第 1 列
    0.0, -1.0, 0.0, 0.0, // 第 2 列
    0.0, 0.0, 0.0, 1.0, // 第 3 列
]);

/// 用于从 z-up 转换到 y-up 的矩阵。
/// 绕 X 轴旋转 -PI/2。
pub const Z_UP_TO_Y_UP: DMat4 = DMat4::from_cols_array(&[
    1.0, 0.0, 0.0, 0.0, // 第 0 列
    0.0, 0.0, -1.0, 0.0, // 第 1 列
    0.0, 1.0, 0.0, 0.0, // 第 2 列
    0.0, 0.0, 0.0, 1.0, // 第 3 列
]);

/// 用于从 x-up 转换到 z-up 的矩阵。
/// 绕 Y 轴旋转 -PI/2。
pub const X_UP_TO_Z_UP: DMat4 = DMat4::from_cols_array(&[
    0.0, 0.0, 1.0, 0.0, // 第 0 列
    0.0, 1.0, 0.0, 0.0, // 第 1 列
    -1.0, 0.0, 0.0, 0.0, // 第 2 列
    0.0, 0.0, 0.0, 1.0, // 第 3 列
]);

/// 用于从 z-up 转换到 x-up 的矩阵。
/// 绕 Y 轴旋转 PI/2。
pub const Z_UP_TO_X_UP: DMat4 = DMat4::from_cols_array(&[
    0.0, 0.0, -1.0, 0.0, // 第 0 列
    0.0, 1.0, 0.0, 0.0, // 第 1 列
    1.0, 0.0, 0.0, 0.0, // 第 2 列
    0.0, 0.0, 0.0, 1.0, // 第 3 列
]);

/// 用于从 x-up 转换到 y-up 的矩阵。
/// 绕 Z 轴旋转 PI/2。
pub const X_UP_TO_Y_UP: DMat4 = DMat4::from_cols_array(&[
    0.0, 1.0, 0.0, 0.0, // 第 0 列
    -1.0, 0.0, 0.0, 0.0, // 第 1 列
    0.0, 0.0, 1.0, 0.0, // 第 2 列
    0.0, 0.0, 0.0, 1.0, // 第 3 列
]);

/// 用于从 y-up 转换到 x-up 的矩阵。
/// 绕 Z 轴旋转 -PI/2。
pub const Y_UP_TO_X_UP: DMat4 = DMat4::from_cols_array(&[
    0.0, -1.0, 0.0, 0.0, // 第 0 列
    1.0, 0.0, 0.0, 0.0, // 第 1 列
    0.0, 0.0, 1.0, 0.0, // 第 2 列
    0.0, 0.0, 0.0, 1.0, // 第 3 列
]);
