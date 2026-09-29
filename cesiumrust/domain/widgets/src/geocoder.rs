//! 地名搜索（geocoder）widget 视图模型。
//!
//! 映射到 CesiumJS `Geocoder/GeocoderViewModel.js`。

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
        self.selected_index = None;
        if self.search_text.len() < self.min_chars {
            self.results.clear();
            self.show_results = false;
        }
    }

    /// 检查搜索文本是否足够长以触发搜索。
    pub fn should_search(&self) -> bool {
        self.search_text.len() >= self.min_chars && !self.is_searching
    }

    /// 开始一次搜索操作。
    pub fn begin_search(&mut self) {
        if self.should_search() {
            self.is_searching = true;
        }
    }

    /// 以结果完成一次搜索。
    pub fn complete_search(&mut self, results: Vec<GeocoderSearchResult>) {
        self.is_searching = false;
        self.results = results;
        self.show_results = !self.results.is_empty();
        self.selected_index = if self.results.is_empty() { None } else { Some(0) };
    }

    /// 清除搜索。
    pub fn clear_search(&mut self) {
        self.search_text.clear();
        self.results.clear();
        self.show_results = false;
        self.selected_index = None;
        self.is_searching = false;
    }

    /// 向上移动选中项。
    pub fn select_previous(&mut self) {
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
        self.results.get(self.selected_index?)
    }

    /// 激活选中的结果（飞至目标位置）。
    pub fn activate_selected(&mut self) -> Option<GeocoderSearchResult> {
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
        vm.set_search_text("Ne");
        assert!(!vm.should_search());
        vm.set_search_text("New");
        assert!(vm.should_search());
    }

    #[test]
    fn test_search_flow() {
        let mut vm = GeocoderViewModel::new();
        vm.set_search_text("New York");
        vm.begin_search();
        assert!(vm.is_searching);

        vm.complete_search(sample_results());
        assert!(!vm.is_searching);
        assert_eq!(vm.results.len(), 3);
        assert!(vm.show_results);
        assert_eq!(vm.selected_index, Some(0));
    }

    #[test]
    fn test_navigation() {
        let mut vm = GeocoderViewModel::new();
        vm.set_search_text("New");
        vm.begin_search();
        vm.complete_search(sample_results());

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
