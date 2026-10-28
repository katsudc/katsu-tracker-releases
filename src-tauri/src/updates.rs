//! Signed update discovery, explicit download/install, and UI progress.
use base64::{engine::general_purpose::STANDARD, Engine};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

pub const ENDPOINT: &str =
    "https://github.com/katsudc/katsu-tracker-releases/releases/latest/download/latest.json";
pub const MAX_PACKAGE: u64 = 256 * 1024 * 1024;
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UpdateStatus {
    pub revision: u64,
    pub phase: String,
    pub version: Option<String>,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub error: Option<String>,
    pub cached: bool,
    pub locale: String,
    pub theme: String,
}
impl Default for UpdateStatus {
    fn default() -> Self {
        Self {
            revision: 0,
            phase: "checking".into(),
            version: None,
            downloaded: 0,
            total: None,
            error: None,
            cached: false,
            locale: "en".into(),
            theme: "rose".into(),
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Package {
    pub version: String,
    pub signature: String,
    #[serde(default)]
    pub url: String,
}
#[derive(Default)]
pub struct Updates {
    pub status: Mutex<UpdateStatus>,
    pending: Mutex<Option<Package>>,
    busy: AtomicBool,
}
pub fn publish(app: &tauri::AppHandle, change: impl FnOnce(&mut UpdateStatus)) {
    let status = {
        let state = app.state::<Updates>();
        let mut value = state.status.lock();
        change(&mut value);
        value.revision += 1;
        value.clone()
    };
    let _ = app.emit("update_status", status);
}
pub fn fail(app: &tauri::AppHandle, code: &str) {
    publish(app, |s| {
        s.phase = "error".into();
        s.error = Some(code.into());
    });
}
pub fn public_key() -> Result<String, String> {
    let value: serde_json::Value =
        serde_json::from_str(include_str!("../tauri.conf.json")).map_err(|_| "configuration")?;
    value["plugins"]["updater"]["pubkey"]
        .as_str()
        .map(str::to_owned)
        .ok_or("configuration".into())
}
pub fn verify(bytes: &[u8], signature: &str) -> Result<(), String> {
    let decode = |s: &str| -> Result<String, String> {
        String::from_utf8(STANDARD.decode(s.trim()).map_err(|_| "signature")?)
            .map_err(|_| "signature".into())
    };
    let key =
        minisign_verify::PublicKey::decode(&decode(&public_key()?)?).map_err(|_| "signature")?;
    let sig = minisign_verify::Signature::decode(&decode(signature)?).map_err(|_| "signature")?;
    key.verify(bytes, &sig, true)
        .map_err(|_| "signature".into())
}
pub fn valid_release(version: &str, url: &str, current: &str) -> bool {
    let newer = semver::Version::parse(version)
        .ok()
        .zip(semver::Version::parse(current).ok())
        .is_some_and(|(a, b)| a > b);
    let Ok(url) = reqwest::Url::parse(url) else {
        return false;
    };
    newer&&url.scheme()=="https"&&url.host_str()==Some("github.com")&&url.username().is_empty()&&url.password().is_none()&&url.port().is_none()&&url.query().is_none()&&url.fragment().is_none()
 &&url.path()==format!("/katsudc/katsu-tracker-releases/releases/download/v{version}/Katsu-Tracker-setup.exe")
}
pub fn cache_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_cache_dir()
        .map(|p| p.join("signed-update"))
        .map_err(|_| "storage".into())
}
fn read_metadata(dir: &Path) -> Option<Package> {
    let path = dir.join("pending.json");
    if std::fs::metadata(&path).ok()?.len() > 65536 {
        return None;
    }
    let mut p: Package = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    if p.url.is_empty() {
        p.url=format!("https://github.com/katsudc/katsu-tracker-releases/releases/download/v{}/Katsu-Tracker-setup.exe",p.version);
    }
    Some(p)
}
fn cached_bytes(dir: &Path, package: &Package) -> Option<Vec<u8>> {
    let meta = read_metadata(dir)?;
    if meta.version != package.version || meta.signature != package.signature {
        return None;
    }
    let path = dir.join("pending.exe");
    if std::fs::metadata(&path).ok()?.len() > MAX_PACKAGE {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    verify(&bytes, &package.signature).ok()?;
    Some(bytes)
}
fn save(dir: &Path, package: &Package, bytes: &[u8]) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|_| "storage")?;
    std::fs::write(dir.join("pending.exe.part"), bytes)
        .and_then(|_| std::fs::rename(dir.join("pending.exe.part"), dir.join("pending.exe")))
        .map_err(|_| "storage")?;
    std::fs::write(
        dir.join("pending.json.part"),
        serde_json::to_vec(package).map_err(|_| "storage")?,
    )
    .and_then(|_| std::fs::rename(dir.join("pending.json.part"), dir.join("pending.json")))
    .map_err(|_| "storage".into())
}
#[tauri::command]
pub fn get_update_status(state: tauri::State<'_, Updates>) -> UpdateStatus {
    state.status.lock().clone()
}
pub fn ready(app: &tauri::AppHandle) -> Option<String> {
    app.state::<Updates>()
        .pending
        .lock()
        .as_ref()
        .map(|p| p.version.clone())
}

async fn check(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<Updates>();
    if state.busy.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    struct Release<'a>(&'a AtomicBool);
    impl Drop for Release<'_> {
        fn drop(&mut self) {
            self.0.store(false, Ordering::SeqCst);
        }
    }
    let _release = Release(&state.busy);
    if matches!(
        state.status.lock().phase.as_str(),
        "downloading" | "verifying" | "preparing" | "installing" | "restarting"
    ) {
        return Ok(());
    }
    publish(app, |s| {
        s.phase = "checking".into();
        s.error = None;
    });
    let result: Result<(), String> = async {
        let updater = app
            .updater_builder()
            .timeout(Duration::from_secs(25))
            .build()
            .map_err(|_| "check_failed")?;
        let update = updater.check().await.map_err(|_| "check_failed")?;
        let Some(update) = update else {
            *state.pending.lock() = None;
            publish(app, |s| {
                s.phase = "up_to_date".into();
                s.version = None;
                s.cached = false;
            });
            return Ok(());
        };
        let package = Package {
            version: update.version,
            signature: update.signature,
            url: update.download_url.to_string(),
        };
        if !valid_release(
            &package.version,
            &package.url,
            &app.package_info().version.to_string(),
        ) {
            return Err("invalid_release".into());
        }
        let cached = cache_dir(app)
            .ok()
            .and_then(|dir| cached_bytes(&dir, &package))
            .is_some();
        *state.pending.lock() = Some(package.clone());
        publish(app, |s| {
            s.phase = "available".into();
            s.version = Some(package.version.clone());
            s.cached = cached;
            s.downloaded = 0;
            s.total = None;
        });
        let _ = app.emit("update_ready", package.version);
        Ok(())
    }
    .await;
    if let Err(code) = result {
        // A verified package from an earlier run remains usable when discovery is offline.
        let recovered = cache_dir(app).ok().and_then(|dir| {
            let p = read_metadata(&dir)?;
            if valid_release(&p.version, &p.url, &app.package_info().version.to_string())
                && cached_bytes(&dir, &p).is_some()
            {
                Some(p)
            } else {
                None
            }
        });
        if let Some(package) = recovered {
            *state.pending.lock() = Some(package.clone());
            publish(app, |s| {
                s.phase = "available".into();
                s.version = Some(package.version);
                s.cached = true;
                s.error = None;
            });
        } else {
            fail(app, &code);
        }
    }
    Ok(())
}
#[tauri::command]
pub async fn check_for_updates(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
) -> Result<(), String> {
    if window.label() != "main" {
        return Err("window".into());
    }
    check(&app).await
}

async fn download(app: &tauri::AppHandle, package: &Package) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::builder()
        .https_only(true)
        .timeout(Duration::from_secs(180))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            let allowed = attempt.url().scheme() == "https"
                && matches!(
                    attempt.url().host_str(),
                    Some(
                        "github.com"
                            | "release-assets.githubusercontent.com"
                            | "objects.githubusercontent.com"
                    )
                );
            if allowed && attempt.previous().len() < 5 {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .build()
        .map_err(|_| "download_failed")?;
    let mut response = client
        .get(&package.url)
        .send()
        .await
        .map_err(|_| "download_failed")?
        .error_for_status()
        .map_err(|_| "download_failed")?;
    let total = response.content_length();
    if total.is_some_and(|n| n > MAX_PACKAGE) {
        return Err("package_size".into());
    }
    publish(app, |s| {
        s.phase = "downloading".into();
        s.downloaded = 0;
        s.total = total;
        s.error = None;
    });
    let mut bytes = Vec::with_capacity(total.unwrap_or(0).min(MAX_PACKAGE) as usize);
    let mut emitted = Instant::now() - Duration::from_secs(1);
    while let Some(chunk) = response.chunk().await.map_err(|_| "download_failed")? {
        if (bytes.len() + chunk.len()) as u64 > MAX_PACKAGE {
            return Err("package_size".into());
        }
        bytes.extend_from_slice(&chunk);
        if emitted.elapsed() >= Duration::from_millis(80) {
            publish(app, |s| s.downloaded = bytes.len() as u64);
            emitted = Instant::now();
        }
    }
    if bytes.is_empty() || total.is_some_and(|n| n != bytes.len() as u64) {
        return Err("download_failed".into());
    }
    publish(app, |s| s.downloaded = bytes.len() as u64);
    Ok(bytes)
}
async fn apply(app: &tauri::AppHandle, locale: String, theme: String) -> Result<(), String> {
    let state = app.state::<Updates>();
    if state.busy.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    struct Release<'a>(&'a AtomicBool);
    impl Drop for Release<'_> {
        fn drop(&mut self) {
            self.0.store(false, Ordering::SeqCst);
        }
    }
    let _release = Release(&state.busy);
    let result: Result<(), String> = async {
        if crate::process::is_game_running() {
            return Err("game_running".into());
        }
        let package = state.pending.lock().clone().ok_or("no_update")?;
        publish(app, |s| {
            s.locale = if locale == "tr" { "tr" } else { "en" }.into();
            s.theme = if ["mint", "blue", "violet", "rose", "amber"].contains(&theme.as_str()) {
                theme
            } else {
                "rose".into()
            };
            s.version = Some(package.version.clone());
            s.error = None;
        });
        let dir = cache_dir(app)?;
        let bytes = if let Some(bytes) = cached_bytes(&dir, &package) {
            bytes
        } else {
            let bytes = download(app, &package).await?;
            publish(app, |s| s.phase = "verifying".into());
            verify(&bytes, &package.signature)?;
            save(&dir, &package, &bytes)?;
            bytes
        };
        publish(app, |s| {
            s.phase = "verifying".into();
            s.downloaded = bytes.len() as u64;
            s.cached = true;
        });
        verify(&bytes, &package.signature)?;
        if crate::process::is_game_running() {
            return Err("game_running".into());
        }
        publish(app, |s| s.phase = "preparing".into());
        crate::update_worker::handoff(app, package, bytes).await
    }
    .await;
    if let Err(ref code) = result {
        fail(app, code);
    }
    result
}
#[tauri::command]
pub async fn apply_update(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    locale: Option<String>,
    theme: Option<String>,
) -> Result<(), String> {
    if window.label() != "main" {
        return Err("window".into());
    }
    apply(
        &app,
        locale.unwrap_or_else(|| "en".into()),
        theme.unwrap_or_else(|| "rose".into()),
    )
    .await
}
// Compatibility command retained for earlier UI integrations; never opens a setup wizard.
pub fn install(app: &tauri::AppHandle) -> Result<(), String> {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = apply(&app, "en".into(), "rose".into()).await;
    });
    Ok(())
}
pub fn start(app: tauri::AppHandle) {
    crate::update_worker::acknowledge_launch(&app);
    if cfg!(debug_assertions) {
        publish(&app, |s| s.phase = "up_to_date".into());
        return;
    }
    tauri::async_runtime::spawn(async move {
        loop {
            let _ = check(&app).await;
            tokio::time::sleep(Duration::from_secs(6 * 60 * 60)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_release_validation() {
        let url="https://github.com/katsudc/katsu-tracker-releases/releases/download/v1.9.6-personal.36/Katsu-Tracker-setup.exe";
        assert!(valid_release("1.9.6-personal.36", url, "1.9.6-personal.35"));
        for invalid in [
            url.replace("https:", "http:"),
            url.replace("katsudc", "other"),
            format!("{url}?redirect=1"),
            url.replace("v1.9.6-personal.36", "v99.0.0"),
        ] {
            assert!(!valid_release(
                "1.9.6-personal.36",
                &invalid,
                "1.9.6-personal.35"
            ));
        }
        let bytes = include_bytes!("../tests/fixtures/updater-signed.txt");
        let signature = include_str!("../tests/fixtures/updater-signed.txt.sig");
        verify(bytes, signature).unwrap();
        let mut changed = bytes.to_vec();
        changed[0] ^= 1;
        assert!(verify(&changed, signature).is_err());
    }
}
