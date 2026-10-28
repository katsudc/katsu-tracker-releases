//! A copy of this signed updater client keeps the progress window alive while
//! NSIS replaces the installed application. It never starts the game services.
use crate::updates::{self, Package, UpdateStatus, Updates};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::Manager;

#[derive(Serialize, Deserialize)]
struct Job {
    token: String,
    created: u64,
    parent: u32,
    application: PathBuf,
    package: Package,
    locale: String,
    theme: String,
}
struct Worker {
    job: Job,
    directory: PathBuf,
    busy: AtomicBool,
    installed: AtomicBool,
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn hidden_command(path: &Path) -> Command {
    let mut command = Command::new(path);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}
fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    std::fs::write(path, serde_json::to_vec(value).map_err(|_| "storage")?)
        .map_err(|_| "storage".into())
}
fn read_small(path: &Path) -> Result<Vec<u8>, String> {
    if std::fs::metadata(path).map_err(|_| "storage")?.len() > 65536 {
        return Err("invalid_job".into());
    }
    std::fs::read(path).map_err(|_| "storage".into())
}
fn validate_job(directory: &Path, token: &str, current: &str) -> Result<Job, String> {
    let id = uuid::Uuid::parse_str(token).map_err(|_| "invalid_job")?;
    if id.to_string() != token
        || directory.file_name().and_then(|n| n.to_str()) != Some(&format!("worker-{token}"))
        || directory
            .parent()
            .and_then(Path::file_name)
            .and_then(|n| n.to_str())
            != Some("signed-update")
    {
        return Err("invalid_job".into());
    }
    let job: Job = serde_json::from_slice(&read_small(&directory.join("job.json"))?)
        .map_err(|_| "invalid_job")?;
    if job.token != token
        || job.parent == 0
        || job.parent == std::process::id()
        || now().saturating_sub(job.created) > 600
        || job.created > now() + 30
        || !updates::valid_release(&job.package.version, &job.package.url, current)
    {
        return Err("invalid_job".into());
    }
    // A job can replace only the application from which this helper was copied.
    let exe = std::env::current_exe().map_err(|_| "storage")?;
    if job.application.file_name().and_then(|s| s.to_str()) != Some("valorant-tracker.exe")
        || job.application == exe
        || !job.application.is_absolute()
    {
        return Err("invalid_job".into());
    }
    let target_size = std::fs::metadata(&job.application)
        .map_err(|_| "storage")?
        .len();
    if target_size > updates::MAX_PACKAGE
        || target_size != std::fs::metadata(&exe).map_err(|_| "storage")?.len()
        || std::fs::read(&job.application).map_err(|_| "storage")?
            != std::fs::read(exe).map_err(|_| "storage")?
    {
        return Err("invalid_job".into());
    }
    let installer = directory.join("update.exe");
    if std::fs::metadata(&installer).map_err(|_| "storage")?.len() > updates::MAX_PACKAGE {
        return Err("package_size".into());
    }
    updates::verify(
        &std::fs::read(installer).map_err(|_| "storage")?,
        &job.package.signature,
    )?;
    Ok(job)
}
pub async fn handoff(
    app: &tauri::AppHandle,
    package: Package,
    bytes: Vec<u8>,
) -> Result<(), String> {
    let token = uuid::Uuid::new_v4().to_string();
    let directory = updates::cache_dir(app)?.join(format!("worker-{token}"));
    std::fs::create_dir_all(&directory).map_err(|_| "storage")?;
    let application = std::env::current_exe().map_err(|_| "storage")?;
    let status = app.state::<Updates>().status.lock().clone();
    let job = Job {
        token: token.clone(),
        created: now(),
        parent: std::process::id(),
        application: application.clone(),
        package,
        locale: status.locale,
        theme: status.theme,
    };
    write_json(&directory.join("job.json"), &job)?;
    std::fs::write(directory.join("update.exe"), bytes).map_err(|_| "storage")?;
    let helper = directory.join("KatsuUpdateWorker.exe");
    std::fs::copy(application, &helper).map_err(|_| "storage")?;
    let mut child = hidden_command(&helper)
        .args(["--katsu-update-worker", &token])
        .spawn()
        .map_err(|_| "helper_failed")?;
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(20) {
        if std::fs::read_to_string(directory.join("ready"))
            .ok()
            .as_deref()
            == Some(&token)
        {
            // The helper now owns the progress screen and waits for this PID.
            app.exit(0);
            return Ok(());
        }
        if child.try_wait().map_err(|_| "helper_failed")?.is_some() {
            return Err("helper_failed".into());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let _ = child.kill();
    Err("helper_failed".into())
}
#[cfg(windows)]
fn wait_for_parent(pid: u32) -> Result<(), String> {
    use windows::Win32::{
        Foundation::{CloseHandle, WAIT_OBJECT_0},
        System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE},
    };
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) else {
            return Ok(());
        };
        let result = WaitForSingleObject(handle, 60000);
        let _ = CloseHandle(handle);
        if result == WAIT_OBJECT_0 {
            Ok(())
        } else {
            Err("close_failed".into())
        }
    }
}
#[cfg(not(windows))]
fn wait_for_parent(_: u32) -> Result<(), String> {
    Err("unsupported".into())
}

/// Shared by the worker and the opt-in harmless installer fixture test.
fn run_installer(installer: &Path, destination: &Path) -> Result<(), String> {
    // /D must be last. /S hides the wizard; only the worker owns restart.
    let mut command = hidden_command(installer);
    command.args(["/S", "/UPDATE", "/NS"]);
    // NSIS parses the remainder after /D= itself and requires it unquoted,
    // including destinations containing spaces. No command shell is involved.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.raw_arg(format!("/D={}", destination.display()));
    }
    #[cfg(not(windows))]
    command.arg(format!("/D={}", destination.display()));
    let result = command.status().map_err(|_| "install_failed")?;
    if result.success() {
        Ok(())
    } else {
        Err("install_failed".into())
    }
}

fn install_and_restart(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<Worker>();
    wait_for_parent(state.job.parent)?;
    if !state.installed.load(Ordering::SeqCst) {
        if crate::process::is_game_running() {
            return Err("game_running".into());
        }
        updates::publish(app, |s| s.phase = "installing".into());
        let installer = state.directory.join("update.exe");
        let bytes = std::fs::read(&installer).map_err(|_| "storage")?;
        updates::verify(&bytes, &state.job.package.signature)?;
        let destination = state.job.application.parent().ok_or("storage")?;
        run_installer(&installer, destination)?;
        state.installed.store(true, Ordering::SeqCst);
    }
    updates::publish(app, |s| s.phase = "restarting".into());
    let ack = state.directory.join("launched.json");
    let _ = std::fs::remove_file(&ack);
    let mut child = hidden_command(&state.job.application)
        .env("KATSU_UPDATE_LAUNCH_ACK", &ack)
        .env("KATSU_UPDATE_LAUNCH_TOKEN", &state.job.token)
        .spawn()
        .map_err(|_| "restart_failed")?;
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(45) {
        if let Ok(bytes) = read_small(&ack) {
            if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                if value["token"] == state.job.token
                    && value["version"] == state.job.package.version
                {
                    updates::publish(app, |s| s.phase = "complete".into());
                    std::thread::sleep(Duration::from_millis(500));
                    app.exit(0);
                    return Ok(());
                }
            }
        }
        if child.try_wait().map_err(|_| "restart_failed")?.is_some() {
            return Err("restart_failed".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err("restart_failed".into())
}
fn begin(app: tauri::AppHandle) {
    let state = app.state::<Worker>();
    if state.busy.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        let state = app.state::<Worker>();
        if let Err(code) = install_and_restart(&app) {
            updates::fail(&app, &code);
        }
        state.busy.store(false, Ordering::SeqCst);
    });
}
#[tauri::command]
fn update_worker_ready(window: tauri::WebviewWindow, app: tauri::AppHandle) -> Result<(), String> {
    if window.label() != "updater" {
        return Err("window".into());
    }
    let state = app.state::<Worker>();
    std::fs::write(state.directory.join("ready"), &state.job.token).map_err(|_| "storage")?;
    begin(app);
    Ok(())
}
#[tauri::command]
fn retry_update_worker(window: tauri::WebviewWindow, app: tauri::AppHandle) -> Result<(), String> {
    if window.label() != "updater" {
        return Err("window".into());
    }
    begin(app);
    Ok(())
}
#[tauri::command]
fn reopen_after_update(window: tauri::WebviewWindow, app: tauri::AppHandle) -> Result<(), String> {
    if window.label() != "updater" {
        return Err("window".into());
    }
    let worker = app.state::<Worker>();
    if worker.busy.load(Ordering::SeqCst) {
        return Err("busy".into());
    }
    hidden_command(&worker.job.application)
        .spawn()
        .map_err(|_| "restart_failed")?;
    app.exit(0);
    Ok(())
}
pub fn acknowledge_launch(app: &tauri::AppHandle) {
    let path = std::env::var_os("KATSU_UPDATE_LAUNCH_ACK").map(PathBuf::from);
    let token = std::env::var("KATSU_UPDATE_LAUNCH_TOKEN").ok();
    std::env::remove_var("KATSU_UPDATE_LAUNCH_ACK");
    std::env::remove_var("KATSU_UPDATE_LAUNCH_TOKEN");
    if let (Some(path), Some(token), Ok(root)) = (path, token, updates::cache_dir(app)) {
        if uuid::Uuid::parse_str(&token).is_ok()
            && path == root.join(format!("worker-{token}")).join("launched.json")
        {
            let _ = write_json(
                &path,
                &serde_json::json!({"token":token,"version":app.package_info().version.to_string()}),
            );
        }
    }
}
pub fn run_if_requested() -> bool {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() != Some("--katsu-update-worker") {
        return false;
    }
    let Some(token) = args.next() else {
        return true;
    };
    if args.next().is_some() {
        return true;
    }
    let Ok(exe) = std::env::current_exe() else {
        return true;
    };
    let Some(directory) = exe.parent().map(Path::to_path_buf) else {
        return true;
    };
    let Ok(job) = validate_job(&directory, &token, env!("CARGO_PKG_VERSION")) else {
        return true;
    };
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows.clear();
    let initial = UpdateStatus {
        phase: "preparing".into(),
        version: Some(job.package.version.clone()),
        locale: job.locale.clone(),
        theme: job.theme.clone(),
        ..Default::default()
    };
    let updates = Updates::default();
    *updates.status.lock() = initial;
    let worker = Worker {
        job,
        directory: directory.clone(),
        busy: AtomicBool::new(false),
        installed: AtomicBool::new(false),
    };
    let _ = tauri::Builder::default()
        .manage(updates)
        .manage(worker)
        .invoke_handler(tauri::generate_handler![
            updates::get_update_status,
            update_worker_ready,
            retry_update_worker,
            reopen_after_update
        ])
        .setup(move |app| {
            let expected = updates::cache_dir(app.handle())?.join(format!("worker-{token}"));
            if expected.canonicalize()? != directory.canonicalize()? {
                return Err("Invalid update directory".into());
            }
            tauri::WebviewWindowBuilder::new(
                app,
                "updater",
                tauri::WebviewUrl::App("index.html#updater".into()),
            )
            .title("katsu's Tracker · Updating")
            .inner_size(480.0, 320.0)
            .resizable(false)
            .decorations(false)
            .center()
            .data_directory(directory.join("webview"))
            .build()?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window
                    .app_handle()
                    .state::<Worker>()
                    .busy
                    .load(Ordering::SeqCst)
                {
                    api.prevent_close();
                }
            }
        })
        .run(context);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_job_tokens_and_locations_before_launch() {
        for (path, token) in [
            ("C:/temp/anything", "bad"),
            (
                "C:/temp/signed-update/worker-wrong",
                "00112233-4455-4677-8899-aabbccddeeff",
            ),
        ] {
            assert!(validate_job(Path::new(path), token, "1.9.6-personal.36").is_err());
        }
    }
    #[test]
    #[ignore = "opt-in local NSIS fixture; never runs the real tracker installer"]
    fn silent_nsis_fixture() {
        let installer =
            PathBuf::from(std::env::var_os("KATSU_TEST_NSIS_FIXTURE").expect("fixture executable"));
        let destination = installer
            .parent()
            .unwrap()
            .join("target with spaces and apostrophe's");
        assert!(installer.file_name().unwrap() == "harmless-updater-fixture.exe");
        run_installer(&installer, &destination).unwrap();
        assert_eq!(
            std::fs::read_to_string(destination.join("silent-result.txt")).unwrap(),
            "silent=1"
        );
        assert!(!destination.join("unexpected-wizard.txt").exists());
    }
}
