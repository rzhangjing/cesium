//! widget 的国际化（i18n）支持。
//!
//! 为所有 widget UI 文本提供区域感知（locale-aware）的字符串资源。

use std::collections::HashMap;

/// 支持的区域设置。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Locale {
    /// 英语（默认）。
    #[default]
    En,
    /// 简体中文。
    ZhCn,
    /// 日语。
    Ja,
    /// 法语。
    Fr,
    /// 德语。
    De,
    /// 西班牙语。
    Es,
}

impl Locale {
    /// 获取区域代码字符串。
    pub fn code(&self) -> &'static str {
        match self {
            Self::En => "en",
            Self::ZhCn => "zh-CN",
            Self::Ja => "ja",
            Self::Fr => "fr",
            Self::De => "de",
            Self::Es => "es",
        }
    }

    /// 从代码字符串解析区域设置。
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "en" => Some(Self::En),
            "zh-CN" | "zh" => Some(Self::ZhCn),
            "ja" => Some(Self::Ja),
            "fr" => Some(Self::Fr),
            "de" => Some(Self::De),
            "es" => Some(Self::Es),
            _ => None,
        }
    }

    /// 获取所有可用区域设置。
    pub fn all() -> &'static [Locale] {
        &[Self::En, Self::ZhCn, Self::Ja, Self::Fr, Self::De, Self::Es]
    }
}

/// 特定区域设置的 widget UI 字符串。
#[derive(Debug, Clone, PartialEq)]
pub struct WidgetStrings {
    /// 动画 widget 字符串。
    pub animation: AnimationStrings,
    /// 时间轴 widget 字符串。
    pub timeline: TimelineStrings,
    /// 场景模式选择器字符串。
    pub scene_mode_picker: SceneModePickerStrings,
    /// 基础图层选择器字符串。
    pub base_layer_picker: BaseLayerPickerStrings,
    /// 地名搜索字符串。
    pub geocoder: GeocoderStrings,
    /// 导航帮助字符串。
    pub navigation_help: NavigationHelpStrings,
    /// 全屏按钮字符串。
    pub fullscreen: FullscreenStrings,
    /// 信息框字符串。
    pub info_box: InfoBoxStrings,
    /// VR 按钮字符串。
    pub vr_button: VRButtonStrings,
}

/// 动画 widget 字符串。
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationStrings {
    /// 播放按钮提示文本。
    pub play: String,
    /// 暂停按钮提示文本。
    pub pause: String,
    /// 反向播放提示文本。
    pub play_reverse: String,
    /// 正向播放提示文本。
    pub play_forward: String,
    /// 实时按钮提示文本。
    pub realtime: String,
    /// 速度倍率标签。
    pub multiplier_label: String,
}

/// 时间轴 widget 字符串。
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineStrings {
    /// 时间轴提示文本。
    pub tooltip: String,
}

/// 场景模式选择器字符串。
#[derive(Debug, Clone, PartialEq)]
pub struct SceneModePickerStrings {
    /// 3D 模式标签。
    pub scene_3d: String,
    /// 2D 模式标签。
    pub scene_2d: String,
    /// Columbus View 标签。
    pub columbus_view: String,
    /// 提示文本。
    pub tooltip: String,
}

/// 基础图层选择器字符串。
#[derive(Debug, Clone, PartialEq)]
pub struct BaseLayerPickerStrings {
    /// 按钮提示文本。
    pub tooltip: String,
    /// 影像类别标签。
    pub imagery: String,
    /// 地形类别标签。
    pub terrain: String,
}

/// 地名搜索字符串。
#[derive(Debug, Clone, PartialEq)]
pub struct GeocoderStrings {
    /// 占位提示文本。
    pub placeholder: String,
    /// 搜索按钮提示文本。
    pub search: String,
    /// 无结果消息。
    pub no_results: String,
}

/// 导航帮助字符串。
#[derive(Debug, Clone, PartialEq)]
pub struct NavigationHelpStrings {
    /// 按钮提示文本。
    pub tooltip: String,
    /// 鼠标导航标题。
    pub mouse_title: String,
    /// 触摸导航标题。
    pub touch_title: String,
    /// 旋转说明。
    pub rotate: String,
    /// 缩放说明。
    pub zoom: String,
    /// 平移说明。
    pub pan: String,
    /// 俯仰说明。
    pub tilt: String,
}

/// 全屏按钮字符串。
#[derive(Debug, Clone, PartialEq)]
pub struct FullscreenStrings {
    /// 进入全屏提示文本。
    pub enter: String,
    /// 退出全屏提示文本。
    pub exit: String,
}

/// 信息框字符串。
#[derive(Debug, Clone, PartialEq)]
pub struct InfoBoxStrings {
    /// 面板标题。
    pub title: String,
    /// 关闭按钮提示文本。
    pub close: String,
    /// 无选中项消息。
    pub no_selection: String,
}

/// VR 按钮字符串。
#[derive(Debug, Clone, PartialEq)]
pub struct VRButtonStrings {
    /// 进入 VR 提示文本。
    pub enter: String,
    /// 退出 VR 提示文本。
    pub exit: String,
}

impl Default for WidgetStrings {
    fn default() -> Self {
        Self::english()
    }
}

impl WidgetStrings {
    /// 获取英语字符串。
    pub fn english() -> Self {
        Self {
            animation: AnimationStrings {
                play: "Play".to_string(),
                pause: "Pause".to_string(),
                play_reverse: "Play Reverse".to_string(),
                play_forward: "Play Forward".to_string(),
                realtime: "Today (real-time)".to_string(),
                multiplier_label: "Speed".to_string(),
            },
            timeline: TimelineStrings {
                tooltip: "Timeline".to_string(),
            },
            scene_mode_picker: SceneModePickerStrings {
                scene_3d: "3D".to_string(),
                scene_2d: "2D".to_string(),
                columbus_view: "Columbus View".to_string(),
                tooltip: "Change scene mode".to_string(),
            },
            base_layer_picker: BaseLayerPickerStrings {
                tooltip: "Imagery / Terrain".to_string(),
                imagery: "Imagery".to_string(),
                terrain: "Terrain".to_string(),
            },
            geocoder: GeocoderStrings {
                placeholder: "Enter an address or landmark...".to_string(),
                search: "Search".to_string(),
                no_results: "No results found".to_string(),
            },
            navigation_help: NavigationHelpStrings {
                tooltip: "Navigation Instructions".to_string(),
                mouse_title: "Mouse Navigation".to_string(),
                touch_title: "Touch Navigation".to_string(),
                rotate: "Left click + drag".to_string(),
                zoom: "Right click + drag, or scroll".to_string(),
                pan: "Ctrl + left click + drag".to_string(),
                tilt: "Ctrl + middle click + drag".to_string(),
            },
            fullscreen: FullscreenStrings {
                enter: "Full screen".to_string(),
                exit: "Exit full screen".to_string(),
            },
            info_box: InfoBoxStrings {
                title: "Entity Information".to_string(),
                close: "Close".to_string(),
                no_selection: "No entity selected".to_string(),
            },
            vr_button: VRButtonStrings {
                enter: "Enter VR".to_string(),
                exit: "Exit VR".to_string(),
            },
        }
    }

    /// 获取简体中文字符串。
    pub fn chinese() -> Self {
        Self {
            animation: AnimationStrings {
                play: "播放".to_string(),
                pause: "暂停".to_string(),
                play_reverse: "反向播放".to_string(),
                play_forward: "正向播放".to_string(),
                realtime: "今天（实时）".to_string(),
                multiplier_label: "速度".to_string(),
            },
            timeline: TimelineStrings {
                tooltip: "时间线".to_string(),
            },
            scene_mode_picker: SceneModePickerStrings {
                scene_3d: "三维".to_string(),
                scene_2d: "二维".to_string(),
                columbus_view: "哥伦布视图".to_string(),
                tooltip: "切换场景模式".to_string(),
            },
            base_layer_picker: BaseLayerPickerStrings {
                tooltip: "影像 / 地形".to_string(),
                imagery: "影像".to_string(),
                terrain: "地形".to_string(),
            },
            geocoder: GeocoderStrings {
                placeholder: "输入地址或地标...".to_string(),
                search: "搜索".to_string(),
                no_results: "未找到结果".to_string(),
            },
            navigation_help: NavigationHelpStrings {
                tooltip: "导航说明".to_string(),
                mouse_title: "鼠标导航".to_string(),
                touch_title: "触摸导航".to_string(),
                rotate: "左键拖拽".to_string(),
                zoom: "右键拖拽或滚轮".to_string(),
                pan: "Ctrl + 左键拖拽".to_string(),
                tilt: "Ctrl + 中键拖拽".to_string(),
            },
            fullscreen: FullscreenStrings {
                enter: "全屏".to_string(),
                exit: "退出全屏".to_string(),
            },
            info_box: InfoBoxStrings {
                title: "实体信息".to_string(),
                close: "关闭".to_string(),
                no_selection: "未选择实体".to_string(),
            },
            vr_button: VRButtonStrings {
                enter: "进入VR".to_string(),
                exit: "退出VR".to_string(),
            },
        }
    }

    /// 获取日语字符串。
    pub fn japanese() -> Self {
        Self {
            animation: AnimationStrings {
                play: "再生".to_string(),
                pause: "一時停止".to_string(),
                play_reverse: "逆再生".to_string(),
                play_forward: "早送り".to_string(),
                realtime: "今日（リアルタイム）".to_string(),
                multiplier_label: "速度".to_string(),
            },
            timeline: TimelineStrings {
                tooltip: "タイムライン".to_string(),
            },
            scene_mode_picker: SceneModePickerStrings {
                scene_3d: "3D".to_string(),
                scene_2d: "2D".to_string(),
                columbus_view: "コロンバスビュー".to_string(),
                tooltip: "シーンモード切替".to_string(),
            },
            base_layer_picker: BaseLayerPickerStrings {
                tooltip: "画像 / 地形".to_string(),
                imagery: "画像".to_string(),
                terrain: "地形".to_string(),
            },
            geocoder: GeocoderStrings {
                placeholder: "住所やランドマークを入力...".to_string(),
                search: "検索".to_string(),
                no_results: "結果が見つかりません".to_string(),
            },
            navigation_help: NavigationHelpStrings {
                tooltip: "操作方法".to_string(),
                mouse_title: "マウス操作".to_string(),
                touch_title: "タッチ操作".to_string(),
                rotate: "左クリック＋ドラッグ".to_string(),
                zoom: "右クリック＋ドラッグ、またはスクロール".to_string(),
                pan: "Ctrl＋左クリック＋ドラッグ".to_string(),
                tilt: "Ctrl＋中クリック＋ドラッグ".to_string(),
            },
            fullscreen: FullscreenStrings {
                enter: "フルスクリーン".to_string(),
                exit: "フルスクリーン解除".to_string(),
            },
            info_box: InfoBoxStrings {
                title: "エンティティ情報".to_string(),
                close: "閉じる".to_string(),
                no_selection: "エンティティが選択されていません".to_string(),
            },
            vr_button: VRButtonStrings {
                enter: "VR開始".to_string(),
                exit: "VR終了".to_string(),
            },
        }
    }
}

/// 国际化管理器。
///
/// 管理所有 widget 的区域特定字符串。
#[derive(Debug, Clone)]
pub struct I18n {
    /// 当前区域设置。
    pub current_locale: Locale,
    /// 以区域设置键组织的可用字符串资源。
    pub resources: HashMap<Locale, WidgetStrings>,
}

impl Default for I18n {
    fn default() -> Self {
        let mut resources = HashMap::new();
        resources.insert(Locale::En, WidgetStrings::english());
        resources.insert(Locale::ZhCn, WidgetStrings::chinese());
        resources.insert(Locale::Ja, WidgetStrings::japanese());
        Self {
            current_locale: Locale::En,
            resources,
        }
    }
}

impl I18n {
    /// 创建一个新的、带默认区域设置的 i18n 管理器。
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置当前区域设置。
    pub fn set_locale(&mut self, locale: Locale) {
        self.current_locale = locale;
    }

    /// 获取当前区域设置的字符串。
    pub fn strings(&self) -> &WidgetStrings {
        self.resources
            .get(&self.current_locale)
            .unwrap_or_else(|| self.resources.get(&Locale::En).unwrap())
    }

    /// 获取特定区域设置的字符串（回退到英语）。
    pub fn strings_for(&self, locale: Locale) -> &WidgetStrings {
        self.resources
            .get(&locale)
            .unwrap_or_else(|| self.resources.get(&Locale::En).unwrap())
    }

    /// 为某个区域设置注册字符串。
    pub fn register_locale(&mut self, locale: Locale, strings: WidgetStrings) {
        self.resources.insert(locale, strings);
    }

    /// 获取可用的区域设置。
    pub fn available_locales(&self) -> Vec<Locale> {
        let mut locales: Vec<Locale> = self.resources.keys().copied().collect();
        locales.sort_by_key(|l| l.code());
        locales
    }

    /// 按键路径获取已翻译的字符串（例如 "animation.play"）。
    pub fn get(&self, key: &str) -> Option<&str> {
        let strings = self.strings();
        let parts: Vec<&str> = key.splitn(2, '.').collect();
        if parts.len() != 2 {
            return None;
        }
        match parts[0] {
            "animation" => match parts[1] {
                "play" => Some(&strings.animation.play),
                "pause" => Some(&strings.animation.pause),
                "play_reverse" => Some(&strings.animation.play_reverse),
                "play_forward" => Some(&strings.animation.play_forward),
                "realtime" => Some(&strings.animation.realtime),
                "multiplier_label" => Some(&strings.animation.multiplier_label),
                _ => None,
            },
            "geocoder" => match parts[1] {
                "placeholder" => Some(&strings.geocoder.placeholder),
                "search" => Some(&strings.geocoder.search),
                "no_results" => Some(&strings.geocoder.no_results),
                _ => None,
            },
            "fullscreen" => match parts[1] {
                "enter" => Some(&strings.fullscreen.enter),
                "exit" => Some(&strings.fullscreen.exit),
                _ => None,
            },
            "info_box" => match parts[1] {
                "title" => Some(&strings.info_box.title),
                "close" => Some(&strings.info_box.close),
                "no_selection" => Some(&strings.info_box.no_selection),
                _ => None,
            },
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locale_codes() {
        assert_eq!(Locale::En.code(), "en");
        assert_eq!(Locale::ZhCn.code(), "zh-CN");
        assert_eq!(Locale::Ja.code(), "ja");
    }

    #[test]
    fn test_locale_from_code() {
        assert_eq!(Locale::from_code("en"), Some(Locale::En));
        assert_eq!(Locale::from_code("zh-CN"), Some(Locale::ZhCn));
        assert_eq!(Locale::from_code("zh"), Some(Locale::ZhCn));
        assert_eq!(Locale::from_code("xx"), None);
    }

    #[test]
    fn test_locale_all() {
        let all = Locale::all();
        assert_eq!(all.len(), 6);
    }

    #[test]
    fn test_english_strings() {
        let strings = WidgetStrings::english();
        assert_eq!(strings.animation.play, "Play");
        assert_eq!(strings.geocoder.placeholder, "Enter an address or landmark...");
    }

    #[test]
    fn test_chinese_strings() {
        let strings = WidgetStrings::chinese();
        assert_eq!(strings.animation.play, "播放");
        assert_eq!(strings.scene_mode_picker.scene_3d, "三维");
        assert_eq!(strings.info_box.close, "关闭");
    }

    #[test]
    fn test_japanese_strings() {
        let strings = WidgetStrings::japanese();
        assert_eq!(strings.animation.play, "再生");
        assert_eq!(strings.geocoder.search, "検索");
    }

    #[test]
    fn test_i18n_default() {
        let i18n = I18n::default();
        assert_eq!(i18n.current_locale, Locale::En);
        assert_eq!(i18n.strings().animation.play, "Play");
    }

    #[test]
    fn test_i18n_set_locale() {
        let mut i18n = I18n::new();
        i18n.set_locale(Locale::ZhCn);
        assert_eq!(i18n.strings().animation.play, "播放");
    }

    #[test]
    fn test_i18n_fallback() {
        let mut i18n = I18n::new();
        i18n.set_locale(Locale::Fr); // 未注册
        // 应回退到英语
        assert_eq!(i18n.strings().animation.play, "Play");
    }

    #[test]
    fn test_i18n_get_key() {
        let mut i18n = I18n::new();
        assert_eq!(i18n.get("animation.play"), Some("Play"));
        assert_eq!(i18n.get("geocoder.search"), Some("Search"));
        assert_eq!(i18n.get("invalid.key"), None);
        assert_eq!(i18n.get("animation.invalid"), None);

        i18n.set_locale(Locale::ZhCn);
        assert_eq!(i18n.get("animation.play"), Some("播放"));
    }

    #[test]
    fn test_i18n_available_locales() {
        let i18n = I18n::new();
        let locales = i18n.available_locales();
        assert_eq!(locales.len(), 3); // en、ja、zh-CN
    }

    #[test]
    fn test_i18n_register_locale() {
        let mut i18n = I18n::new();
        let french = WidgetStrings {
            animation: AnimationStrings {
                play: "Jouer".to_string(),
                pause: "Pause".to_string(),
                play_reverse: "Lecture inverse".to_string(),
                play_forward: "Avance rapide".to_string(),
                realtime: "Aujourd'hui (temps réel)".to_string(),
                multiplier_label: "Vitesse".to_string(),
            },
            ..WidgetStrings::english()
        };
        i18n.register_locale(Locale::Fr, french);
        i18n.set_locale(Locale::Fr);
        assert_eq!(i18n.strings().animation.play, "Jouer");
    }
}
