//! 地名搜索（geocoder）widget 视图模型。
//!
//! 提供即输即搜的地名编码交互模型：输入文本、自动补全、
//! 结果高亮与上下选择，以及选中后的飞至目标位置。

/// 用于显示的地名搜索结果。
#[derive(Debug, Clone, PartialEq)]
pub struct GeocoderSearchResult {
    /// 结果的显示名称。
    pub display_name: String,
    /// 目标位置描述（矩形或点）。
    pub destination: GeocoderSearchDestination,
}

/// 地名搜索结果的目标位置。
#[derive(Debug, Clone, PartialEq)]
pub enum GeocoderSearchDestination {
    /// 以弧度表示的矩形 [west, south, east, north]。
    Rectangle([f64; 4]),
    /// 一个带经度、纬度与可选高度的点。
    Point {
        /// 经度（弧度）。
        longitude: f64,
        /// 纬度（弧度）。
        latitude: f64,
        /// 高度（米）。
        height: Option<f64>,
    },
}

/// 地名搜索 widget 视图模型。
///
/// 提供即输即搜的地名编码功能。
#[derive(Debug, Clone)]
pub struct GeocoderViewModel {
    /// 当前搜索文本。
    pub search_text: String,
    /// 是否正在进行搜索。
    pub is_searching: bool,
    /// 搜索结果。
    pub results: Vec<GeocoderSearchResult>,
    /// 结果面板是否可见。
    pub show_results: bool,
    /// 当前高亮结果的索引。
    pub selected_index: Option<usize>,
    /// widget 是否可见。
    pub show: bool,
    /// 是否启用自动补全。
    pub auto_complete: bool,
    /// 触发搜索前的最少字符数。
    pub min_chars: usize,
    /// 飞至目标位置的时长（秒）。
    pub flight_duration: f64,
    /// 输入框的占位提示文本。
    pub placeholder: String,
}

impl Default for GeocoderViewModel {
    /// 默认空文本、未搜索、widget 可见、自动补全开启、最少 3 字符触发。
    fn default() -> Self {
        Self {
            search_text: String::new(),
            is_searching: false,
            results: Vec::new(),
            show_results: false,
            selected_index: None,
            show: true,
            auto_complete: true,
            min_chars: 3,
            flight_duration: 1.5,
            placeholder: "Enter an address or landmark...".to_string(),
        }
    }
}

impl GeocoderViewModel {
    /// 创建一个新的地名搜索视图模型。
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置搜索文本。
    pub fn set_search_text(&mut self, text: impl Into<String>) {
        self.search_text = text.into();
        // 文本变化后重置高亮，避免旧索引与新结果错位
        self.selected_index = None;
        // 不足最小字符数时清空并收起结果面板
        if self.search_text.len() < self.min_chars {
            self.results.clear();
            self.show_results = false;
        }
    }

    /// 检查搜索文本是否足够长以触发搜索。
    pub fn should_search(&self) -> bool {
        // 需同时满足：文本达最小长度且尚未处于搜索中
        self.search_text.len() >= self.min_chars && !self.is_searching
    }

    /// 开始一次搜索操作。
    pub fn begin_search(&mut self) {
        // 仅当满足触发条件才进入搜索态，防止重复发起
        if self.should_search() {
            self.is_searching = true;
        }
    }

    /// 以结果完成一次搜索。
    pub fn complete_search(&mut self, results: Vec<GeocoderSearchResult>) {
        // 结束搜索态并接管新结果；有结果则默认高亮首项
        self.is_searching = false;
        self.results = results;
        self.show_results = !self.results.is_empty();
        self.selected_index = if self.results.is_empty() { None } else { Some(0) };
    }

    /// 清除搜索。
    pub fn clear_search(&mut self) {
        // 一次性回到初始空态：清文本、清结果、收起面板、退出搜索
        self.search_text.clear();
        self.results.clear();
        self.show_results = false;
        self.selected_index = None;
        self.is_searching = false;
    }

    /// 向上移动选中项。
    pub fn select_previous(&mut self) {
        // 空结果直接返回；否则向前循环选择，到顶时回绕到末项
        if self.results.is_empty() {
            return;
        }
        self.selected_index = Some(match self.selected_index {
            Some(0) => self.results.len() - 1,
            Some(i) => i - 1,
            None => 0,
        });
    }

    /// 向下移动选中项。
    pub fn select_next(&mut self) {
        // 空结果直接返回；否则向后循环选择，到尾时回绕到首项
        if self.results.is_empty() {
            return;
        }
        self.selected_index = Some(match self.selected_index {
            Some(i) if i >= self.results.len() - 1 => 0,
            Some(i) => i + 1,
            None => 0,
        });
    }

    /// 获取当前选中的结果。
    pub fn selected_result(&self) -> Option<&GeocoderSearchResult> {
        // 无高亮索引时 short-circuit 返回 None，否则按索引取项
        self.results.get(self.selected_index?)
    }

    /// 激活选中的结果（飞至目标位置）。
    pub fn activate_selected(&mut self) -> Option<GeocoderSearchResult> {
        // 无选中项则不激活；否则把显示名回填输入框并收起面板，交由上层发起飞行
        let result = self.selected_result()?.clone();
        self.search_text = result.display_name.clone();
        self.show_results = false;
        Some(result)
    }

    /// 隐藏结果面板。
    pub fn hide_results(&mut self) {
        self.show_results = false;
    }

    /// 显示结果面板。
    pub fn show_results_panel(&mut self) {
        // 仅在有结果时才展开面板，避免弹出空白列表
        if !self.results.is_empty() {
            self.show_results = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::approx_constant)] // 地理坐标（新奥尔良 -90°/30°），非数学常量；见 docs/deferred.md #2
    fn sample_results() -> Vec<GeocoderSearchResult> {
        vec![
            GeocoderSearchResult {
                display_name: "New York, NY".to_string(),
                destination: GeocoderSearchDestination::Point {
                    longitude: -1.2921,
                    latitude: 0.7106,
                    height: None,
                },
            },
            GeocoderSearchResult {
                display_name: "New Orleans, LA".to_string(),
                destination: GeocoderSearchDestination::Point {
                    longitude: -1.5708,
                    latitude: 0.5236,
                    height: None,
                },
            },
            GeocoderSearchResult {
                display_name: "United States".to_string(),
                destination: GeocoderSearchDestination::Rectangle([
                    -2.2092, 0.3194, -1.1636, 0.8538,
                ]),
            },
        ]
    }

    #[test]
    fn test_default() {
        let vm = GeocoderViewModel::default();
        assert!(vm.search_text.is_empty());
        assert!(!vm.is_searching);
        assert!(vm.results.is_empty());
        assert!(vm.auto_complete);
        assert_eq!(vm.min_chars, 3);
    }

    #[test]
    fn test_set_search_text() {
        let mut vm = GeocoderViewModel::new();
        vm.set_search_text("New");
        assert_eq!(vm.search_text, "New");
        assert!(vm.should_search());
    }

    #[test]
    fn test_min_chars() {
        let mut vm = GeocoderViewModel::new();
        // 不足 3 字符不应触发，达到 3 字符则可搜索
        vm.set_search_text("Ne");
        assert!(!vm.should_search());
        vm.set_search_text("New");
        assert!(vm.should_search());
    }

    #[test]
    fn test_search_flow() {
        let mut vm = GeocoderViewModel::new();
        // 达到最小字符后 begin_search 进入搜索态
        vm.set_search_text("New York");
        vm.begin_search();
        assert!(vm.is_searching);

        // 完成搜索后接管结果并高亮首项、展开面板
        vm.complete_search(sample_results());
        assert!(!vm.is_searching);
        assert_eq!(vm.results.len(), 3);
        assert!(vm.show_results);
        assert_eq!(vm.selected_index, Some(0));
    }

    #[test]
    fn test_navigation() {
        let mut vm = GeocoderViewModel::new();
        // 先走一次完整搜索拿到 3 条结果，默认高亮首项
        vm.set_search_text("New");
        vm.begin_search();
        vm.complete_search(sample_results());

        // 向下逐步移动，到尾后回绕到首项
        assert_eq!(vm.selected_index, Some(0));
        vm.select_next();
        assert_eq!(vm.selected_index, Some(1));
        vm.select_next();
        assert_eq!(vm.selected_index, Some(2));
        vm.select_next();
        assert_eq!(vm.selected_index, Some(0)); // 回绕

        vm.select_previous();
        assert_eq!(vm.selected_index, Some(2)); // 回退
    }

    #[test]
    fn test_activate_selected() {
        let mut vm = GeocoderViewModel::new();
        vm.set_search_text("New");
        vm.begin_search();
        vm.complete_search(sample_results());

        // 激活首项后应把显示名回填文本框并收起面板
        let result = vm.activate_selected().unwrap();
        assert_eq!(result.display_name, "New York, NY");
        assert_eq!(vm.search_text, "New York, NY");
        assert!(!vm.show_results);
    }

    #[test]
    fn test_clear_search() {
        let mut vm = GeocoderViewModel::new();
        vm.set_search_text("New");
        vm.begin_search();
        vm.complete_search(sample_results());
        vm.clear_search();

        assert!(vm.search_text.is_empty());
        assert!(vm.results.is_empty());
        assert!(!vm.show_results);
        assert!(vm.selected_index.is_none());
    }

    #[test]
    fn test_empty_results() {
        let mut vm = GeocoderViewModel::new();
        vm.set_search_text("xyzzy");
        vm.begin_search();
        vm.complete_search(vec![]);

        assert!(!vm.show_results);
        assert!(vm.selected_index.is_none());
        assert!(vm.selected_result().is_none());
    }
}
