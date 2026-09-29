//! 地理编码服务接口与类型。
//!
//! 映射到 CesiumJS：
//! - `Core/GeocoderService.js`
//! - `Core/GeocodeType.js`
//! - `Core/BingMapsGeocoderService.js`
//! - `Core/PeliasGeocoderService.js`
//! - `Core/OpenCageGeocoderService.js`

use serde::{Deserialize, Serialize};

/// 要执行的地理编码类型。
///
/// 映射到 CesiumJS `Core/GeocodeType.js`。
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
///
/// 映射到 CesiumJS `GeocoderService.Result`。
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
///
/// 映射到 CesiumJS `Core/GeocoderService.js`。
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
        Self::default()
    }

    /// 使用预定义结果创建。
    pub fn with_results(results: Vec<GeocoderResult>) -> Self {
        Self {
            results,
            credit: None,
        }
    }
}

impl GeocoderService for MockGeocoderService {
    fn credit(&self) -> Option<&str> {
        self.credit.as_deref()
    }

    fn geocode(&self, _query: &str, _geocode_type: GeocodeType) -> Vec<GeocoderResult> {
        self.results.clone()
    }
}

/// 从地理编码结果的归属中解析 credit。
///
/// 映射到 CesiumJS `GeocoderService.getCreditsFromResult`。
pub fn get_credits_from_result(result: &GeocoderResult) -> Vec<&GeocoderAttribution> {
    result.attributions.iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geocode_type_default() {
        assert_eq!(GeocodeType::default(), GeocodeType::Search);
    }

    #[test]
    fn test_geocoder_result_rectangle() {
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
