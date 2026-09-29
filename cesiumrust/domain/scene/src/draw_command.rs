//! 绘制命令生成与渲染通道管理。
//!
//! 映射到 CesiumJS `Renderer/DrawCommand.js` 与 `Scene/Pass.js`

use glam::DMat4;
use serde::{Deserialize, Serialize};

/// 渲染通道类型。
///
/// 映射到 CesiumJS `Scene/Pass.js`
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
pub enum RenderPass {
    /// 环境通道（天空、大气）。
    Environment = 0,
    /// 3D Tiles 与地形。
    Cesium3DTile = 1,
    /// 不透明图元。
    #[default]
    Opaque = 2,
    /// 半透明图元。
    Translucent = 3,
    /// 叠加通道（标签、多段线）。
    Overlay = 4,
}

/// 用于渲染的混合状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BlendState {
    /// 无混合（不透明）。
    #[default]
    Opaque,
    /// Alpha 混合。
    AlphaBlend,
    /// 叠加混合。
    Additive,
    /// 预乘 Alpha。
    PremultipliedAlpha,
}

/// 深度测试状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepthState {
    /// 是否启用深度测试。
    pub enabled: bool,
    /// 是否启用深度写入。
    pub write_enabled: bool,
}

impl Default for DepthState {
    fn default() -> Self {
        Self {
            enabled: true,
            write_enabled: true,
        }
    }
}

/// 一个表示单次渲染操作的绘制命令。
///
/// 映射到 CesiumJS `Renderer/DrawCommand.js`
#[derive(Debug, Clone)]
pub struct DrawCommand {
    /// 该命令所属的渲染通道。
    pub pass: RenderPass,

    /// 模型矩阵（局部到世界）。
    pub model_matrix: DMat4,

    /// 网格/几何资产 ID。
    pub geometry_id: u64,

    /// 材质/着色器程序 ID。
    pub material_id: u64,

    /// 可选的纹理 ID。
    pub texture_ids: Vec<u64>,

    /// 混合状态。
    pub blend_state: BlendState,

    /// 深度状态。
    pub depth_state: DepthState,

    /// 是否剔除背面。
    pub cull_face: bool,

    /// 用于排序的排序键（距离或自定义）。
    pub sort_key: f64,

    /// 实例数量（用于实例化渲染）。
    pub instance_count: u32,

    /// 该命令是否投射阴影。
    pub casts_shadows: bool,

    /// 该命令是否接收阴影。
    pub receives_shadows: bool,

    /// 用于对象选择的拾取 ID。
    pub pick_id: Option<u64>,
}

impl Default for DrawCommand {
    fn default() -> Self {
        Self {
            pass: RenderPass::Opaque,
            model_matrix: DMat4::IDENTITY,
            geometry_id: 0,
            material_id: 0,
            texture_ids: Vec::new(),
            blend_state: BlendState::Opaque,
            depth_state: DepthState::default(),
            cull_face: true,
            sort_key: 0.0,
            instance_count: 1,
            casts_shadows: true,
            receives_shadows: true,
            pick_id: None,
        }
    }
}

impl DrawCommand {
    /// 使用给定的几何与材质创建一个新的绘制命令。
    pub fn new(geometry_id: u64, material_id: u64) -> Self {
        Self {
            geometry_id,
            material_id,
            ..Default::default()
        }
    }

    /// 设置模型矩阵。
    pub fn with_model_matrix(mut self, matrix: DMat4) -> Self {
        self.model_matrix = matrix;
        self
    }

    /// 设置渲染通道。
    pub fn with_pass(mut self, pass: RenderPass) -> Self {
        self.pass = pass;
        self
    }

    /// 设置混合状态。
    pub fn with_blend_state(mut self, blend: BlendState) -> Self {
        self.blend_state = blend;
        self
    }

    /// 设置排序键。
    pub fn with_sort_key(mut self, key: f64) -> Self {
        self.sort_key = key;
        self
    }

    /// 设置拾取 ID。
    pub fn with_pick_id(mut self, id: u64) -> Self {
        self.pick_id = Some(id);
        self
    }

    /// 若这是一个透明命令则返回 true。
    pub fn is_transparent(&self) -> bool {
        !matches!(self.blend_state, BlendState::Opaque)
    }
}

/// 按渲染通道组织的绘制命令集合。
#[derive(Debug, Default)]
pub struct RenderCommandList {
    /// 按通道组织的命令。
    passes: std::collections::BTreeMap<RenderPass, Vec<DrawCommand>>,
}

impl RenderCommandList {
    /// 创建一个空的命令列表。
    pub fn new() -> Self {
        Self {
            passes: std::collections::BTreeMap::new(),
        }
    }

    /// 向列表添加一个命令。
    pub fn push(&mut self, command: DrawCommand) {
        self.passes.entry(command.pass).or_default().push(command);
    }

    /// 返回命令总数。
    pub fn len(&self) -> usize {
        self.passes.values().map(|v| v.len()).sum()
    }

    /// 若列表为空则返回 true。
    pub fn is_empty(&self) -> bool {
        self.passes.values().all(|v| v.is_empty())
    }

    /// 返回特定通道的命令。
    pub fn commands_for_pass(&self, pass: RenderPass) -> &[DrawCommand] {
        self.passes.get(&pass).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// 对每个通道内的命令排序。
    ///
    /// 不透明通道按从前到后排序。
    /// 透明通道按从后到前排序。
    pub fn sort(&mut self) {
        for (pass, commands) in self.passes.iter_mut() {
            match pass {
                RenderPass::Translucent | RenderPass::Overlay => {
                    // 透明：从后到前
                    commands.sort_by(|a, b| {
                        b.sort_key.partial_cmp(&a.sort_key).unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
                _ => {
                    // 不透明：从前到后（early-z 优化）
                    commands.sort_by(|a, b| {
                        a.sort_key.partial_cmp(&b.sort_key).unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
            }
        }
    }

    /// 返回一个遍历所有通道及其命令的迭代器。
    pub fn iter(&self) -> impl Iterator<Item = (&RenderPass, &Vec<DrawCommand>)> {
        self.passes.iter()
    }

    /// 清除所有命令。
    pub fn clear(&mut self) {
        self.passes.clear();
    }
}

/// 用于渲染的帧统计。
#[derive(Debug, Clone, Default)]
pub struct FrameStatistics {
    /// 已执行的绘制命令数量。
    pub draw_calls: usize,

    /// 已渲染的三角形数量。
    pub triangles: u64,

    /// 已处理的顶点数量。
    pub vertices: u64,

    /// 纹理绑定次数。
    pub texture_binds: usize,

    /// 着色器切换次数。
    pub shader_switches: usize,

    /// 被剔除对象的数量。
    pub culled_objects: usize,

    /// 帧时间（毫秒）。
    pub frame_time_ms: f64,
}

impl FrameStatistics {
    /// 重置所有统计。
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// 将另一个统计对象合并到本对象中。
    pub fn merge(&mut self, other: &FrameStatistics) {
        self.draw_calls += other.draw_calls;
        self.triangles += other.triangles;
        self.vertices += other.vertices;
        self.texture_binds += other.texture_binds;
        self.shader_switches += other.shader_switches;
        self.culled_objects += other.culled_objects;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draw_command_default() {
        let cmd = DrawCommand::default();
        assert_eq!(cmd.pass, RenderPass::Opaque);
        assert_eq!(cmd.blend_state, BlendState::Opaque);
        assert!(cmd.depth_state.enabled);
        assert!(cmd.cull_face);
        assert!(!cmd.is_transparent());
    }

    #[test]
    fn test_draw_command_builder() {
        let cmd = DrawCommand::new(1, 2)
            .with_pass(RenderPass::Translucent)
            .with_blend_state(BlendState::AlphaBlend)
            .with_sort_key(100.0)
            .with_pick_id(42);

        assert_eq!(cmd.geometry_id, 1);
        assert_eq!(cmd.material_id, 2);
        assert_eq!(cmd.pass, RenderPass::Translucent);
        assert_eq!(cmd.blend_state, BlendState::AlphaBlend);
        assert_eq!(cmd.sort_key, 100.0);
        assert_eq!(cmd.pick_id, Some(42));
        assert!(cmd.is_transparent());
    }

    #[test]
    fn test_render_command_list() {
        let mut list = RenderCommandList::new();

        list.push(DrawCommand::new(1, 1).with_pass(RenderPass::Opaque));
        list.push(DrawCommand::new(2, 2).with_pass(RenderPass::Translucent));
        list.push(DrawCommand::new(3, 3).with_pass(RenderPass::Opaque));

        assert_eq!(list.len(), 3);
        assert_eq!(list.commands_for_pass(RenderPass::Opaque).len(), 2);
        assert_eq!(list.commands_for_pass(RenderPass::Translucent).len(), 1);
    }

    #[test]
    fn test_render_command_list_sort() {
        let mut list = RenderCommandList::new();

        // 添加排序键不同的不透明命令
        list.push(DrawCommand::new(1, 1).with_pass(RenderPass::Opaque).with_sort_key(100.0));
        list.push(DrawCommand::new(2, 2).with_pass(RenderPass::Opaque).with_sort_key(50.0));
        list.push(DrawCommand::new(3, 3).with_pass(RenderPass::Opaque).with_sort_key(200.0));

        list.sort();

        let opaque = list.commands_for_pass(RenderPass::Opaque);
        // 从前到后：50, 100, 200
        assert_eq!(opaque[0].geometry_id, 2);
        assert_eq!(opaque[1].geometry_id, 1);
        assert_eq!(opaque[2].geometry_id, 3);
    }

    #[test]
    fn test_translucent_sort_back_to_front() {
        let mut list = RenderCommandList::new();

        list.push(DrawCommand::new(1, 1).with_pass(RenderPass::Translucent).with_sort_key(100.0));
        list.push(DrawCommand::new(2, 2).with_pass(RenderPass::Translucent).with_sort_key(50.0));

        list.sort();

        let translucent = list.commands_for_pass(RenderPass::Translucent);
        // 从后到前：100, 50
        assert_eq!(translucent[0].geometry_id, 1);
        assert_eq!(translucent[1].geometry_id, 2);
    }

    #[test]
    fn test_render_pass_ordering() {
        assert!(RenderPass::Environment < RenderPass::Cesium3DTile);
        assert!(RenderPass::Cesium3DTile < RenderPass::Opaque);
        assert!(RenderPass::Opaque < RenderPass::Translucent);
        assert!(RenderPass::Translucent < RenderPass::Overlay);
    }

    #[test]
    fn test_frame_statistics() {
        let mut stats = FrameStatistics {
            draw_calls: 100,
            triangles: 50000,
            ..Default::default()
        };

        let other = FrameStatistics {
            draw_calls: 50,
            triangles: 25000,
            ..Default::default()
        };

        stats.merge(&other);

        assert_eq!(stats.draw_calls, 150);
        assert_eq!(stats.triangles, 75000);
    }

    #[test]
    fn test_command_list_clear() {
        let mut list = RenderCommandList::new();
        list.push(DrawCommand::new(1, 1));
        list.push(DrawCommand::new(2, 2));

        assert!(!list.is_empty());

        list.clear();

        assert!(list.is_empty());
        assert_eq!(list.len(), 0);
    }
}
