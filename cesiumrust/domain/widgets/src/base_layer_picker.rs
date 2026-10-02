//! 基础图层选择器视图模型。
//!
//! 以类别聚合影像/地形提供器选项，维护下拉开关、当前选中项与
//! 提示文本，为图层切换 UI 提供纯领域状态。

/// 一个提供器类别（例如 "Imagery"、"Terrain"）。
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderCategory {
    /// 类别名称。
    pub name: String,
    /// 该类别中的提供器视图模型。
    pub providers: Vec<ProviderViewModel>,
}

impl ProviderCategory {
    /// 创建一个新的提供器类别。
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            providers: Vec::new(),
        }
    }

    /// 向该类别添加一个提供器。
    pub fn add_provider(&mut self, provider: ProviderViewModel) {
        // 按插入顺序追加，索引即为后续选择时使用的 provider_idx
        self.providers.push(provider);
    }

    /// 获取提供器数量。
    pub fn provider_count(&self) -> usize {
        // 直接返回本类别下提供器列表的长度
        self.providers.len()
    }
}

/// 表示单个影像/地形提供器选项的视图模型。
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderViewModel {
    /// 显示名称。
    pub name: String,
    /// 提示文本。
    pub tooltip: String,
    /// 图标 URL 或标识符。
    pub icon_url: String,
    /// 提供器类别名称。
    pub category: String,
    /// 该提供器当前是否选中。
    pub is_selected: bool,
    /// 提供器的创建参数（URL、密钥等）。
    pub creation_parameters: serde_json::Value,
}

impl ProviderViewModel {
    /// 创建一个新的提供器视图模型。
    pub fn new(name: impl Into<String>, category: impl Into<String>) -> Self {
        // 默认提示与名称同名；名称后续被复用，先取出再入结构
        let name = name.into();
        Self {
            tooltip: name.clone(),
            icon_url: String::new(),
            is_selected: false,
            creation_parameters: serde_json::Value::Null,
            name,
            category: category.into(),
        }
    }

    /// 设置提示文本。
    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        // 链式设置悬停提示，返回自身以便连写
        self.tooltip = tooltip.into();
        self
    }

    /// 设置图标 URL。
    pub fn with_icon(mut self, icon_url: impl Into<String>) -> Self {
        self.icon_url = icon_url.into();
        self
    }

    /// 设置创建参数。
    pub fn with_parameters(mut self, params: serde_json::Value) -> Self {
        self.creation_parameters = params;
        self
    }
}

/// 基础图层选择器视图模型。
///
/// 控制影像与地形提供器的选择。
#[derive(Debug, Clone)]
pub struct BaseLayerPickerViewModel {
    /// 选择器下拉菜单是否打开。
    pub is_dropdown_open: bool,
    /// widget 是否可见。
    pub show: bool,
    /// 提供器类别。
    pub categories: Vec<ProviderCategory>,
    /// 选中影像提供器的索引（在其类别内）。
    pub selected_imagery_index: Option<(usize, usize)>,
    /// 选中地形提供器的索引（在其类别内）。
    pub selected_terrain_index: Option<(usize, usize)>,
}

impl Default for BaseLayerPickerViewModel {
    /// 默认下拉关闭、widget 可见、无类别与选中项。
    fn default() -> Self {
        Self {
            is_dropdown_open: false,
            show: true,
            categories: Vec::new(),
            selected_imagery_index: None,
            selected_terrain_index: None,
        }
    }
}

impl BaseLayerPickerViewModel {
    /// 创建一个新的基础图层选择器。
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加一个提供器类别。
    pub fn add_category(&mut self, category: ProviderCategory) {
        self.categories.push(category);
    }

    /// 切换下拉菜单。
    pub fn toggle_dropdown(&mut self) {
        // 就地翻转开合标志，具体互斥交由具体选项选中后的关闭处理
        self.is_dropdown_open = !self.is_dropdown_open;
    }

    /// 关闭下拉菜单。
    pub fn close_dropdown(&mut self) {
        // 无论当前开合，统一置为关闭
        self.is_dropdown_open = false;
    }

    /// 按类别与提供器索引选择一个影像提供器。
    pub fn select_imagery(&mut self, category_idx: usize, provider_idx: usize) {
        // 取消选择上一个
        // 影像选择互斥：先清除旧选中项的高亮标志
        if let Some((ci, pi)) = self.selected_imagery_index {
            if let Some(cat) = self.categories.get_mut(ci) {
                if let Some(prov) = cat.providers.get_mut(pi) {
                    prov.is_selected = false;
                }
            }
        }

        // 选择新的
        if let Some(cat) = self.categories.get_mut(category_idx) {
            if let Some(prov) = cat.providers.get_mut(provider_idx) {
                prov.is_selected = true;
                self.selected_imagery_index = Some((category_idx, provider_idx));
            }
        }

        self.is_dropdown_open = false;
    }

    /// 按类别与提供器索引选择一个地形提供器。
    pub fn select_terrain(&mut self, category_idx: usize, provider_idx: usize) {
        // 取消选择上一个
        // 地形选择互斥：先清除旧选中项的高亮标志
        if let Some((ci, pi)) = self.selected_terrain_index {
            if let Some(cat) = self.categories.get_mut(ci) {
                if let Some(prov) = cat.providers.get_mut(pi) {
                    prov.is_selected = false;
                }
            }
        }

        // 选择新的
        if let Some(cat) = self.categories.get_mut(category_idx) {
            if let Some(prov) = cat.providers.get_mut(provider_idx) {
                prov.is_selected = true;
                self.selected_terrain_index = Some((category_idx, provider_idx));
            }
        }

        self.is_dropdown_open = false;
    }

    /// 获取当前选中的影像提供器。
    pub fn selected_imagery_provider(&self) -> Option<&ProviderViewModel> {
        // 依记录的 (类别, 提供器) 索引逐级取出，任一越界则 None
        let (ci, pi) = self.selected_imagery_index?;
        self.categories.get(ci)?.providers.get(pi)
    }

    /// 获取当前选中的地形提供器。
    pub fn selected_terrain_provider(&self) -> Option<&ProviderViewModel> {
        // 与影像同理，依记录索引逐级取出
        let (ci, pi) = self.selected_terrain_index?;
        self.categories.get(ci)?.providers.get(pi)
    }

    /// 获取所有类别中提供器的总数。
    pub fn total_provider_count(&self) -> usize {
        // 汇总各类别的提供器数量，用于展示选项总规模
        self.categories.iter().map(|c| c.provider_count()).sum()
    }

    /// 获取显示当前选择的按钮提示文本。
    pub fn button_tooltip(&self) -> String {
        // 已选影像时提示当前图层名，否则提示去选择基础图层
        if let Some(prov) = self.selected_imagery_provider() {
            format!("Current imagery: {}", prov.name)
        } else {
            "Select base layer".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_picker() -> BaseLayerPickerViewModel {
        // 构造一个含 Imagery(2 个)/Terrain(1 个) 两类别的测试选择器
        let mut vm = BaseLayerPickerViewModel::new();

        let mut imagery_cat = ProviderCategory::new("Imagery");
        imagery_cat.add_provider(
            ProviderViewModel::new("Bing Maps", "Imagery")
                .with_tooltip("Bing Maps aerial imagery"),
        );
        imagery_cat.add_provider(
            ProviderViewModel::new("OpenStreetMap", "Imagery")
                .with_tooltip("OSM street map"),
        );

        let mut terrain_cat = ProviderCategory::new("Terrain");
        terrain_cat.add_provider(
            ProviderViewModel::new("Cesium World Terrain", "Terrain")
                .with_tooltip("High-res terrain"),
        );

        vm.add_category(imagery_cat);
        vm.add_category(terrain_cat);
        vm
    }

    #[test]
    fn test_default() {
        let vm = BaseLayerPickerViewModel::default();
        assert!(!vm.is_dropdown_open);
        assert!(vm.show);
        assert!(vm.categories.is_empty());
        assert!(vm.selected_imagery_index.is_none());
    }

    #[test]
    fn test_add_categories() {
        let vm = make_test_picker();
        assert_eq!(vm.categories.len(), 2);
        assert_eq!(vm.total_provider_count(), 3);
    }

    #[test]
    fn test_select_imagery() {
        let mut vm = make_test_picker();
        vm.select_imagery(0, 1); // OpenStreetMap
        let selected = vm.selected_imagery_provider().unwrap();
        assert_eq!(selected.name, "OpenStreetMap");
        assert!(selected.is_selected);
        assert!(!vm.is_dropdown_open);
    }

    #[test]
    fn test_select_imagery_deselects_previous() {
        let mut vm = make_test_picker();
        vm.select_imagery(0, 0); // Bing
        vm.select_imagery(0, 1); // OSM
        assert!(!vm.categories[0].providers[0].is_selected);
        assert!(vm.categories[0].providers[1].is_selected);
    }

    #[test]
    fn test_select_terrain() {
        let mut vm = make_test_picker();
        vm.select_terrain(1, 0);
        let selected = vm.selected_terrain_provider().unwrap();
        assert_eq!(selected.name, "Cesium World Terrain");
    }

    #[test]
    fn test_toggle_dropdown() {
        let mut vm = make_test_picker();
        vm.toggle_dropdown();
        assert!(vm.is_dropdown_open);
        vm.toggle_dropdown();
        assert!(!vm.is_dropdown_open);
    }

    #[test]
    fn test_button_tooltip() {
        let mut vm = make_test_picker();
        assert_eq!(vm.button_tooltip(), "Select base layer");
        vm.select_imagery(0, 0);
        assert_eq!(vm.button_tooltip(), "Current imagery: Bing Maps");
    }

    #[test]
    fn test_provider_view_model_builder() {
        let prov = ProviderViewModel::new("Test", "Cat")
            .with_tooltip("A tooltip")
            .with_icon("icon.png")
            .with_parameters(serde_json::json!({"url": "http://example.com"}));
        assert_eq!(prov.name, "Test");
        assert_eq!(prov.tooltip, "A tooltip");
        assert_eq!(prov.icon_url, "icon.png");
        assert_eq!(prov.creation_parameters["url"], "http://example.com");
    }

    #[test]
    fn test_invalid_selection() {
        let mut vm = make_test_picker();
        vm.select_imagery(99, 0); // 无效类别
        // 对于无效索引不应设置选中项
        assert!(vm.selected_imagery_index.is_none());
        assert!(vm.selected_imagery_provider().is_none());
    }
}
