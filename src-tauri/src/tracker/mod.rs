mod personal_rounds;
mod models;
mod provider;
#[cfg(test)]
mod tests;
pub use models::*;
use parking_lot::Mutex;
use provider::{SessionProvider, StatsProvider};
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tauri::{Emitter, Manager};

pub struct Tracker {
    dir: PathBuf,
    cache: Arc<Mutex<Snapshot>>,
    identity: Mutex<Option<Profile>>,
    busy: AtomicBool,
}
fn read_json<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> Option<T> {
    if std::fs::metadata(path).ok()?.len() > 5 * 1024 * 1024 {
        return None;
    }
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}
fn save_json<T: Serialize>(path: &std::path::Path, value: &T) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    let bytes = serde_json::to_vec(value).map_err(|_| "cache_error")?;
    std::fs::write(&tmp, bytes)
        .and_then(|_| std::fs::rename(tmp, path))
        .map_err(|_| "cache_error".into())
}
// One-time compatibility cleanup for the removed optional provider. Never read credentials.
fn remove_legacy_provider_files(dir: &std::path::Path) {
    for name in ["henrik.key", "config.json"] {
        let _ = std::fs::remove_file(dir.join(name));
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("henrik-")
                && name.ends_with(".json")
                && entry.file_type().is_ok_and(|t| t.is_file())
            {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}
fn session_cache(path: &std::path::Path) -> Option<Snapshot> {
    let mut saved: Snapshot = read_json(path)?;
    if saved.schema != 1 || saved.provider != "session" {
        return None;
    }
    sanitize_snapshot(&mut saved);
    Some(saved)
}
pub fn setup(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let dir = app.path().app_data_dir()?.join("tracker");
    std::fs::create_dir_all(&dir)?;
    let last = dir.join("last.json");
    let mut cache = session_cache(&last).unwrap_or_else(|| {
        // A previously identified account may be retained, without external statistics.
        let profile = read_json::<Snapshot>(&last).and_then(|s| s.profile);
        Snapshot {
            schema: 1,
            provider: "session".into(),
            profile,
            ..Default::default()
        }
    });
    remove_legacy_provider_files(&dir);
    cache.status = if cache.profile.is_some() {
        "cached"
    } else {
        "no_profile"
    }
    .into();
    save_json(&last, &cache).ok();
    app.manage(Tracker {
        dir,
        identity: Mutex::new(cache.profile.clone()),
        cache: Arc::new(Mutex::new(cache)),
        busy: AtomicBool::new(false),
    });
    Ok(())
}
#[tauri::command]
pub fn tracker_load(state: tauri::State<'_, Tracker>) -> Snapshot {
    state.cache.lock().clone()
}
#[tauri::command]
pub async fn tracker_refresh(
    app: tauri::AppHandle,
    state: tauri::State<'_, Tracker>,
    session: tauri::State<'_, crate::state::AppState>,
    force: Option<bool>,
) -> Result<Snapshot, String> {
    let old = state.cache.lock().clone();
    if state.busy.swap(true, Ordering::SeqCst) {
        return Ok(old);
    }
    struct Release<'a>(&'a AtomicBool);
    impl Drop for Release<'_> {
        fn drop(&mut self) {
            self.0.store(false, Ordering::SeqCst);
        }
    }
    let _release = Release(&state.busy);
    let current_id = session.api.puuid.read().clone();
    let connected = *session.api.connected.read();
    let who = if connected && !current_id.is_empty() {
        current_id
    } else {
        state
            .identity
            .lock()
            .as_ref()
            .map(|p| p.puuid.clone())
            .unwrap_or_default()
    };
    if uuid::Uuid::parse_str(&who).is_err() {
        let mut value = old;
        value.status = "no_profile".into();
        return Ok(value);
    }
    let shard = session.api.shard.read().clone();
    let region = if !shard.is_empty() {
        shard
    } else {
        state
            .identity
            .lock()
            .as_ref()
            .map(|p| p.region.clone())
            .unwrap_or_default()
    };
    let path = state.dir.join(format!("session-{}-{}.json", region, who));
    let mut seed = if old
        .profile
        .as_ref()
        .is_some_and(|p| p.puuid == who && p.region == region)
    {
        old
    } else {
        session_cache(&path).unwrap_or_default()
    };
    seed.schema = 1;
    seed.provider = "session".into();
    if seed.retry_at.is_some_and(|until| until > now()) {
        return Ok(seed);
    }
    if !force.unwrap_or(false)
        && seed
            .updated_at
            .is_some_and(|t| now().saturating_sub(t) < 900)
        && !seed.partial
    {
        *state.cache.lock() = seed.clone();
        return Ok(seed);
    }
    let profile = seed.profile.clone().unwrap_or(Profile {
        puuid: who.clone(),
        region: region.clone(),
        ..Default::default()
    });
    *state.identity.lock() = Some(profile.clone());
    let account_changed = state
        .cache
        .lock()
        .profile
        .as_ref()
        .is_some_and(|p| p.puuid != who);
    if account_changed {
        *state.cache.lock() = seed.clone();
        let _ = app.emit("tracker_updated", &seed);
    }
    let cache = state.cache.clone();
    let progress_app = app.clone();
    let progress_id = who.clone();
    let progress: provider::Progress = Arc::new(move |value| {
        let active = progress_app.state::<crate::state::AppState>();
        if *active.api.connected.read() && *active.api.puuid.read() != progress_id {
            return;
        }
        if value
            .profile
            .as_ref()
            .is_some_and(|p| p.puuid == progress_id)
        {
            *cache.lock() = value.clone();
            let _ = progress_app.emit("tracker_updated", value);
        }
    });
    let result = SessionProvider(session.api.clone())
        .fetch(Profile { region, ..profile }, seed.clone(), progress)
        .await;
    if *session.api.connected.read() && *session.api.puuid.read() != who {
        return Ok(state.cache.lock().clone());
    }
    match result {
        Ok(mut value) => {
            sanitize_snapshot(&mut value);
            *state.cache.lock() = value.clone();
            save_json(&path, &value).ok();
            save_json(&state.dir.join("last.json"), &value).ok();
            let _ = app.emit("tracker_updated", &value);
            Ok(value)
        }
        Err(error) => {
            let progress = state.cache.lock().clone();
            if progress.profile.as_ref().is_some_and(|p| p.puuid == who) {
                seed = progress;
            }
            seed.status = error.code.into();
            seed.retry_at = (error.retry > 0).then(|| now() + error.retry);
            *state.cache.lock() = seed.clone();
            let _ = app.emit("tracker_updated", &seed);
            Ok(seed)
        }
    }
}

/// Remove only this tracker's explicit personal-history cache files, never Riot/session files.
#[tauri::command]
pub fn tracker_clear_cache(window:tauri::WebviewWindow,app:tauri::AppHandle,state:tauri::State<'_,Tracker>)->Result<(),String>{
    if window.label()!="main"{return Err("window".into());}
    if state.busy.swap(true,Ordering::SeqCst){return Err("busy".into());}
    struct ClearRelease<'a>(&'a AtomicBool);
    impl Drop for ClearRelease<'_>{fn drop(&mut self){self.0.store(false,Ordering::SeqCst);}}
    let _release=ClearRelease(&state.busy);
    if *app.state::<crate::state::AppState>().in_game_session.read(){return Err("busy".into());}
    let root=state.dir.canonicalize().map_err(|_|"storage")?;
    for entry in std::fs::read_dir(&root).map_err(|_|"storage")?.flatten().take(128) {
        let name=entry.file_name().to_string_lossy().into_owned();
        let kind=entry.file_type().map_err(|_|"storage")?;
        if !kind.is_file() || kind.is_symlink(){continue;}
        let owned=name=="last.json" || (name.starts_with("session-") && name.ends_with(".json") && name.chars().all(|c|c.is_ascii_alphanumeric()||c=='-'||c=='.'));
        if owned && entry.path().parent()==Some(root.as_path()){std::fs::remove_file(entry.path()).map_err(|_|"storage")?;}
    }
    let empty=Snapshot {schema:1,provider:"session".into(),status:"no_profile".into(),..Default::default()};
    *state.cache.lock()=empty.clone();*state.identity.lock()=None;
    let _=app.emit("tracker_updated",&empty);Ok(())
}
