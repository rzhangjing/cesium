//! 信息框（info box）widget 视图模型。
//!
//! 在面板中展示选中实体的标题与描述，支持展开/收起、跟踪模式
//! 与相机视角偏移，以及长描述的摘要截断。

/// 信息框 widget 视图模型。
///
/// 在一个面板中显示关于选中实体的信息。
#[derive(Debug, Clone)]
pub struct InfoBoxViewModel {
    /// 信息框是否可见。
    pub show: bool,
    /// 是否显示信息框边框/面板（展开）。
    pub is_frame_visible: bool,
    /// 标题文本（通常为实体名称）。
    pub title: String,
    /// 描述内容（HTML 或纯文本）。
    pub description: String,
    /// 关闭按钮是否可见。
    pub show_close: bool,
    /// 信息框是否有内容可显示。
    pub has_content: bool,
    /// 跟踪实体时的相机视角偏移。
    pub camera_view_offset: Option<[f64; 3]>,
    /// 信息框是否处于“跟踪”模式。
    pub is_tracking: bool,
}

impl Default for InfoBoxViewModel {
    /// 默认 widget 可见但面板收起、无内容、关闭按钮可见、非跟踪。
    fn default() -> Self {
        Self {
            show: true,
            is_frame_visible: false,
            title: String::new(),
            description: String::new(),
            show_close: true,
            has_content: false,
            camera_view_offset: None,
            is_tracking: false,
        }
    }
}

impl InfoBoxViewModel {
    /// 创建一个新的信息框视图模型。
    pub fn new() -> Self {
        Self::default()
    }

    /// 显示实体信息。
    pub fn show_entity(&mut self, title: impl Into<String>, description: impl Into<String>) {
        // 写入标题与描述并标记有内容，同时自动展开面板
        self.title = title.into();
        self.description = description.into();
        self.has_content = true;
        self.is_frame_visible = true;
    }

    /// 清除信息框内容。
    pub fn clear(&mut self) {
        // 回到完全空态：清文本、收起面板、退出跟踪并丢弃相机偏移
        self.title.clear();
        self.description.clear();
        self.has_content = false;
        self.is_frame_visible = false;
        self.is_tracking = false;
        self.camera_view_offset = None;
    }

    /// 关闭信息框面板（隐藏边框但保留 widget）。
    pub fn close(&mut self) {
        // 仅收起面板，保留已加载的标题与描述内容
        self.is_frame_visible = false;
    }

    /// 切换边框可见性。
    pub fn toggle_frame(&mut self) {
        // 仅在有内容时才允许展开/收起，空面板不会因切换而弹出
        if self.has_content {
            self.is_frame_visible = !self.is_frame_visible;
        }
    }

    /// 设置跟踪模式。
    pub fn set_tracking(&mut self, tracking: bool) {
        // 开关相机跟随选中实体的跟踪模式
        self.is_tracking = tracking;
    }

    /// 设置用于跟踪的相机视角偏移。
    pub fn set_camera_offset(&mut self, offset: [f64; 3]) {
        // 记录相机相对实体的偏移，供跟踪时保持预设视角
        self.camera_view_offset = Some(offset);
    }

    /// 获取信息框的摘要行。
    pub fn summary(&self) -> String {
        // 无内容则空串；否则超 100 字符时截断为前 97 字符加省略号
        if !self.has_content {
            return String::new();
        }
        if self.description.len() > 100 {
            format!("{}...", &self.description[..97])
        } else {
            self.description.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let vm = InfoBoxViewModel::default();
        // 默认可见但面板收起、无内容、标题为空
        assert!(vm.show);
        assert!(!vm.is_frame_visible);
        assert!(!vm.has_content);
        assert!(vm.title.is_empty());
    }

    #[test]
    fn test_show_entity() {
        let mut vm = InfoBoxViewModel::new();
        // 展示实体应同时写入标题/描述并展开面板
        vm.show_entity("Test Entity", "A description");
        assert_eq!(vm.title, "Test Entity");
        assert_eq!(vm.description, "A description");
        assert!(vm.has_content);
        assert!(vm.is_frame_visible);
    }

    #[test]
    fn test_clear() {
        let mut vm = InfoBoxViewModel::new();
        vm.show_entity("Test", "Desc");
        vm.set_tracking(true);
        vm.set_camera_offset([1.0, 2.0, 3.0]);
        vm.clear();

        assert!(vm.title.is_empty());
        assert!(vm.description.is_empty());
        assert!(!vm.has_content);
        assert!(!vm.is_frame_visible);
        assert!(!vm.is_tracking);
        assert!(vm.camera_view_offset.is_none());
    }

    #[test]
    fn test_close() {
        let mut vm = InfoBoxViewModel::new();
        // 先展示实体使面板展开
        vm.show_entity("Test", "Desc");
        assert!(vm.is_frame_visible);
        vm.close();
        // close 仅收起面板，不清空内容
        assert!(!vm.is_frame_visible);
        assert!(vm.has_content); // 内容被保留
    }

    #[test]
    fn test_toggle_frame() {
        let mut vm = InfoBoxViewModel::new();
        // 无内容 - 切换不应生效
        // 未展示实体时 toggle 保持收起，验证内容约束
        vm.toggle_frame();
        assert!(!vm.is_frame_visible);

        vm.show_entity("Test", "Desc");
        vm.toggle_frame();
        assert!(!vm.is_frame_visible);
        vm.toggle_frame();
        assert!(vm.is_frame_visible);
    }

    #[test]
    fn test_summary_short() {
        let mut vm = InfoBoxViewModel::new();
        vm.show_entity("Test", "Short desc");
        assert_eq!(vm.summary(), "Short desc");
    }

    #[test]
    fn test_summary_long() {
        let mut vm = InfoBoxViewModel::new();
        // 构造 200 字符长描述，预期被截断为 100 字符（含省略号）
        let long_desc = "A".repeat(200);
        vm.show_entity("Test", long_desc);
        let summary = vm.summary();
        assert_eq!(summary.len(), 100); // 97 个字符 + "..."
        assert!(summary.ends_with("..."));
    }

    #[test]
    fn test_summary_no_content() {
        let vm = InfoBoxViewModel::new();
        assert!(vm.summary().is_empty());
    }

    #[test]
    fn test_tracking() {
        let mut vm = InfoBoxViewModel::new();
        vm.set_tracking(true);
        assert!(vm.is_tracking);
        vm.set_camera_offset([100.0, 200.0, 300.0]);
        assert_eq!(vm.camera_view_offset, Some([100.0, 200.0, 300.0]));
    }
}
