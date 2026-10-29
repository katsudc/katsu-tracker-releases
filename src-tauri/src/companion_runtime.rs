//! Run-scoped optional integrations and native tray. Detector implementations stay untouched.
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{path::PathBuf, sync::{atomic::{AtomicBool, Ordering}, OnceLock}};
use tauri::{Emitter, Manager, menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem}, tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent}};

#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct DesktopPreferences {
    pub minimize_to_tray: bool,
    pub close_to_tray: bool,
    pub start_minimized: bool,
}
#[derive(Clone, Serialize)]
pub struct RuntimeStatus {
    pub mode: String,
    pub previous_unclean: bool,
    pub disabled_modules: Vec<String>,
    pub failed_modules: Vec<String>,
    pub started_minimized: bool,
}
pub struct Runtime {
    path: PathBuf,
    marker: PathBuf,
    status: Mutex<RuntimeStatus>,
    desktop: Mutex<DesktopPreferences>,
    desired_discord: AtomicBool,
    initializing: AtomicBool,
}
struct Tray {
    streamer: CheckMenuItem<tauri::Wry>,
    spike: CheckMenuItem<tauri::Wry>,
    instalock: CheckMenuItem<tauri::Wry>,
    open: MenuItem<tauri::Wry>,
    state: MenuItem<tauri::Wry>,
    exit: MenuItem<tauri::Wry>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraySnapshot {
    pub streamer: bool, pub spike: bool, pub instalock: bool,
    pub game: String, pub locale: String,
    pub preferences: DesktopPreferences,
}
fn label(locale: &str, key: &str) -> String {
    static LABELS: OnceLock<Value> = OnceLock::new();
    let labels = LABELS.get_or_init(|| serde_json::from_str(include_str!("../../raw/suite-native-labels.json")).unwrap_or_default());
    labels.get(if locale == "tr" { "tr" } else { "en" }).and_then(|v| v.get(key)).and_then(Value::as_str).unwrap_or(key).into()
}
fn read<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> Option<T> {
    if std::fs::metadata(path).ok()?.len() > 4096 { return None; }
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}
fn write<T: Serialize>(path: &std::path::Path, value: &T) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec(value).map_err(|_| "storage")?).and_then(|_| std::fs::rename(tmp, path)).map_err(|_| "storage".into())
}
pub fn normal(app: &tauri::AppHandle) -> bool {
    app.try_state::<Runtime>().is_none_or(|r| r.status.lock().mode == "normal")
}
pub fn status(app: &tauri::AppHandle) -> RuntimeStatus { app.state::<Runtime>().status.lock().clone() }
pub fn setup(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    let marker = dir.join("run-state.json");
    let prior = marker.exists() && read::<Value>(&marker).is_none_or(|v| v.get("clean").and_then(Value::as_bool) != Some(true));
    let force_safe = std::env::args().any(|a| a == "--safe-mode");
    let path = dir.join("desktop-preferences.json");
    let desktop = read::<DesktopPreferences>(&path).unwrap_or_default();
    write(&marker, &json!({"schema":1,"clean":false,"version":app.package_info().version.to_string()})).ok();
    app.manage(Runtime {
        path, marker, status: Mutex::new(RuntimeStatus {
            mode: if force_safe { "safe" } else if prior { "pending" } else { "normal" }.into(),
            previous_unclean: prior, disabled_modules: if prior || force_safe { vec!["spotify".into(),"discord".into(),"game_bar".into(),"overlays".into(),"effects".into()] } else { vec![] },
            failed_modules: vec![], started_minimized: desktop.start_minimized && !prior && !force_safe,
        }),
        desktop: Mutex::new(desktop), desired_discord: AtomicBool::new(false), initializing: AtomicBool::new(false),
    });
    if setup_tray(app).is_err() { app.state::<Runtime>().status.lock().failed_modules.push("tray".into()); }
    if normal(app) { start_optional(app); }
    if status(app).started_minimized && app.try_state::<Tray>().is_some() {
        if let Some(window) = app.get_webview_window("main") { let _ = window.hide(); crate::set_webview_visible(&window, false); }
    }
    Ok(())
}
fn start_optional(app: &tauri::AppHandle) {
    let runtime = app.state::<Runtime>();
    if runtime.initializing.swap(true, Ordering::SeqCst) { return; }
    tracing::info!("[Startup] optional integrations starting");
    // The original initialization order is retained in normal mode.
    let mut failures: Vec<String> = runtime.status.lock().failed_modules.iter().filter(|v| v.as_str() == "tray").cloned().collect();
    tracing::info!("[Startup] initializing game_bar");
    if crate::spike_gamebar::setup(app).is_err() { failures.push("game_bar".into()); }
    tracing::info!("[Startup] initializing overlays");
    if crate::spike::setup(app).is_err() { failures.push("overlays".into()); }
    tracing::info!("[Startup] initializing spotify");
    if crate::media::setup(app).is_err() { failures.push("spotify".into()); }
    app.state::<crate::state::AppState>().discord.set_enabled(runtime.desired_discord.load(Ordering::Relaxed));
    tracing::info!("[Startup] optional integrations ready; failures={}", failures.len());
    runtime.status.lock().failed_modules = failures;
}
pub fn clean_exit(app: &tauri::AppHandle) {
    if let Some(runtime) = app.try_state::<Runtime>() {
        write(&runtime.marker, &json!({"schema":1,"clean":true,"version":app.package_info().version.to_string()})).ok();
    }
}
pub fn close_to_tray(app: &tauri::AppHandle) -> bool { app.try_state::<Tray>().is_some() && app.try_state::<Runtime>().is_some_and(|r| r.desktop.lock().close_to_tray) }
pub fn minimize_to_tray(app: &tauri::AppHandle) -> bool { app.try_state::<Tray>().is_some() && app.try_state::<Runtime>().is_some_and(|r| r.desktop.lock().minimize_to_tray) }
pub fn reveal(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        crate::set_webview_visible(&window, true);
        let _ = window.unminimize(); let _ = window.show(); let _ = window.set_focus();
        let _ = app.emit("show-overlay", ());
    }
}
fn main_only(window: &tauri::WebviewWindow) -> Result<(), String> { if window.label() == "main" { Ok(()) } else { Err("window".into()) } }
#[tauri::command]
pub fn runtime_status(app: tauri::AppHandle) -> RuntimeStatus { status(&app) }
#[tauri::command]
pub fn own_account_id(state: tauri::State<'_, crate::state::AppState>) -> Option<String> {
    if !*state.api.connected.read() { return None; }
    let id=state.api.puuid.read().clone();
    (id.len()==36 && id.chars().all(|c|c.is_ascii_hexdigit()||c=='-')).then_some(id)
}
#[tauri::command]
pub async fn runtime_choose(window: tauri::WebviewWindow, app: tauri::AppHandle, safe: bool) -> Result<RuntimeStatus, String> {
    main_only(&window)?;
    {
        let runtime = app.state::<Runtime>(); let mut st = runtime.status.lock();
        if st.mode != "pending" { return Ok(st.clone()); }
        st.mode = if safe { "safe" } else { "normal" }.into();
        if !safe { st.disabled_modules.clear(); }
    }
    if !safe {
        // WebView2 window creation must not run inside a synchronous IPC handler
        // on Windows. Keep the event loop free while the existing overlay setup
        // creates its WebView; await completion before resolving the choice.
        let startup_app = app.clone();
        tauri::async_runtime::spawn_blocking(move || start_optional(&startup_app))
            .await.map_err(|_| "integration_start_failed".to_string())?;
    }
    let result = status(&app); let _ = app.emit("runtime-status", &result); Ok(result)
}
#[tauri::command]
pub fn guarded_set_discord_rpc(app: tauri::AppHandle, state: tauri::State<'_, crate::state::AppState>, enabled: bool) {
    if let Some(runtime) = app.try_state::<Runtime>() { runtime.desired_discord.store(enabled, Ordering::Relaxed); }
    state.discord.set_enabled(normal(&app) && enabled);
}
#[tauri::command]
pub fn guarded_media_status(app: tauri::AppHandle) -> crate::media::Snapshot {
    app.try_state::<crate::media::Media>().map(|s| crate::media::media_status(s)).unwrap_or_default()
}
#[tauri::command]
pub async fn guarded_media_control(app: tauri::AppHandle, action: String, volume: Option<f32>) -> Result<(), String> {
    if !normal(&app) { return Err("integration_disabled".into()); }
    let media = app.try_state::<crate::media::Media>().ok_or("integration_unavailable")?;
    crate::media::media_control(media, action, volume).await
}
#[tauri::command]
pub fn guarded_spike_get_status(app: tauri::AppHandle) -> Value {
    if let Some(state) = app.try_state::<crate::spike::Spike>() { return serde_json::to_value(crate::spike::spike_get_status(state)).unwrap_or_default(); }
    json!({"enabled":false,"icon_enabled":false,"icon_status":"Kapalı","icon_similarity":0,"remaining":null,"full_defuse":false,"half_defuse":false,"demo":false,"error":null,
        "calibrated":false,"recording":false,"recorded_seconds":0,"recording_ready":false,"full_sequence":false,"reference_seconds":0,"correlation_ready":false,
        "correlation_threshold":0,"correlation_similarity":0,"similarity":0,"peak_similarity":0,"listening":false,"level":0,"threshold":0,"reference_name":""})
}
#[tauri::command]
pub fn guarded_get_spike_output_status(app: tauri::AppHandle, locale: Option<String>) -> Value {
    if app.try_state::<crate::spike_gamebar::GameBar>().is_some() {
        return serde_json::to_value(crate::spike_gamebar::get_spike_output_status(app, locale)).unwrap_or_default();
    }
    let mut value = serde_json::to_value(crate::spike_gamebar::OutputStatus::default()).unwrap_or_default();
    value["phase"] = json!(if normal(&app) { "unavailable" } else { "safe_mode" }); value
}
fn setup_tray(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let open = MenuItem::with_id(app, "open", label("en","suite.trayOpen"), true, None::<&str>)?;
    let streamer = CheckMenuItem::with_id(app,"streamer",label("en","suite.trayStreamer"),true,false,None::<&str>)?;
    let spike = CheckMenuItem::with_id(app,"spike",label("en","suite.traySpike"),normal(app),false,None::<&str>)?;
    let instalock = CheckMenuItem::with_id(app,"instalock",label("en","suite.trayInstalock"),true,false,None::<&str>)?;
    let state = MenuItem::with_id(app,"game",label("en","suite.state.disconnected"),false,None::<&str>)?;
    let exit = MenuItem::with_id(app,"exit",label("en","suite.trayExit"),true,None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app,&[&open,&separator,&streamer,&spike,&instalock,&state,&exit])?;
    let mut builder = TrayIconBuilder::with_id("katsu-main").tooltip("katsu's Tracker").menu(&menu).show_menu_on_left_click(false)
        .on_menu_event(|app,event| match event.id().as_ref() {
            "open" => reveal(app), "exit" => app.exit(0),
            "streamer" | "spike" | "instalock" => { let _ = app.emit("tray-action", event.id().as_ref()); }, _ => {}
        })
        .on_tray_icon_event(|icon,event| { if matches!(event,TrayIconEvent::Click { button:MouseButton::Left,button_state:MouseButtonState::Up,.. }) { reveal(icon.app_handle()); } });
    if let Some(icon) = app.default_window_icon() { builder = builder.icon(icon.clone()); }
    builder.build(app)?;
    app.manage(Tray { streamer,spike,instalock,open,state,exit }); Ok(())
}
#[tauri::command]
pub fn tray_sync(window: tauri::WebviewWindow, app: tauri::AppHandle, snapshot: TraySnapshot) -> Result<(), String> {
    main_only(&window)?;
    let tray = app.try_state::<Tray>().ok_or("tray_unavailable")?;
    let _ = tray.streamer.set_checked(snapshot.streamer); let _ = tray.spike.set_checked(snapshot.spike); let _ = tray.instalock.set_checked(snapshot.instalock);
    let _ = tray.spike.set_enabled(normal(&app));
    let _ = tray.open.set_text(label(&snapshot.locale,"suite.trayOpen"));
    let _ = tray.streamer.set_text(label(&snapshot.locale,"suite.trayStreamer"));
    let _ = tray.spike.set_text(label(&snapshot.locale,"suite.traySpike"));
    let _ = tray.instalock.set_text(label(&snapshot.locale,"suite.trayInstalock"));
    let _ = tray.exit.set_text(label(&snapshot.locale,"suite.trayExit"));
    let game = if ["idle","pregame","ingame","disconnected"].contains(&snapshot.game.as_str()) { snapshot.game.as_str() } else { "disconnected" };
    let _ = tray.state.set_text(format!("{}: {}",label(&snapshot.locale,"suite.trayState"),label(&snapshot.locale,&format!("suite.state.{game}"))));
    let runtime = app.state::<Runtime>(); let mut prefs = runtime.desktop.lock();
    if *prefs != snapshot.preferences { write(&runtime.path,&snapshot.preferences)?; *prefs = snapshot.preferences; }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn companion_smoke_marker_preserves_preferences() {
        let dir=std::env::temp_dir().join(format!("katsu-smoke-{}",uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        let marker=dir.join("run-state.json");let prefs=dir.join("desktop-preferences.json");
        let saved=super::DesktopPreferences {close_to_tray:true,start_minimized:false,minimize_to_tray:true};
        super::write(&prefs,&saved).unwrap();let original=std::fs::read(&prefs).unwrap();
        super::write(&marker,&serde_json::json!({"clean":false})).unwrap();
        assert_eq!(super::read::<serde_json::Value>(&marker).unwrap()["clean"],false);
        super::write(&marker,&serde_json::json!({"clean":true})).unwrap();
        assert_eq!(super::read::<serde_json::Value>(&marker).unwrap()["clean"],true);
        assert_eq!(std::fs::read(&prefs).unwrap(),original);
        std::fs::remove_file(marker).unwrap();std::fs::remove_file(prefs).unwrap();std::fs::remove_dir(dir).unwrap();
    }
}
