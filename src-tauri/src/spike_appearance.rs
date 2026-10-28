//! Presentation preferences only. The icon detector never reads this file.
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{Emitter, Manager};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct WidgetAppearance {
    pub font_size: f64,
    pub background_opacity: f64,
}
impl Default for WidgetAppearance {
    fn default() -> Self {
        Self {
            font_size: 30.0,
            background_opacity: 0.78,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Appearance {
    pub position: Option<Point>,
    pub font_size: f64,
    pub background_opacity: f64,
    pub game_bar: WidgetAppearance,
}
impl Default for Appearance {
    fn default() -> Self {
        Self {
            position: None,
            font_size: 30.0,
            background_opacity: 0.78,
            game_bar: WidgetAppearance::default(),
        }
    }
}
impl Appearance {
    fn validate(&self) -> Result<(), String> {
        if !self.font_size.is_finite()
            || !(14.0..=84.0).contains(&self.font_size)
            || !self.background_opacity.is_finite()
            || !(0.0..=1.0).contains(&self.background_opacity)
            || !self.game_bar.font_size.is_finite()
            || !(14.0..=84.0).contains(&self.game_bar.font_size)
            || !self.game_bar.background_opacity.is_finite()
            || !(0.0..=1.0).contains(&self.game_bar.background_opacity)
            || self.position.is_some_and(|p| {
                !p.x.is_finite()
                    || !p.y.is_finite()
                    || !(0.0..=1.0).contains(&p.x)
                    || !(0.0..=1.0).contains(&p.y)
            })
        {
            return Err("Invalid overlay appearance".into());
        }
        Ok(())
    }
    pub fn logical_size(&self) -> (f64, f64) {
        (
            (self.font_size * 4.2 + 12.0).max(200.0),
            self.font_size * 1.2 + 10.0,
        )
    }
}
pub struct Preferences {
    value: Mutex<Appearance>,
    path: PathBuf,
}
fn decode(bytes: &[u8]) -> Option<Appearance> {
    if bytes.len() > 64 * 1024 {
        return None;
    }
    let json: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let mut value: Appearance = serde_json::from_value(json.clone()).ok()?;
    if json.get("game_bar").is_none() {
        value.game_bar = WidgetAppearance {
            font_size: value.font_size,
            background_opacity: value.background_opacity,
        };
    }
    value.validate().ok()?;
    Some(value)
}
pub fn setup(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let path = app.path().app_data_dir()?.join("spike-overlay.json");
    let value = std::fs::read(&path)
        .ok()
        .and_then(|b| decode(&b))
        .unwrap_or_default();
    app.manage(Preferences {
        value: Mutex::new(value),
        path,
    });
    Ok(())
}
pub fn current(app: &tauri::AppHandle) -> Appearance {
    app.state::<Preferences>().value.lock().clone()
}
#[tauri::command]
pub fn get_spike_appearance(app: tauri::AppHandle) -> Appearance {
    current(&app)
}
#[tauri::command]
pub fn set_spike_appearance(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    appearance: Appearance,
) -> Result<Appearance, String> {
    if window.label() != "main" {
        return Err("window".into());
    }
    appearance.validate()?;
    let state = app.state::<Preferences>();
    let mut value = state.value.lock();
    persist(&app, &state, &mut value, appearance)
}
fn persist(
    app: &tauri::AppHandle,
    state: &Preferences,
    value: &mut Appearance,
    appearance: Appearance,
) -> Result<Appearance, String> {
    std::fs::create_dir_all(state.path.parent().ok_or("storage")?).map_err(|_| "storage")?;
    let temp = state.path.with_extension("json.tmp");
    std::fs::write(
        &temp,
        serde_json::to_vec_pretty(&appearance).map_err(|_| "storage")?,
    )
    .map_err(|_| "storage")?;
    std::fs::rename(&temp, &state.path).map_err(|_| "storage")?;
    *value = appearance.clone();
    let _ = app.emit("spike-overlay-appearance", &appearance);
    Ok(appearance)
}
#[tauri::command]
pub fn set_spike_gamebar_appearance(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    appearance: WidgetAppearance,
) -> Result<Appearance, String> {
    if window.label() != "main" {
        return Err("window".into());
    }
    let state = app.state::<Preferences>();
    let mut value = state.value.lock();
    let mut updated = value.clone();
    updated.game_bar = appearance;
    updated.validate()?;
    persist(&app, &state, &mut value, updated)
}
#[derive(Serialize)]
pub struct EditorContext {
    width: u32,
    height: u32,
    scale: f64,
    game_found: bool,
}
#[tauri::command]
pub fn get_spike_editor_context(window: tauri::WebviewWindow) -> Result<EditorContext, String> {
    if window.label() != "main" {
        return Err("window".into());
    }
    let game = crate::spike_display::game_monitor();
    let monitors = window.available_monitors().map_err(|_| "display")?;
    let target = game
        .or_else(|| {
            window
                .current_monitor()
                .ok()
                .flatten()
                .map(|m| crate::spike_display::MonitorBounds {
                    x: m.position().x,
                    y: m.position().y,
                    width: m.size().width,
                    height: m.size().height,
                })
        })
        .unwrap_or(crate::spike_display::MonitorBounds {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        });
    let scale = crate::spike_display::target_scale(target, &monitors);
    Ok(EditorContext {
        width: target.width,
        height: target.height,
        scale,
        game_found: game.is_some(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn appearance_validation_and_roundtrip() {
        let a = Appearance {
            position: Some(Point { x: 0.3, y: 0.8 }),
            font_size: 64.0,
            background_opacity: 0.0,
            ..Appearance::default()
        };
        assert!(a.validate().is_ok());
        assert_eq!(
            a,
            serde_json::from_str(&serde_json::to_string(&a).unwrap()).unwrap()
        );
        for value in [f64::NAN, -1.0, 100.0] {
            assert!(Appearance {
                font_size: value,
                ..a.clone()
            }
            .validate()
            .is_err());
        }
        assert!(Appearance {
            position: Some(Point { x: 1.1, y: 0.0 }),
            ..a
        }
        .validate()
        .is_err());
    }
    #[test]
    fn legacy_preferences_migrate_without_changing_desktop_style() {
        let value =
            decode(br#"{"position":{"x":0.3,"y":0.2},"font_size":61,"background_opacity":0.2}"#)
                .unwrap();
        assert_eq!(value.font_size, 61.0);
        assert_eq!(value.game_bar.font_size, 61.0);
        assert_eq!(value.game_bar.background_opacity, 0.2);
        let mut updated = value.clone();
        updated.game_bar.font_size = 24.0;
        assert_eq!(updated.position, value.position);
        assert_eq!(updated.font_size, 61.0);
        assert!(updated.validate().is_ok());
        updated.game_bar.font_size = f64::NAN;
        assert!(updated.validate().is_err());
    }
}
