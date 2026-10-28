//! Game Bar presentation bridge. It reads no game memory and sends no game input.
use crate::{
    spike_appearance::Appearance,
    spike_screen_mode::{self, ScreenMode},
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};

const PACKAGE: &str = "Katsu.Tracker.SpikeWidget";
const GAME_BAR: &str = "Microsoft.XboxGamingOverlay";
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct OutputStatus {
    enabled: bool,
    mode: ScreenMode,
    output: String,
    game_bar_installed: bool,
    widget_installed: bool,
    pinned: bool,
    click_through: bool,
    widget_visible: bool,
    game_bar_open: bool,
    phase: String,
    error: Option<String>,
}
impl Default for OutputStatus {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: ScreenMode::Unknown,
            output: "desktop".into(),
            game_bar_installed: false,
            widget_installed: false,
            pinned: false,
            click_through: false,
            widget_visible: false,
            game_bar_open: false,
            phase: "checking".into(),
            error: None,
        }
    }
}
#[derive(Default)]
struct Inner {
    status: OutputStatus,
    family: Option<String>,
    enabled: bool,
    locale: String,
    presentation_visible: bool,
    manual_until: Option<Instant>,
    last_visibility: Option<(bool, Instant)>,
}
pub struct GameBar {
    inner: Mutex<Inner>,
    busy: AtomicBool,
    stop: Arc<AtomicBool>,
}
#[derive(Deserialize)]
struct WidgetStatus {
    schema: u32,
    pinned: bool,
    click_through: bool,
    heartbeat_ticks: f64,
    #[serde(default)]
    visible: bool,
    #[serde(default)]
    display_mode: Option<i32>,
}

pub fn use_game_bar(mode: ScreenMode, demo: bool) -> bool {
    mode == ScreenMode::Fullscreen && !demo
}
fn counters() -> (i64, i64) {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::Performance::{
            QueryPerformanceCounter, QueryPerformanceFrequency,
        };
        let (mut ticks, mut frequency) = (0, 1);
        let _ = QueryPerformanceCounter(&mut ticks);
        let _ = QueryPerformanceFrequency(&mut frequency);
        return (ticks, frequency.max(1));
    }
    #[cfg(not(windows))]
    (0, 1)
}
fn local_state(family: &str) -> Option<PathBuf> {
    if family.len() > 180
        || !family
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
    {
        return None;
    }
    std::env::var_os("LOCALAPPDATA").map(|p| {
        PathBuf::from(p)
            .join("Packages")
            .join(family)
            .join("LocalState")
    })
}
fn heartbeat(family: &str) -> Option<WidgetStatus> {
    let Some(path) = local_state(family).map(|p| p.join("widget-status.json")) else {
        return None;
    };
    let value = std::fs::metadata(&path)
        .ok()
        .filter(|m| m.len() < 8192)
        .and_then(|_| std::fs::read(path).ok())
        .and_then(|b| serde_json::from_slice::<WidgetStatus>(&b).ok());
    let (now, hz) = counters();
    value.filter(|v| {
        v.schema == 1
            && v.heartbeat_ticks.is_finite()
            && v.heartbeat_ticks <= now as f64
            && now as f64 - v.heartbeat_ticks < (hz * 4) as f64
    })
}
#[cfg(windows)]
fn installed() -> Result<(bool, Option<String>), String> {
    use windows::{core::HSTRING, Management::Deployment::PackageManager};
    let manager = PackageManager::new().map_err(|_| "package_query")?;
    let packages = manager
        .FindPackagesByUserSecurityId(&HSTRING::new())
        .map_err(|_| "package_query")?;
    let mut game_bar = false;
    let mut family = None;
    for package in packages {
        let Ok(id) = package.Id() else {
            continue;
        };
        let Ok(name) = id.Name() else {
            continue;
        };
        if name == GAME_BAR {
            game_bar = true;
        }
        if name == PACKAGE && id.Publisher().is_ok_and(|p| p == "CN=katsu") {
            family = id.FamilyName().ok().map(|s| s.to_string());
        }
    }
    Ok((game_bar, family))
}
#[cfg(not(windows))]
fn installed() -> Result<(bool, Option<String>), String> {
    Ok((false, None))
}

fn emit(app: &tauri::AppHandle) {
    let status = app.state::<GameBar>().inner.lock().status.clone();
    tracing::info!("[Spike output] mode={:?} output={} game_bar={} widget={} pinned={} click_through={} phase={} error={:?}",status.mode,status.output,status.game_bar_installed,status.widget_installed,status.pinned,status.click_through,status.phase,status.error);
    let _ = app.emit("spike-output-status", status);
}
fn phase(app: &tauri::AppHandle, name: &str, error: Option<String>) {
    let state = app.state::<GameBar>();
    {
        let mut i = state.inner.lock();
        i.status.phase = name.into();
        i.status.error = error;
    }
    emit(app);
}
pub fn setup(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let stop = Arc::new(AtomicBool::new(false));
    app.manage(GameBar {
        inner: Mutex::new(Inner::default()),
        busy: AtomicBool::new(false),
        stop: stop.clone(),
    });
    let app = app.clone();
    std::thread::Builder::new()
        .name("spike-output".into())
        .spawn(move || {
            #[cfg(windows)]
            unsafe {
                let _ = windows::Win32::System::WinRT::RoInitialize(
                    windows::Win32::System::WinRT::RO_INIT_MULTITHREADED,
                );
            }
            let mut inventory = Instant::now() - Duration::from_secs(10);
            while !stop.load(Ordering::Relaxed) {
                let puuid = app
                    .state::<crate::state::AppState>()
                    .api
                    .puuid
                    .read()
                    .clone();
                let mode = spike_screen_mode::config_root().map_or(ScreenMode::Unknown, |root| {
                    spike_screen_mode::selected(&root, &puuid)
                });
                let state = app.state::<GameBar>();
                let before = state.inner.lock().status.clone();
                if inventory.elapsed() > Duration::from_secs(5) {
                    if let Ok((present, family)) = installed() {
                        let mut i = state.inner.lock();
                        i.status.game_bar_installed = present;
                        i.status.widget_installed = family.is_some();
                        i.family = family;
                        // An external local-test installation can resolve a previous error.
                        if i.status.game_bar_installed && i.status.widget_installed {
                            i.status.error = None;
                        }
                    }
                    inventory = Instant::now();
                }
                {
                    let mut i = state.inner.lock();
                    i.status.enabled = i.enabled;
                    i.status.mode = mode;
                    i.status.output = if use_game_bar(mode, false) {
                        "game_bar"
                    } else {
                        "desktop"
                    }
                    .into();
                    let health = i.family.as_deref().and_then(heartbeat);
                    if let Some(health) = health.as_ref() {
                        i.status.pinned = health.pinned;
                        i.status.click_through = health.click_through;
                    }
                    i.status.widget_visible = health.as_ref().is_some_and(|h| h.visible);
                    #[cfg(windows)]
                    {
                        i.status.game_bar_open = windows::Gaming::UI::GameBar::Visible()
                            .unwrap_or(false)
                            || health.as_ref().is_some_and(|h| h.display_mode == Some(0));
                    }
                    if !state.busy.load(Ordering::Relaxed) && i.status.error.is_none() {
                        i.status.phase = if !i.status.game_bar_installed {
                            "game_bar_missing"
                        } else if !i.status.widget_installed {
                            "widget_missing"
                        } else if !i.status.pinned {
                            "pin_required"
                        } else {
                            "ready"
                        }
                        .into();
                    }
                }
                if before != state.inner.lock().status {
                    emit(&app);
                }
                let command = {
                    let mut i = state.inner.lock();
                    let now = Instant::now();
                    if i.status.game_bar_open {
                        i.manual_until = None;
                    }
                    let manual = i.manual_until.is_some_and(|until| now < until);
                    let wanted = i.presentation_visible || i.status.game_bar_open || manual;
                    let observed = i.family.as_deref().and_then(heartbeat).map(|h| h.visible);
                    if i.status.widget_installed
                        && i.status.game_bar_installed
                        && window_request(&mut i.last_visibility, wanted, observed, now)
                    {
                        i.family.clone().map(|family| {
                            format!(
                                "ms-gamebar://launch/{}/{}_App_Spike",
                                if wanted { "restore" } else { "minimize" },
                                family
                            )
                        })
                    } else {
                        None
                    }
                };
                if let Some(uri) = command {
                    if let Err(error) = launch_uri(uri) {
                        tracing::warn!("[Spike output] Widget visibility request failed: {error}");
                    }
                }
                std::thread::sleep(Duration::from_millis(500));
            }
        })?;
    Ok(())
}

/// Returns true when the desktop overlay must stand down. Only presentation changes.
pub fn sync(
    app: &tauri::AppHandle,
    enabled: bool,
    remaining: Option<f64>,
    demo: bool,
    game: Option<crate::spike_display::MonitorBounds>,
    scale: f64,
    appearance: &Appearance,
    icon_status: &str,
    icon_score: f32,
) -> bool {
    let state = app.state::<GameBar>();
    let (family, route, locale, configuring) = {
        let mut i = state.inner.lock();
        i.enabled = enabled;
        (
            i.family.clone(),
            use_game_bar(i.status.mode, demo),
            i.locale.clone(),
            i.manual_until.is_some_and(|until| Instant::now() < until),
        )
    };
    if let Some(dir) = family.as_deref().and_then(local_state) {
        let (now, hz) = counters();
        let remaining = remaining.filter(|r| r.is_finite() && *r > 0.0 && *r <= 45.0);
        let visible = enabled && route && game.is_some() && remaining.is_some();
        state.inner.lock().presentation_visible = visible;
        let size = game
            .map(|g| (g.width as f64 / scale, g.height as f64 / scale))
            .unwrap_or((1920.0, 1080.0));
        let payload = serde_json::json!({"schema":1,"configuring":configuring,"enabled":enabled,"game_found":game.is_some(),"detector_status":icon_status,"detector_score":icon_score,"timer_active":remaining.is_some(),"visible":visible,"fullscreen":enabled&&route&&game.is_some(),"locale":locale,"heartbeat_ticks":now,"expires_ticks":now as f64+remaining.unwrap_or(0.0)*hz as f64,"width":size.0,"height":size.1,"font_size":appearance.game_bar.font_size,"background_opacity":appearance.game_bar.background_opacity});
        if std::fs::create_dir_all(&dir).is_ok() {
            let temp = dir.join("tracker-state.json.tmp");
            if let Ok(bytes) = serde_json::to_vec(&payload) {
                if std::fs::write(&temp, bytes).is_ok() {
                    let _ = std::fs::rename(temp, dir.join("tracker-state.json"));
                }
            }
        }
    }
    route
}
pub fn shutdown(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<GameBar>() {
        state.stop.store(true, Ordering::Relaxed);
        let family = state.inner.lock().family.clone();
        if let Some(path) = family
            .as_deref()
            .and_then(local_state)
            .map(|p| p.join("tracker-state.json"))
        {
            let _ = std::fs::write(path, b"{\"schema\":1,\"visible\":false}");
        }
    }
}

fn prepare(app: &tauri::AppHandle, trust: bool) -> Result<(), String> {
    // Consent covers downloads and local publisher registration. Configuration
    // discovery and timer activation alone must never install software.
    if !trust {
        return Err("installation_consent_required".into());
    }
    let state = app.state::<GameBar>();
    let (present, widget) = installed()?;
    if !present {
        phase(app, "installing_game_bar", None);
        let winget = std::env::var_os("LOCALAPPDATA")
            .map(|p| PathBuf::from(p).join("Microsoft/WindowsApps/winget.exe"))
            .filter(|p| p.is_file())
            .ok_or("store_required")?;
        let mut command = std::process::Command::new(winget);
        command.args([
            "install",
            "--id",
            "9NZKPSTSNW4P",
            "--source",
            "msstore",
            "--exact",
            "--silent",
            "--accept-source-agreements",
            "--accept-package-agreements",
            "--disable-interactivity",
        ]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        command
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        let status = command.status().map_err(|_| "store_required")?;
        if !status.success() && !installed()?.0 {
            return Err("game_bar_install".into());
        }
    }
    if widget.is_none() {
        phase(app, "installing_widget", None);
        let directory = app
            .path()
            .resource_dir()
            .map_err(|_| "widget_package_missing")?
            .join("gamebar");
        let installer = directory.join("install_widget.ps1");
        let package = directory.join("KatsuSpikeWidget.msix");
        let certificate = directory.join("WidgetPublisher.cer");
        if !installer.is_file() || !package.is_file() || !certificate.is_file() {
            return Err("widget_package_missing".into());
        }
        // The public certificate and helper must match this compiled build.
        // Never elevate a substituted script or trust an arbitrary bundled cert.
        if std::fs::read(&certificate).ok().as_deref()
            != Some(include_bytes!("../../gamebar/build/WidgetPublisher.cer").as_slice())
            || std::fs::read(&installer).ok().as_deref()
                != Some(include_bytes!("../../gamebar/install_widget.ps1").as_slice())
        {
            return Err("widget_package_invalid".into());
        }
        let shell = std::env::var_os("SystemRoot")
            .map(PathBuf::from)
            .map(|p| p.join("System32/WindowsPowerShell/v1.0/powershell.exe"))
            .filter(|p| p.is_file())
            .ok_or("widget_install")?;
        let mut command = std::process::Command::new(shell);
        command
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(installer)
            .arg("-PackagePath")
            .arg(package)
            .arg("-CertificatePath")
            .arg(certificate)
            .arg("-ExpectedCertificateThumbprint")
            .arg(include_str!("../../gamebar/build/WidgetPublisher.thumbprint").trim());
        if trust {
            command.arg("-TrustMachinePublisher");
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let result = command
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|_| "widget_install")?;
        if result.code() == Some(3) {
            return Err("publisher_trust_required".into());
        }
        if result.code() == Some(4) {
            return Err("administrator_required".into());
        }
        if result.code() == Some(5) {
            return Err("setup_cancelled".into());
        }
        if !result.success() {
            tracing::warn!(
                "[Spike output] Widget installation exited with {:?}",
                result.code()
            );
            return Err("widget_install".into());
        }
    }
    let (present, family) = installed()?;
    {
        let mut i = state.inner.lock();
        i.status.game_bar_installed = present;
        i.status.widget_installed = family.is_some();
        i.family = family;
    }
    phase(app, "pin_required", None);
    Ok(())
}
fn begin_prepare(app: tauri::AppHandle, trust: bool) {
    let state = app.state::<GameBar>();
    if state.busy.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        #[cfg(windows)]
        unsafe {
            let _ = windows::Win32::System::WinRT::RoInitialize(
                windows::Win32::System::WinRT::RO_INIT_MULTITHREADED,
            );
        }
        if let Err(error) = prepare(&app, trust) {
            phase(&app, "error", Some(error));
        }
        app.state::<GameBar>().busy.store(false, Ordering::SeqCst);
    });
}
#[tauri::command]
pub fn get_spike_output_status(app: tauri::AppHandle, locale: Option<String>) -> OutputStatus {
    let state = app.state::<GameBar>();
    let mut inner = state.inner.lock();
    inner.locale = if locale.as_deref() == Some("tr") {
        "tr"
    } else {
        "en"
    }
    .into();
    inner.status.clone()
}
#[tauri::command]
pub fn prepare_spike_game_bar(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    trust_publisher: Option<bool>,
) -> Result<(), String> {
    if window.label() != "main" {
        return Err("window".into());
    }
    begin_prepare(app, trust_publisher.unwrap_or(false));
    Ok(())
}
#[tauri::command]
pub async fn open_spike_game_bar(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    store: Option<bool>,
) -> Result<(), String> {
    if window.label() != "main" {
        return Err("window".into());
    }
    let family = app.state::<GameBar>().inner.lock().family.clone();
    let uri = if store.unwrap_or(false) {
        "ms-windows-store://pdp/?productid=9NZKPSTSNW4P".into()
    } else {
        family.map_or("ms-gamebar:".into(), |f| {
            format!("ms-gamebar://launch/activate/{f}_App_Spike")
        })
    };
    if !store.unwrap_or(false) {
        app.state::<GameBar>().inner.lock().manual_until =
            Some(Instant::now() + Duration::from_secs(15));
    }
    tauri::async_runtime::spawn_blocking(move || launch_uri(uri))
        .await
        .map_err(|_| "game_bar_launch")?
}

fn window_request(
    last: &mut Option<(bool, Instant)>,
    wanted: bool,
    observed: Option<bool>,
    now: Instant,
) -> bool {
    if observed == Some(wanted) || (!wanted && observed.is_none()) {
        if last.is_none_or(|(previous, _)| previous != wanted) {
            *last = Some((wanted, now));
        }
        return false;
    }
    if last.is_none_or(|(previous, at)| {
        previous != wanted || now.duration_since(at) >= Duration::from_secs(2)
    }) {
        *last = Some((wanted, now));
        return true;
    }
    false
}
fn launch_uri(uri: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows::Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED};
        unsafe {
            RoInitialize(RO_INIT_MULTITHREADED).map_err(|_| "game_bar_launch")?;
        }
        struct Apartment;
        impl Drop for Apartment {
            fn drop(&mut self) {
                unsafe {
                    RoUninitialize();
                }
            }
        }
        let _apartment = Apartment;
        let uri = windows::Foundation::Uri::CreateUri(&windows::core::HSTRING::from(uri))
            .map_err(|_| "game_bar_launch")?;
        let opened = windows::System::Launcher::LaunchUriAsync(&uri)
            .map_err(|_| "game_bar_launch")?
            .get()
            .map_err(|_| "game_bar_launch")?;
        if opened {
            Ok(())
        } else {
            Err("game_bar_launch".into())
        }
    }
    #[cfg(not(windows))]
    {
        let _ = uri;
        Err("unsupported".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn idle_widget_is_hidden_and_restored_only_when_needed() {
        let now = Instant::now();
        let mut last = None;
        assert!(
            !window_request(&mut last, false, None, now),
            "do not launch an inactive widget just to hide it"
        );
        assert!(window_request(&mut last, true, Some(false), now));
        assert!(
            !window_request(
                &mut last,
                true,
                Some(false),
                now + Duration::from_millis(100)
            ),
            "no URI spam"
        );
        assert!(!window_request(
            &mut last,
            true,
            Some(true),
            now + Duration::from_secs(1)
        ));
        assert!(
            window_request(&mut last, false, Some(true), now + Duration::from_secs(2)),
            "remove the actual window when countdown ends"
        );
        assert!(!window_request(
            &mut last,
            false,
            Some(false),
            now + Duration::from_secs(3)
        ));
        assert!(
            window_request(&mut last, true, Some(false), now + Duration::from_secs(4)),
            "next countdown restores it"
        );
    }
    #[cfg(windows)]
    #[test]
    #[ignore = "local integration check; requires the owner-approved Game Bar widget"]
    fn live_widget_restore_and_minimize() {
        use windows::Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED};
        unsafe {
            RoInitialize(RO_INIT_MULTITHREADED).unwrap();
        }
        struct Apartment;
        impl Drop for Apartment {
            fn drop(&mut self) {
                unsafe {
                    RoUninitialize();
                }
            }
        }
        let _apartment = Apartment;
        let family = installed()
            .unwrap()
            .1
            .expect("local widget must be installed");
        let path = local_state(&family).unwrap().join("tracker-state.json");
        struct Reset {
            path: PathBuf,
            family: String,
            original: Option<Vec<u8>>,
        }
        impl Drop for Reset {
            fn drop(&mut self) {
                let _ = std::fs::write(
                    &self.path,
                    self.original
                        .as_deref()
                        .unwrap_or(br#"{"schema":1,"visible":false}"#),
                );
                let _ = launch_uri(format!(
                    "ms-gamebar://launch/minimize/{}_App_Spike",
                    self.family
                ));
            }
        }
        let _reset = Reset {
            original: std::fs::read(&path).ok(),
            path: path.clone(),
            family: family.clone(),
        };
        let (now, hz) = counters();
        let mut data = serde_json::json!({"schema":1,"visible":true,"fullscreen":true,"heartbeat_ticks":now,"expires_ticks":now+hz*15,"font_size":31,"background_opacity":0.8});
        std::fs::write(&path, serde_json::to_vec(&data).unwrap()).unwrap();
        launch_uri(format!("ms-gamebar://launch/restore/{family}_App_Spike")).unwrap();
        let wait = |visible| {
            for _ in 0..20 {
                if heartbeat(&family).is_some_and(|h| h.visible == visible) {
                    return true;
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            false
        };
        assert!(
            wait(true),
            "the native renderer must restore its actual widget window"
        );
        data["visible"] = serde_json::Value::Bool(false);
        data["heartbeat_ticks"] = serde_json::Value::from(counters().0);
        std::fs::write(&path, serde_json::to_vec(&data).unwrap()).unwrap();
        launch_uri(format!("ms-gamebar://launch/minimize/{family}_App_Spike")).unwrap();
        assert!(
            wait(false),
            "the idle window must be minimized, not just transparent"
        );
    }
    #[test]
    fn only_fullscreen_routes_to_game_bar() {
        assert!(use_game_bar(ScreenMode::Fullscreen, false));
        for mode in [
            ScreenMode::Borderless,
            ScreenMode::Windowed,
            ScreenMode::Unknown,
        ] {
            assert!(!use_game_bar(mode, false));
        }
        assert!(!use_game_bar(ScreenMode::Fullscreen, true));
    }
}
