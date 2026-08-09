use bevy::prelude::*;

use crate::camera::FlyToRequest;

#[derive(Resource, Debug, Clone)]
pub struct GeocoderWidget {
    pub search_text: String,
    pub is_active: bool,
    pub show: bool,
}

impl Default for GeocoderWidget {
    fn default() -> Self {
        Self {
            search_text: String::new(),
            is_active: false,
            show: false,
        }
    }
}

impl GeocoderWidget {
    pub fn clear(&mut self) {
        self.search_text.clear();
        self.is_active = false;
    }
}

pub fn setup_geocoder_widget(mut _commands: Commands) {}

pub fn geocoder_widget_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    _keyboard_chars: Res<ButtonInput<KeyCode>>,
    mut widget: ResMut<GeocoderWidget>,
    mut fly_events: EventWriter<FlyToRequest>,
) {
    if keyboard.just_pressed(KeyCode::Slash) && !widget.is_active {
        widget.is_active = true;
        widget.search_text.clear();
        info!("Geocoder activated. Enter lat,lon to fly.");
        return;
    }

    if !widget.is_active {
        return;
    }

    if keyboard.just_pressed(KeyCode::Escape) {
        widget.clear();
        return;
    }

    if keyboard.just_pressed(KeyCode::Enter) {
        let text = widget.search_text.trim().to_string();
        if !text.is_empty() {
            let result = parse_lat_lon(&text);
            if let Some((lat_deg, lon_deg)) = result {
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

    if keyboard.just_pressed(KeyCode::Backspace) {
        widget.search_text.pop();
        return;
    }

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
            let upper = if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
                ch.to_ascii_uppercase()
            } else {
                ch
            };
            widget.search_text.push(upper);
        }
    }

    if keyboard.just_pressed(KeyCode::Comma) {
        widget.search_text.push(',');
    }
    if keyboard.just_pressed(KeyCode::Period) {
        widget.search_text.push('.');
    }
    if keyboard.just_pressed(KeyCode::Minus) || keyboard.just_pressed(KeyCode::NumpadSubtract) {
        widget.search_text.push('-');
    }
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

    info!("Geocoder: {}", widget.search_text);
}

fn parse_lat_lon(text: &str) -> Option<(f64, f64)> {
    let parts: Vec<&str> = text.split(',').collect();
    if parts.len() != 2 {
        return None;
    }

    let lat: f64 = parts[0].trim().parse().ok()?;
    let lon: f64 = parts[1].trim().parse().ok()?;

    if !(-90.0..=90.0).contains(&lat) {
        return None;
    }
    if !(-180.0..=180.0).contains(&lon) {
        return None;
    }

    Some((lat, lon))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lat_lon_valid() {
        let result = parse_lat_lon("40.7128,-74.0060");
        assert!(result.is_some());
        let (lat, lon) = result.unwrap();
        assert!((lat - 40.7128).abs() < 1e-6);
        assert!((lon - (-74.0060)).abs() < 1e-6);
    }

    #[test]
    fn test_parse_lat_lon_with_spaces() {
        let result = parse_lat_lon(" 51.5074 , -0.1278 ");
        assert!(result.is_some());
        let (lat, lon) = result.unwrap();
        assert!((lat - 51.5074).abs() < 1e-6);
        assert!((lon - (-0.1278)).abs() < 1e-6);
    }

    #[test]
    fn test_parse_lat_lon_invalid() {
        assert!(parse_lat_lon("").is_none());
        assert!(parse_lat_lon("abc").is_none());
        assert!(parse_lat_lon("91,0").is_none());
        assert!(parse_lat_lon("0,181").is_none());
        assert!(parse_lat_lon("0,0,0").is_none());
    }

    #[test]
    fn test_geocoder_widget_default() {
        let widget = GeocoderWidget::default();
        assert!(!widget.is_active);
        assert!(widget.search_text.is_empty());
        assert!(!widget.show);
    }

    #[test]
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
