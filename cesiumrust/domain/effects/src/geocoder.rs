//! 地理编码服务接口与类型。
//!
//! 定义搜索/逆地理编码两类查询的结果模型（矩形或点目标）与归属鸣谢，
//! 以及供上层注入的地理编码服务 trait 和一个用于测试的模拟实现。

use serde::{Deserialize, Serialize};

/// 要执行的地理编码类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GeocodeType {
    /// 按名称/地址搜索位置。
    #[default]
    Search,
    /// 对位置进行逆向地理编码以获取名称/地址。
    Reverse,
}

/// 地理编码操作的结果。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeocoderResult {
    /// 位置的显示名称。
    pub display_name: String,
    /// 目标，可为边界矩形 [west, south, east, north]（弧度），
    /// 或一个点 [lon, lat, height]（弧度/米）。
    pub destination: GeocoderDestination,
    /// 结果的归属鸣谢。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attributions: Vec<GeocoderAttribution>,
}

/// 地理编码结果的目标——矩形或点二选一。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GeocoderDestination {
    /// 边界矩形 [west, south, east, north]（弧度）。
    Rectangle([f64; 4]),
    /// 一个点 [经度, 纬度]（弧度），带可选高度。
    Point {
        /// 经度（弧度）。
        longitude: f64,
        /// 纬度（弧度）。
        latitude: f64,
        /// 高度（米，可选）。
        #[serde(default)]
        height: Option<f64>,
    },
}

/// 来自地理编码结果的归属鸣谢。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeocoderAttribution {
    /// 该归属是否必须展示。
    #[serde(default)]
    pub mandatory: bool,
    /// 该归属是否可折叠。
    #[serde(default)]
    pub collapsible: bool,
    /// 归属的 HTML 内容。
    pub html: String,
}

/// 地理编码服务的 trait。
pub trait GeocoderService {
    /// 获取执行一次地理编码后要显示的 credit。
    fn credit(&self) -> Option<&str>;

    /// 执行一次地理编码操作。
    ///
    /// 返回与查询匹配的结果列表。
    fn geocode(&self, query: &str, geocode_type: GeocodeType) -> Vec<GeocoderResult>;
}

/// 用于测试的模拟地理编码服务。
#[derive(Debug, Clone, Default)]
pub struct MockGeocoderService {
    /// 对任意查询都返回的结果。
    pub results: Vec<GeocoderResult>,
    /// credit 字符串。
    pub credit: Option<String>,
}

impl MockGeocoderService {
    /// 创建一个新的模拟地理编码器。
    pub fn new() -> Self {
        // 空结果、无 credit 的中性实例
        Self::default()
    }

    /// 使用预定义结果创建。
    pub fn with_results(results: Vec<GeocoderResult>) -> Self {
        // 固定返回集，credit 置空
        Self {
            results,
            credit: None,
        }
    }
}

impl GeocoderService for MockGeocoderService {
    /// 返回预设的 credit（若有）。
    fn credit(&self) -> Option<&str> {
        // 模拟服务直接返回构置时存入的可选 credit 字符串
        self.credit.as_deref()
    }

    /// 执行一次模拟地理编码。
    fn geocode(&self, _query: &str, _geocode_type: GeocodeType) -> Vec<GeocoderResult> {
        // 忽略查询内容，原样返回预先配置的结果列表
        self.results.clone()
    }
}

/// 从地理编码结果的归属中解析 credit。
pub fn get_credits_from_result(result: &GeocoderResult) -> Vec<&GeocoderAttribution> {
    // 把结果携带的归属列表原样暴露为引用列表供上层展示
    result.attributions.iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geocode_type_default() {
        // 默认地理编码类型为正向搜索
        assert_eq!(GeocodeType::default(), GeocodeType::Search);
    }

    #[test]
    fn test_geocoder_result_rectangle() {
        // 构造一个纽约边界矩形结果，验证西<东、南<北
        let result = GeocoderResult {
            display_name: "New York, NY".to_string(),
            destination: GeocoderDestination::Rectangle([
                -1.2985, 0.7086, -1.2968, 0.7098,
            ]),
            attributions: vec![],
        };
        assert_eq!(result.display_name, "New York, NY");
        if let GeocoderDestination::Rectangle(r) = result.destination {
            assert!(r[0] < r[2]); // 西 < 东
            assert!(r[1] < r[3]); // 南 < 北
        } else {
            panic!("Expected Rectangle");
        }
    }

    #[test]
    fn test_geocoder_result_point() {
        // 构造一个带高度的点目标（埃菲尔铁塔），验证经纬度与高度
        let result = GeocoderResult {
            display_name: "Eiffel Tower".to_string(),
            destination: GeocoderDestination::Point {
                longitude: 0.0407,
                latitude: 0.8517,
                height: Some(330.0),
            },
            attributions: vec![],
        };
        if let GeocoderDestination::Point { longitude, latitude, height } = result.destination {
            assert!((longitude - 0.0407).abs() < 1e-4);
            assert!((latitude - 0.8517).abs() < 1e-4);
            assert_eq!(height, Some(330.0));
        } else {
            panic!("Expected Point");
        }
    }

    #[test]
    fn test_geocoder_attribution() {
        let attr = GeocoderAttribution {
            mandatory: true,
            collapsible: false,
            html: "<a href='https://example.com'>Example</a>".to_string(),
        };
        assert!(attr.mandatory);
        assert!(!attr.collapsible);
    }

    #[test]
    fn test_mock_geocoder_service() {
        // 预置单条结果，无论查询什么均应原样返回
        let results = vec![GeocoderResult {
            display_name: "Test Location".to_string(),
            destination: GeocoderDestination::Point {
                longitude: 0.0,
                latitude: 0.0,
                height: None,
            },
            attributions: vec![],
        }];

        let service = MockGeocoderService::with_results(results);
        let geocode_results = service.geocode("test", GeocodeType::Search);
        assert_eq!(geocode_results.len(), 1);
        assert_eq!(geocode_results[0].display_name, "Test Location");
    }

    #[test]
    fn test_mock_geocoder_credit() {
        let mut service = MockGeocoderService::new();
        assert!(service.credit().is_none());

        service.credit = Some("Test Credit".to_string());
        assert_eq!(service.credit(), Some("Test Credit"));
    }

    #[test]
    fn test_get_credits_from_result() {
        // 两条归属的 mandatory 标志应被完整保留
        let result = GeocoderResult {
            display_name: "Test".to_string(),
            destination: GeocoderDestination::Point {
                longitude: 0.0,
                latitude: 0.0,
                height: None,
            },
            attributions: vec![
                GeocoderAttribution {
                    mandatory: true,
                    collapsible: false,
                    html: "Credit 1".to_string(),
                },
                GeocoderAttribution {
                    mandatory: false,
                    collapsible: true,
                    html: "Credit 2".to_string(),
                },
            ],
        };

        let credits = get_credits_from_result(&result);
        assert_eq!(credits.len(), 2);
        assert!(credits[0].mandatory);
        assert!(!credits[1].mandatory);
    }

    #[test]
    fn test_geocoder_result_serialization() {
        // 序列化/反序列化往返应保留 display_name
        let result = GeocoderResult {
            display_name: "Test".to_string(),
            destination: GeocoderDestination::Rectangle([0.0, 0.0, 1.0, 1.0]),
            attributions: vec![],
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("Test"));

        let deserialized: GeocoderResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.display_name, "Test");
    }
}
