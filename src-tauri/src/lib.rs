mod media;
mod updates;
mod update_worker;
mod spike_appearance;
mod agent_catalog;
mod api;
mod spike;
// Audio capture is available only to historical tests, never to the shipped application.
#[cfg(test)]
mod spike_audio;
mod spike_signal;
mod spike_sequence;
mod spike_correlation;
#[allow(dead_code)]
mod spike_signal_legacy;
mod spike_recording;
mod spike_round;
mod spike_display;
mod spike_icon;
mod spike_session;
mod commands;
mod translate;
mod constants;
mod discord;
mod logger;
mod presets;
mod process;
mod single_instance;
mod state;
mod last_match;
mod seen_names;
mod party;
mod rank;
mod tracker;

use single_instance::{SingleInstanceGuard, SingleInstanceResult};
use state::AppState;
use std::sync::Arc;
use tauri::Manager;

/// Opt-in troubleshooting mode: no Riot session or API traffic.
pub fn riot_offline_diagnostic() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::args().any(|arg| arg == "--riot-offline-diagnostic"))
}

/// Turn off WebView2 browser chrome shortcuts (Ctrl+F find, Ctrl+P print, …).
#[cfg(windows)]
fn disable_browser_accelerator_keys(webview: &tauri::webview::PlatformWebview) {
    use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Settings3;
    use windows_core::Interface;

    let controller = webview.controller();
    unsafe {
        let Ok(core) = controller.CoreWebView2() else {
            tracing::warn!("[WebView] CoreWebView2 unavailable; Ctrl+F still enabled");
            return;
        };
        let Ok(settings) = core.Settings() else {
            tracing::warn!("[WebView] Settings unavailable; Ctrl+F still enabled");
            return;
        };
        let Ok(settings3) = settings.cast::<ICoreWebView2Settings3>() else {
            tracing::warn!("[WebView] Settings3 unavailable; Ctrl+F still enabled");
            return;
        };
        if let Err(e) = settings3.SetAreBrowserAcceleratorKeysEnabled(false) {
            tracing::warn!("[WebView] Failed to disable accelerator keys: {e}");
        } else {
            tracing::info!("[WebView] Browser accelerator keys disabled (Ctrl+F / Ctrl+P / …)");
        }
    }
}

/// Flip the WebView2 controller's visibility. While invisible it stops
/// painting and throttles timers; the native window hide/minimize alone
/// leaves it rendering in the background.
pub fn set_webview_visible(window: &tauri::WebviewWindow, visible: bool) {
    #[cfg(windows)]
    {
        let _ = window.with_webview(move |webview| unsafe {
            if let Err(e) = webview.controller().SetIsVisible(visible) {
                tracing::warn!("[WebView] SetIsVisible({visible}) failed: {e}");
            }
        });
    }
    #[cfg(not(windows))]
    let _ = (window, visible);
}

/// Result of attempting to run the application
pub enum RunResult {
    /// Application ran normally and exited
    Completed,
    /// Another instance is already running
    AlreadyRunning,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> RunResult {
    if update_worker::run_if_requested() { return RunResult::Completed; }
    // Try to acquire single-instance lock before doing anything else
    let guard = match single_instance::try_acquire_single_instance() {
        SingleInstanceResult::Primary(guard) => guard,
        SingleInstanceResult::Secondary => {
            // Another instance is already running, we've signaled it to show overlay
            // Exit silently
            return RunResult::AlreadyRunning;
        }
        SingleInstanceResult::Error(e) => {
            eprintln!("Single instance check failed: {}", e);
            // Continue anyway - better to run than to fail completely
            // Create a dummy guard (won't hold mutex)
            SingleInstanceGuard::dummy()
        }
    };

    let context = tauri::generate_context!();

    // Get the shutdown flag before we start (guard is Send-safe now)
    let shutdown_flag = guard.shutdown_flag();

    // Keep guard alive for the duration of the app
    let _guard = Arc::new(guard);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(updates::Updates::default())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            tracker::tracker_load,
            tracker::tracker_refresh,
            updates::get_update_status,
            updates::check_for_updates,
            updates::apply_update,
            spike_appearance::get_spike_appearance,
            spike_appearance::set_spike_appearance,
            spike_appearance::get_spike_editor_context,
            media::media_status,
            media::media_control,
            spike::spike_get_status,
            spike::spike_set_enabled,
            spike::spike_reset,
            spike::spike_demo,
            commands::get_app_constants,
            commands::initialize,
            commands::get_game_state,
            commands::set_auto_lock,
            commands::get_auto_lock,
            commands::set_auto_lock_delay,
            commands::get_auto_lock_delay,
            commands::set_map_preferences,
            commands::pause_watching,
            commands::resume_watching,
            commands::reconnect,
            commands::set_discord_rpc,
            commands::get_discord_rpc,
            commands::get_connection_status,
            commands::get_player_loadout,
            commands::get_chat_messages,
            commands::get_active_conversations,
            commands::send_message,
            commands::get_paginated_chat_messages,
            commands::get_friends,
            commands::get_outgoing_friend_requests,
            commands::send_friend_request,
            commands::cancel_friend_request,
            commands::get_dm_cid,
            commands::get_cached_image,
            commands::get_peak_rank,
            commands::get_frequent_teammates,
            commands::get_storefront,
            commands::get_wallet,
            commands::get_player_settings,
            commands::capture_player_settings,
            commands::list_presets,
            commands::delete_preset,
            commands::apply_preset,
            commands::rename_preset,
            commands::duplicate_preset,
            commands::arm_preset,
            commands::close_riot_and_arm_preset,
            commands::disarm_preset,
            commands::get_armed_preset,
            commands::get_game_running,
            commands::get_preset_crosshairs,
            // License commands
            commands::get_machine_id,
            commands::get_license_request_data,
            commands::get_activation_code,
            commands::check_license,
            commands::import_license,
            commands::get_license_info,
            commands::reset_license,
            commands::minimize_window,
            commands::close_window,
            commands::set_webview_visible,
            commands::set_always_on_top,
            commands::focus_window,
            commands::open_log_file,
            commands::log_frontend_message,
            commands::translate_text,
            commands::get_install_count,
            commands::get_ready_update,
            commands::restart_to_update,
            last_match::get_last_match,
        ])
        .setup(move |app| {
            // Initialize logger first
            let log_dir = app.path().app_log_dir().expect("Failed to get log dir");
            logger::init_logger(log_dir);

            // Initialize the settings-preset store from the app data dir.
            if let Ok(data_dir) = app.path().app_data_dir() {
                let store = presets::PresetStore::load(presets::presets_path(&data_dir));
                *app.state::<AppState>().presets.write() = Some(Arc::new(store));
                crate::seen_names::init(&data_dir);
            } else {
                tracing::error!("[Presets] Could not resolve app_data_dir; presets disabled");
                crate::seen_names::init(&std::env::temp_dir());
            }

            spike_appearance::setup(app.handle())?;
            spike::setup(app.handle())?;
            media::setup(app.handle())?;
            tracker::setup(app.handle())?;
            updates::start(app.handle().clone());

            // Personal build: no telemetry or install counter.

            // Start the named pipe server to listen for signals from other instances
            single_instance::start_pipe_server(app.handle().clone(), shutdown_flag.clone());

            // Start the connection supervisor: it owns the whole connection
            // lifecycle (connect, watch, self-reconnect, autolock) and emits
            // `connection_changed` / `game_state_changed` events to the frontend.
            commands::start_supervisor(app.handle().clone());


            // WebView2 treats Ctrl+F / Ctrl+P / F3 as browser chrome. The
            // tauri.conf `browserAcceleratorKeys` field isn't in 2.8/2.9
            // schema, so flip the setting on the live controller instead.
            #[cfg(windows)]
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.with_webview(|webview| {
                    disable_browser_accelerator_keys(&webview);
                });
            }

            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" { return; }
            if window.label() == "main" && matches!(event, tauri::WindowEvent::Destroyed) {
                spike::shutdown(window.app_handle());
                window.app_handle().exit(0);
            }
            // Taskbar minimize/restore (non-tray mode) bypasses the frontend.
            if let tauri::WindowEvent::Resized(_) = event {
                if let Some(webview) = window.app_handle().get_webview_window(window.label()) {
                    let minimized = window.is_minimized().unwrap_or(false);
                    let shown = window.is_visible().unwrap_or(true);
                    set_webview_visible(&webview, shown && !minimized);
                }
            }
        })
        .build(context)
        .expect("error while building tauri application")
        .run(|app, event| { if matches!(event, tauri::RunEvent::Exit) { spike::shutdown(app); } });

    RunResult::Completed
}
