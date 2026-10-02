// legacy CesiumJS-port style debt (deferred.md #18); revisit at M13 lint-cleanup 或本文件在其里程碑被重写时
#![allow(clippy::derivable_impls)]

//! 地理编码搜索控件：用键盘驱动的极简经纬度输入框。
//!
//! 按 `/` 激活输入，逐键累积字母/数字/小数点/负号/逗号，回车
//! 解析为 (纬度, 经度) 并发出 [`FlyToRequest`]，`Esc` 取消。
//! 这是一个无图像依赖的文本小部件，真实 UI 布局尚未接入。

use bevy::prelude::*;

use crate::camera::FlyToRequest;

/// 地理编码控件状态资源。
#[derive(Resource, Debug, Clone)]
pub struct GeocoderWidget {
    /// 当前输入的文本缓冲。
    pub search_text: String,
    /// 是否处于激活（输入中）状态。
    pub is_active: bool,
    /// 是否显示（预留给未来的可见性控制）。
    pub show: bool,
}

impl Default for GeocoderWidget {
    /// 默认空文本、未激活、不显示。
    fn default() -> Self {
        Self {
            search_text: String::new(),
            is_active: false,
            show: false,
        }
    }
}

impl GeocoderWidget {
    /// 清空输入并退出激活状态（保留 show 不变）。
    pub fn clear(&mut self) {
        self.search_text.clear();
        self.is_active = false;
    }
}

/// 控件初始化占位系统（当前无实体需生成，保留接口）。
pub fn setup_geocoder_widget(mut _commands: Commands) {}

/// 主输入系统：根据键盘事件维护 [`GeocoderWidget`] 状态，回车时
/// 解析经纬度并发出飞行请求。
///
/// # 参数
/// - `keyboard`：按键输入状态
/// - `_keyboard_chars`：预留（未使用的字符输入）
/// - `widget`：控件状态（可写）
/// - `fly_events`：解析成功时发送的 [`FlyToRequest`] 事件写出器
pub fn geocoder_widget_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    _keyboard_chars: Res<ButtonInput<KeyCode>>,
    mut widget: ResMut<GeocoderWidget>,
    mut fly_events: EventWriter<FlyToRequest>,
) {
    // 按 `/` 且未激活→进入输入模式，清空缓冲。
    if keyboard.just_pressed(KeyCode::Slash) && !widget.is_active {
        // 以 `/` 作为激活热键，与命令行式交互习惯一致。
        widget.is_active = true;
        widget.search_text.clear();
        info!("Geocoder activated. Enter lat,lon to fly.");
        return;
    }

    // 未激活时忽略后续所有按键。
    if !widget.is_active {
        // 未处于输入模式，直接返回避免误采集。
        return;
    }

    // Esc 取消：清空并退出。
    if keyboard.just_pressed(KeyCode::Escape) {
        widget.clear();
        return;
    }

    // Enter 提交：解析当前文本为经纬度，成功则发飞行请求。
    if keyboard.just_pressed(KeyCode::Enter) {
        let text = widget.search_text.trim().to_string();
        if !text.is_empty() {
            // 尝试把缓冲解析为经纬度二元组。
            let result = parse_lat_lon(&text);
            if let Some((lat_deg, lon_deg)) = result {
                // 纬度在前、经度在后；高度固定 100km 以上以便俯瞰。
                let carto = cesium_geospatial::cartographic::Cartographic::from_degrees(
                    lon_deg, lat_deg, 100000.0,
                );
                fly_events.send(FlyToRequest {
                    destination: carto,
                    duration_secs: 1.5,
                });
                info!("Flying to lat={:.4}, lon={:.4}", lat_deg, lon_deg);
            } else {
                info!(
                    "Could not parse '{}'. Expected format: lat,lon (e.g. 40.7,-74.0)",
                    text
                );
            }
        }
        widget.clear();
        return;
    }

    // 退格删除末位字符。
    if keyboard.just_pressed(KeyCode::Backspace) {
        // 仅移除缓冲末尾一个字符，支持逐字修正。
        widget.search_text.pop();
        return;
    }

    // 字母键：手动映射 KeyCode→字符（Bevy 不直接提供字符输入）。
    // 遍历 26 个字母主键，逐个检测是否刚按下。
    for &code in &[
        KeyCode::KeyA, KeyCode::KeyB, KeyCode::KeyC, KeyCode::KeyD,
        KeyCode::KeyE, KeyCode::KeyF, KeyCode::KeyG, KeyCode::KeyH,
        KeyCode::KeyI, KeyCode::KeyJ, KeyCode::KeyK, KeyCode::KeyL,
        KeyCode::KeyM, KeyCode::KeyN, KeyCode::KeyO, KeyCode::KeyP,
        KeyCode::KeyQ, KeyCode::KeyR, KeyCode::KeyS, KeyCode::KeyT,
        KeyCode::KeyU, KeyCode::KeyV, KeyCode::KeyW, KeyCode::KeyX,
        KeyCode::KeyY, KeyCode::KeyZ,
    ] {
        if keyboard.just_pressed(code) {
            // 按键映射为对应小写字母，未命中则跳过本项。
            let ch = match code {
                KeyCode::KeyA => 'a',
                KeyCode::KeyB => 'b',
                KeyCode::KeyC => 'c',
                KeyCode::KeyD => 'd',
                KeyCode::KeyE => 'e',
                KeyCode::KeyF => 'f',
                KeyCode::KeyG => 'g',
                KeyCode::KeyH => 'h',
                KeyCode::KeyI => 'i',
                KeyCode::KeyJ => 'j',
                KeyCode::KeyK => 'k',
                KeyCode::KeyL => 'l',
                KeyCode::KeyM => 'm',
                KeyCode::KeyN => 'n',
                KeyCode::KeyO => 'o',
                KeyCode::KeyP => 'p',
                KeyCode::KeyQ => 'q',
                KeyCode::KeyR => 'r',
                KeyCode::KeyS => 's',
                KeyCode::KeyT => 't',
                KeyCode::KeyU => 'u',
                KeyCode::KeyV => 'v',
                KeyCode::KeyW => 'w',
                KeyCode::KeyX => 'x',
                KeyCode::KeyY => 'y',
                KeyCode::KeyZ => 'z',
                _ => continue,
            };
            // Shift 按下时转大写。
            let upper = if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
                ch.to_ascii_uppercase()
            } else {
                ch
            };
            widget.search_text.push(upper);
        }
    }

    // 分隔符与小数点：逗号分隔经纬度，句点作小数点。
    if keyboard.just_pressed(KeyCode::Comma) {
        // 逗号作为纬度与经度的分隔符写入缓冲。
        widget.search_text.push(',');
    }
    if keyboard.just_pressed(KeyCode::Period) {
        // 句点作为小数点，允许输入分数度数。
        widget.search_text.push('.');
    }
    // 负号：兼容主键区与数字键盘减号。
    if keyboard.just_pressed(KeyCode::Minus) || keyboard.just_pressed(KeyCode::NumpadSubtract) {
        // 允许输入南纬/西经的负值。
        widget.search_text.push('-');
    }
    // 数字 0-9：主键区与数字键盘均可触发，逐个追加到缓冲。
    if keyboard.just_pressed(KeyCode::Digit0) || keyboard.just_pressed(KeyCode::Numpad0) {
        widget.search_text.push('0');
    }
    if keyboard.just_pressed(KeyCode::Digit1) || keyboard.just_pressed(KeyCode::Numpad1) {
        widget.search_text.push('1');
    }
    if keyboard.just_pressed(KeyCode::Digit2) || keyboard.just_pressed(KeyCode::Numpad2) {
        widget.search_text.push('2');
    }
    if keyboard.just_pressed(KeyCode::Digit3) || keyboard.just_pressed(KeyCode::Numpad3) {
        widget.search_text.push('3');
    }
    if keyboard.just_pressed(KeyCode::Digit4) || keyboard.just_pressed(KeyCode::Numpad4) {
        widget.search_text.push('4');
    }
    if keyboard.just_pressed(KeyCode::Digit5) || keyboard.just_pressed(KeyCode::Numpad5) {
        widget.search_text.push('5');
    }
    if keyboard.just_pressed(KeyCode::Digit6) || keyboard.just_pressed(KeyCode::Numpad6) {
        widget.search_text.push('6');
    }
    if keyboard.just_pressed(KeyCode::Digit7) || keyboard.just_pressed(KeyCode::Numpad7) {
        widget.search_text.push('7');
    }
    if keyboard.just_pressed(KeyCode::Digit8) || keyboard.just_pressed(KeyCode::Numpad8) {
        widget.search_text.push('8');
    }
    if keyboard.just_pressed(KeyCode::Digit9) || keyboard.just_pressed(KeyCode::Numpad9) {
        widget.search_text.push('9');
    }

    // 每次输入后回显当前缓冲，便于无 UI 时调试。
    info!("Geocoder: {}", widget.search_text);
}

/// 解析 "lat,lon" 格式的经纬度字符串。
///
/// 输入以逗号分隔两个十进制度数分量，先纬后经。
///
/// # 返回
/// 合法时返回 (纬度, 经度)；分隔不为两项、非数字或超出
/// 纬度 [-90,90] / 经度 [-180,180] 范围时返回 `None`。
fn parse_lat_lon(text: &str) -> Option<(f64, f64)> {
    // 以逗号切分，必须恰好两个分量。
    let parts: Vec<&str> = text.split(',').collect();
    if parts.len() != 2 {
        return None;
    }

    // 分别解析为 f64，任一失败即返回 None（`?` 提前退出）。
    let lat: f64 = parts[0].trim().parse().ok()?;
    let lon: f64 = parts[1].trim().parse().ok()?;

    // 纬度范围校验 [-90, 90]。
    if !(-90.0..=90.0).contains(&lat) {
        return None;
    }
    // 经度范围校验 [-180, 180]。
    if !(-180.0..=180.0).contains(&lon) {
        return None;
    }

    // 全部校验通过，返回 (纬度, 经度) 元组。
    Some((lat, lon))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 合法经纬度解析后数值应精确匹配。
    fn test_parse_lat_lon_valid() {
        let result = parse_lat_lon("40.7128,-74.0060");
        assert!(result.is_some());
        let (lat, lon) = result.unwrap();
        assert!((lat - 40.7128).abs() < 1e-6);
        assert!((lon - (-74.0060)).abs() < 1e-6);
    }

    #[test]
    /// 分量两侧空白应被 trim 掉，仍解析成功。
    fn test_parse_lat_lon_with_spaces() {
        let result = parse_lat_lon(" 51.5074 , -0.1278 ");
        assert!(result.is_some());
        let (lat, lon) = result.unwrap();
        assert!((lat - 51.5074).abs() < 1e-6);
        assert!((lon - (-0.1278)).abs() < 1e-6);
    }

    #[test]
    /// 空串/非数字/超范围/多余分量均应返回 None。
    fn test_parse_lat_lon_invalid() {
        assert!(parse_lat_lon("").is_none());
        assert!(parse_lat_lon("abc").is_none());
        assert!(parse_lat_lon("91,0").is_none());
        assert!(parse_lat_lon("0,181").is_none());
        assert!(parse_lat_lon("0,0,0").is_none());
    }

    #[test]
    /// 默认状态：未激活、空文本、不显示。
    fn test_geocoder_widget_default() {
        let widget = GeocoderWidget::default();
        assert!(!widget.is_active);
        assert!(widget.search_text.is_empty());
        assert!(!widget.show);
    }

    #[test]
    /// clear 清空文本并退出激活，但不改动 show。
    fn test_geocoder_widget_clear() {
        let mut widget = GeocoderWidget {
            search_text: "test".into(),
            is_active: true,
            show: true,
        };
        widget.clear();
        assert!(widget.search_text.is_empty());
        assert!(!widget.is_active);
    }
}
