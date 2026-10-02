//! 调试检查器的领域模型。
//!
//! 提供面向运行时可视化与诊断的纯数据模型：
//! - [`DebugInspector`] 汇聚各类调试开关与逐瓦片信息；
//! - [`PerformanceOverlay`] 承载 HUD 显示所需的帧率与内存采样；
//! - [`TilesetInspector`] 控制 3D Tiles 内容的检查与高亮。
//!
//! 本模块仅描述“检查什么”，具体的绘制与采集由渲染适配器完成。

use std::collections::HashMap;

/// 用于场景诊断的调试检查器。
///
/// 聚合各类可视化开关、逐瓦片调试信息与帧统计，供上层按需读取。
#[derive(Debug, Clone, Default)]
pub struct DebugInspector {
    /// 检查器是否启用。
    pub enabled: bool,
    /// 显示线框渲染。
    pub wireframe: bool,
    /// 显示包围体。
    pub show_bounding_volumes: bool,
    /// 显示瓦片坐标。
    pub show_tile_coordinates: bool,
    /// 显示渲染统计覆盖层。
    pub show_statistics: bool,
    /// 显示视锥剔除可视化。
    pub show_frustums: bool,
    /// 显示深度缓冲可视化。
    pub show_depth: bool,
    /// 显示法线可视化。
    pub show_normals: bool,
    /// 显示拾取调试颜色。
    pub show_pick_debug: bool,
    /// 瓦片的高亮模式。
    pub highlight_mode: HighlightMode,
    /// 逐瓦片的调试信息。
    pub tile_debug_info: HashMap<u64, TileDebugInfo>,
    /// 帧统计。
    pub frame_stats: FrameDebugStats,
}

/// 用于瓦片检查的高亮模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HighlightMode {
    /// 不做任何高亮（默认）。
    #[default]
    None,
    /// 按树中深度高亮。
    Depth,
    /// 按几何误差高亮。
    GeometricError,
    /// 按到相机的距离高亮。
    Distance,
    /// 按渲染状态高亮。
    RenderState,
    /// 每瓦片随机颜色。
    RandomColor,
}

/// 逐瓦片的调试信息。
#[derive(Debug, Clone, Default)]
pub struct TileDebugInfo {
    /// 瓦片唯一标识。
    pub tile_id: u64,
    /// 在瓦片树中的深度（根为 0）。
    pub depth: u32,
    /// 几何误差（世界坐标单位）。
    pub geometric_error: f64,
    /// 瓦片中心到相机的距离。
    pub distance_to_camera: f64,
    /// 投影后的屏幕空间误差（SSE，像素）。
    pub screen_space_error: f64,
    /// 本帧是否被实际渲染。
    pub is_rendered: bool,
    /// 本帧遍历是否访问过该瓦片。
    pub is_visited: bool,
    /// 是否被拾取选中。
    pub is_selected: bool,
    /// 内容类型标识（如 b3dm、pnts）。
    pub content_type: String,
    /// 三角形数量。
    pub triangles_count: u64,
    /// 顶点数量。
    pub vertices_count: u64,
    /// 内容加载耗时（毫秒）。
    pub load_time_ms: f64,
}

/// 帧级别的调试统计。
#[derive(Debug, Clone, Default)]
pub struct FrameDebugStats {
    /// 帧序号。
    pub frame_number: u64,
    /// 本帧绘制调用次数。
    pub draw_calls: u32,
    /// 本帧渲染的三角形总数。
    pub triangles_rendered: u64,
    /// 本帧渲染的顶点总数。
    pub vertices_rendered: u64,
    /// 已渲染瓦片数。
    pub tiles_rendered: u32,
    /// 已访问瓦片数。
    pub tiles_visited: u32,
    /// 被剔除瓦片数。
    pub tiles_culled: u32,
    /// 正在加载瓦片数。
    pub tiles_loading: u32,
    /// 已加载完成瓦片数。
    pub tiles_loaded: u32,
    /// CPU 帧时（毫秒）。
    pub frame_time_ms: f64,
    /// GPU 帧时（毫秒）。
    pub gpu_time_ms: f64,
    /// 已用显存（字节）。
    pub memory_used_bytes: u64,
    /// 纹理数量。
    pub texture_count: u32,
    /// 着色器程序数量。
    pub shader_count: u32,
    /// GPU 缓冲区数量。
    pub buffer_count: u32,
}

impl DebugInspector {
    /// 创建一个全部开关关闭、采用默认高亮模式的检查器。
    pub fn new() -> Self {
        Self::default()
    }

    /// 启用所有调试可视化。
    pub fn enable_all(&mut self) {
        self.enabled = true;
        self.wireframe = true;
        self.show_bounding_volumes = true;
        self.show_tile_coordinates = true;
        self.show_statistics = true;
        self.show_frustums = true;
    }

    /// 禁用所有调试可视化。
    pub fn disable_all(&mut self) {
        self.enabled = false;
        self.wireframe = false;
        self.show_bounding_volumes = false;
        self.show_tile_coordinates = false;
        self.show_statistics = false;
        self.show_frustums = false;
        self.show_depth = false;
        self.show_normals = false;
        self.show_pick_debug = false;
        self.highlight_mode = HighlightMode::None;
    }

    /// 记录瓦片调试信息。
    pub fn record_tile(&mut self, info: TileDebugInfo) {
        self.tile_debug_info.insert(info.tile_id, info);
    }

    /// 获取特定瓦片的调试信息。
    pub fn get_tile_info(&self, tile_id: u64) -> Option<&TileDebugInfo> {
        self.tile_debug_info.get(&tile_id)
    }

    /// 清除逐瓦片的调试信息（在帧开始时调用）。
    pub fn clear_tile_info(&mut self) {
        self.tile_debug_info.clear();
    }

    /// 更新帧统计。
    pub fn update_frame_stats(&mut self, stats: FrameDebugStats) {
        self.frame_stats = stats;
    }

    /// 获取用于显示的摘要字符串。
    pub fn summary(&self) -> String {
        let s = &self.frame_stats;
        format!(
            "Frame {} | Draw calls: {} | Tris: {} | Tiles: {} rendered / {} visited / {} culled | {:.2} ms",
            s.frame_number,
            s.draw_calls,
            s.triangles_rendered,
            s.tiles_rendered,
            s.tiles_visited,
            s.tiles_culled,
            s.frame_time_ms,
        )
    }
}

/// 用于 HUD 显示的性能覆盖层数据。
#[derive(Debug, Clone, Default)]
pub struct PerformanceOverlay {
    /// 当前瞬时帧率（每秒帧数）。
    pub fps: f64,
    /// 最近一帧 CPU 帧时（毫秒）。
    pub frame_time_ms: f64,
    /// 最近一帧 GPU 帧时（毫秒）。
    pub gpu_frame_time_ms: f64,
    /// 最近一帧绘制调用次数。
    pub draw_calls: u32,
    /// 最近一帧三角形数。
    pub triangles: u64,
    /// 纹理占用内存（MB）。
    pub texture_memory_mb: f64,
    /// 缓冲区占用内存（MB）。
    pub buffer_memory_mb: f64,
    /// 瓦片占用内存（MB）。
    pub tile_memory_mb: f64,
    /// 最近帧时采样历史（最多保留 120 个）。
    pub history: Vec<f64>,
}

impl PerformanceOverlay {
    /// 创建空的性能覆盖层，历史采样列表初始为空。
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录一个帧时间采样。
    pub fn record_frame(&mut self, frame_time_ms: f64) {
        self.frame_time_ms = frame_time_ms;
        self.fps = if frame_time_ms > 0.0 { 1000.0 / frame_time_ms } else { 0.0 };
        self.history.push(frame_time_ms);
        if self.history.len() > 120 {
            self.history.remove(0);
        }
    }

    /// 历史帧时间的平均值。
    pub fn average_frame_time(&self) -> f64 {
        if self.history.is_empty() {
            return 0.0;
        }
        self.history.iter().sum::<f64>() / self.history.len() as f64
    }

    /// 历史中的最低 FPS。
    pub fn min_fps(&self) -> f64 {
        let max_time = self.history.iter().cloned().fold(0.0f64, f64::max);
        if max_time > 0.0 { 1000.0 / max_time } else { 0.0 }
    }
}

/// 用于检视 3D Tiles 内容的瓦片集检查器。
#[derive(Debug, Clone, Default)]
pub struct TilesetInspector {
    /// 检查器是否激活。
    pub active: bool,
    /// 当前选中的瓦片 ID。
    pub selected_tile: Option<u64>,
    /// 显示内容包围体。
    pub show_content_volume: bool,
    /// 显示查看器请求体。
    pub show_viewer_volume: bool,
    /// 按瓦片集着色。
    pub colorize_tileset: bool,
    /// 冻结帧（停止更新）。
    pub freeze_frame: bool,
    /// 最大屏幕空间误差覆盖值。
    pub max_sse_override: Option<f64>,
}

impl TilesetInspector {
    /// 创建未激活、无选中瓦片的瓦片集检查器。
    pub fn new() -> Self {
        Self::default()
    }

    /// 选中一个瓦片进行检查。
    pub fn select_tile(&mut self, tile_id: u64) {
        self.selected_tile = Some(tile_id);
    }

    /// 取消选择当前瓦片。
    pub fn deselect(&mut self) {
        self.selected_tile = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_inspector_toggle() {
        let mut inspector = DebugInspector::new();
        assert!(!inspector.enabled);

        inspector.enable_all();
        assert!(inspector.enabled);
        assert!(inspector.wireframe);
        assert!(inspector.show_bounding_volumes);
        assert!(inspector.show_statistics);

        inspector.disable_all();
        assert!(!inspector.enabled);
        assert!(!inspector.wireframe);
    }

    #[test]
    fn test_tile_debug_info() {
        let mut inspector = DebugInspector::new();
        inspector.record_tile(TileDebugInfo {
            tile_id: 42,
            depth: 3,
            geometric_error: 100.0,
            distance_to_camera: 500.0,
            screen_space_error: 8.5,
            is_rendered: true,
            is_visited: true,
            is_selected: false,
            content_type: "b3dm".to_string(),
            triangles_count: 15000,
            vertices_count: 8000,
            load_time_ms: 12.5,
        });

        let info = inspector.get_tile_info(42).unwrap();
        assert_eq!(info.depth, 3);
        assert_eq!(info.triangles_count, 15000);
        assert!(info.is_rendered);

        inspector.clear_tile_info();
        assert!(inspector.get_tile_info(42).is_none());
    }

    #[test]
    fn test_frame_stats_summary() {
        let mut inspector = DebugInspector::new();
        inspector.update_frame_stats(FrameDebugStats {
            frame_number: 100,
            draw_calls: 256,
            triangles_rendered: 1_500_000,
            tiles_rendered: 128,
            tiles_visited: 200,
            tiles_culled: 72,
            frame_time_ms: 16.67,
            ..Default::default()
        });

        let summary = inspector.summary();
        assert!(summary.contains("Frame 100"));
        assert!(summary.contains("Draw calls: 256"));
        assert!(summary.contains("128 rendered"));
    }

    #[test]
    fn test_performance_overlay() {
        let mut overlay = PerformanceOverlay::new();
        overlay.record_frame(16.67);
        overlay.record_frame(14.0);
        overlay.record_frame(20.0);

        assert!(overlay.fps > 0.0);
        assert_eq!(overlay.history.len(), 3);
        assert!(overlay.average_frame_time() > 15.0);
        assert!(overlay.min_fps() < 60.0);
    }

    #[test]
    fn test_performance_overlay_history_limit() {
        let mut overlay = PerformanceOverlay::new();
        for _ in 0..150 {
            overlay.record_frame(16.0);
        }
        assert_eq!(overlay.history.len(), 120);
    }

    #[test]
    fn test_tileset_inspector() {
        let mut inspector = TilesetInspector::new();
        assert!(!inspector.active);
        assert!(inspector.selected_tile.is_none());

        inspector.select_tile(99);
        assert_eq!(inspector.selected_tile, Some(99));

        inspector.deselect();
        assert!(inspector.selected_tile.is_none());
    }

    #[test]
    fn test_highlight_modes() {
        let mut inspector = DebugInspector::new();
        assert_eq!(inspector.highlight_mode, HighlightMode::None);

        inspector.highlight_mode = HighlightMode::Depth;
        assert_eq!(inspector.highlight_mode, HighlightMode::Depth);

        inspector.highlight_mode = HighlightMode::RandomColor;
        assert_eq!(inspector.highlight_mode, HighlightMode::RandomColor);
    }
}
