//! 用于影像图层分割的分割方向。
//!
//! 映射到 CesiumJS `Scene/SplitDirection.js`。

use serde::{Deserialize, Serialize};

/// 相对于分割位置显示某个图元或 ImageryLayer 的方向。
///
/// 映射到 CesiumJS `Scene/SplitDirection.js`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SplitDirection {
    /// 显示在分割位置的左侧。
    Left,
    /// 始终显示（不分割）。
    #[default]
    None,
    /// 显示在分割位置的右侧。
    Right,
}

impl SplitDirection {
    /// 获取用于 shader 的数值。
    ///
    /// - Left: -1.0
    /// - None: 0.0
    /// - Right: 1.0
    pub fn to_shader_value(&self) -> f64 {
        match self {
            Self::Left => -1.0,
            Self::None => 0.0,
            Self::Right => 1.0,
        }
    }

    /// 从 shader 数值创建。
    pub fn from_shader_value(value: f64) -> Self {
        if value < -0.5 {
            Self::Left
        } else if value > 0.5 {
            Self::Right
        } else {
            Self::None
        }
    }

    /// 检查分割是否处于激活状态（非 None）。
    pub fn is_split(&self) -> bool {
        !matches!(self, Self::None)
    }

    /// 检查在给定分割位置下此方向是否应当显示。
    ///
    /// `split_position` 处于 [0, 1] 范围（0 = 左边缘，1 = 右边缘）。
    /// `screen_x` 是归一化的屏幕 X 坐标 [0, 1]。
    pub fn should_show_at(&self, screen_x: f64, split_position: f64) -> bool {
        match self {
            Self::None => true,
            Self::Left => screen_x <= split_position,
            Self::Right => screen_x > split_position,
        }
    }
}

/// 场景的分割器配置。
///
/// 映射到 CesiumJS `Scene/Splitter.js` 和 `Scene.splitPosition`。
#[derive(Debug, Clone, PartialEq)]
pub struct SplitterConfig {
    /// 分割是否启用。
    pub enabled: bool,
    /// 分割位置，以屏幕宽度分数表示 [0, 1]。
    pub split_position: f64,
}

impl Default for SplitterConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            split_position: 0.5,
        }
    }
}

impl SplitterConfig {
    /// 创建一个新的分割器配置。
    pub fn new(enabled: bool, split_position: f64) -> Self {
        Self {
            enabled,
            split_position: split_position.clamp(0.0, 1.0),
        }
    }

    /// 设置分割位置，限制到 [0, 1]。
    pub fn set_split_position(&mut self, position: f64) {
        self.split_position = position.clamp(0.0, 1.0);
    }

    /// 针对给定的视口宽度获取以像素计的分割位置。
    pub fn split_position_pixels(&self, viewport_width: f64) -> f64 {
        self.split_position * viewport_width
    }

    /// 修改片元 shader 以纳入分割逻辑。
    ///
    /// 返回要插入的额外 shader 代码。
    ///
    /// **GLSL 形式** — 镜像上游 `SplitDirection.js` / `czm_splitPosition`，
    /// 使用 `gl_FragCoord`（像素空间）和 `discard`。为 GLSL blueprint 奇偶校验测试而逐字保留。
    pub fn shader_modification(&self) -> &str {
        if self.enabled {
            r#"
    // Split direction check
    float splitPosition = czm_splitPosition;
    if (v_splitDirection < 0.0 && gl_FragCoord.x > splitPosition) {
        discard;
    }
    if (v_splitDirection > 0.0 && gl_FragCoord.x <= splitPosition) {
        discard;
    }
"#
        } else {
            ""
        }
    }

    /// [`Self::shader_modification`] 面向 Bevy/wgpu 后端的 WGSL 形式。
    ///
    /// 与 GLSL 源码相比有两个机械性差异，两者都由目标语言强制要求
    /// （记录为 `docs/deviations.md#dev-034`）：
    /// - WGSL 没有 `discard` 语句；惯用的等价写法是在写入颜色输出之前
    ///   从片元入口点提前 `return`（保留渲染目标采样不变，与 `discard` 对应）。
    /// - `gl_FragCoord` 变为 `@builtin(position)` 内建量（同样处于像素空间，
    ///   `.xy` 从左上角起算），而 `czm_splitPosition` 变为一个普通的
    ///   `split_position_px` 标量（适配器在打包前会将 [0, 1] 分数乘以视口宽度）。
    ///
    /// `split_direction` 是每个图元的 varying（`-1`/`0`/`1`，恰好是
    /// [`SplitDirection::to_shader_value`] 产生的值）。
    pub fn wgsl_shader_modification(&self) -> &str {
        if self.enabled {
            r#"
    // Split direction check (WGSL)
    if (split_direction < 0.0 && position.x > split_position_px) {
        return;
    }
    if (split_direction > 0.0 && position.x <= split_position_px) {
        return;
    }
"#
        } else {
            ""
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_direction_default() {
        assert_eq!(SplitDirection::default(), SplitDirection::None);
    }

    #[test]
    fn test_split_direction_shader_values() {
        assert_eq!(SplitDirection::Left.to_shader_value(), -1.0);
        assert_eq!(SplitDirection::None.to_shader_value(), 0.0);
        assert_eq!(SplitDirection::Right.to_shader_value(), 1.0);
    }

    #[test]
    fn test_split_direction_from_shader_value() {
        assert_eq!(SplitDirection::from_shader_value(-1.0), SplitDirection::Left);
        assert_eq!(SplitDirection::from_shader_value(0.0), SplitDirection::None);
        assert_eq!(SplitDirection::from_shader_value(1.0), SplitDirection::Right);
        assert_eq!(SplitDirection::from_shader_value(-0.3), SplitDirection::None);
        assert_eq!(SplitDirection::from_shader_value(0.3), SplitDirection::None);
    }

    #[test]
    fn test_split_direction_is_split() {
        assert!(SplitDirection::Left.is_split());
        assert!(!SplitDirection::None.is_split());
        assert!(SplitDirection::Right.is_split());
    }

    #[test]
    fn test_split_direction_should_show() {
        let split_pos = 0.5;

        // None 始终显示
        assert!(SplitDirection::None.should_show_at(0.0, split_pos));
        assert!(SplitDirection::None.should_show_at(0.5, split_pos));
        assert!(SplitDirection::None.should_show_at(1.0, split_pos));

        // Left 在分割处或之前显示
        assert!(SplitDirection::Left.should_show_at(0.0, split_pos));
        assert!(SplitDirection::Left.should_show_at(0.5, split_pos));
        assert!(!SplitDirection::Left.should_show_at(0.6, split_pos));

        // Right 在分割之后显示
        assert!(!SplitDirection::Right.should_show_at(0.0, split_pos));
        assert!(!SplitDirection::Right.should_show_at(0.5, split_pos));
        assert!(SplitDirection::Right.should_show_at(0.6, split_pos));
    }

    #[test]
    fn test_splitter_config_default() {
        let config = SplitterConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.split_position, 0.5);
    }

    #[test]
    fn test_splitter_config_new() {
        let config = SplitterConfig::new(true, 0.7);
        assert!(config.enabled);
        assert_eq!(config.split_position, 0.7);

        // 被限制
        let config2 = SplitterConfig::new(true, 1.5);
        assert_eq!(config2.split_position, 1.0);

        let config3 = SplitterConfig::new(true, -0.5);
        assert_eq!(config3.split_position, 0.0);
    }

    #[test]
    fn test_splitter_config_set_position() {
        let mut config = SplitterConfig::default();
        config.set_split_position(0.3);
        assert_eq!(config.split_position, 0.3);

        config.set_split_position(2.0);
        assert_eq!(config.split_position, 1.0);
    }

    #[test]
    fn test_splitter_config_pixels() {
        let config = SplitterConfig::new(true, 0.5);
        assert_eq!(config.split_position_pixels(1920.0), 960.0);
        assert_eq!(config.split_position_pixels(1080.0), 540.0);
    }

    #[test]
    fn test_splitter_shader_modification() {
        let disabled = SplitterConfig::default();
        assert_eq!(disabled.shader_modification(), "");

        let enabled = SplitterConfig::new(true, 0.5);
        let shader = enabled.shader_modification();
        assert!(shader.contains("czm_splitPosition"));
        assert!(shader.contains("discard"));
    }

    #[test]
    fn test_splitter_wgsl_shader_modification() {
        // WGSL 变体在禁用时必须为空，与 GLSL 完全一致。
        let disabled = SplitterConfig::default();
        assert_eq!(disabled.wgsl_shader_modification(), "");

        let enabled = SplitterConfig::new(true, 0.5);
        let shader = enabled.wgsl_shader_modification();
        // WGSL 没有 `discard` / `gl_FragCoord` / `czm_` —— 该变体的全部意义
        // 就在于它改用目标语言的拼写。
        assert!(!shader.contains("discard"), "WGSL must not use GLSL `discard`");
        assert!(
            !shader.contains("gl_FragCoord"),
            "WGSL must not reference `gl_FragCoord`"
        );
        assert!(shader.contains("position.x"), "uses the `position` builtin");
        assert!(shader.contains("return;"), "early-return replaces `discard`");
        assert!(shader.contains("split_position_px"));
        assert!(shader.contains("split_direction"));
    }

    #[test]
    fn test_split_direction_serialization() {
        let dir = SplitDirection::Left;
        let json = serde_json::to_string(&dir).unwrap();
        let deserialized: SplitDirection = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, SplitDirection::Left);
    }
}
