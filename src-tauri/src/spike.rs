//! Opt-in audio timer. Selected reference audio stays local; the full capture is temporary.
use crate::spike_signal::{self as signal, Template, RATE};
use parking_lot::Mutex;
#[cfg(test)]
use crate::spike_signal::Detector;
use crate::spike_recording::Recording;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::{Arc, atomic::{AtomicBool, Ordering}}, time::{Duration, Instant}};
use tauri::{Emitter, Manager, State, WebviewWindow};

const RECORD_SECONDS: f64 = 15.0;
const MARGIN: f64 = 0.5;

#[derive(Clone, Serialize, Deserialize)]
struct Config {
    #[serde(default = "default_icon_enabled")] icon_enabled: bool,
    template: Template, threshold: f32, #[serde(default)] source: String,
    #[serde(default)] enabled: bool,
    #[serde(default = "default_correlation_threshold")] correlation_threshold: f32,
}

fn default_icon_enabled() -> bool {true}

fn default_correlation_threshold() -> f32 {0.25}

fn load_config(path: &std::path::Path) -> Option<Config> {
    std::fs::read(path).ok().filter(|b| b.len()<2_000_000)
        .and_then(|b|serde_json::from_slice::<Config>(&b).ok())
        .filter(|c|c.template.valid() && c.threshold.is_finite() && (0.7..=0.98).contains(&c.threshold)
            && c.correlation_threshold.is_finite() && (0.10..=0.95).contains(&c.correlation_threshold))
        .map(|mut c| {
            if c.template.sample_rate == 8000 && c.source == "youtube:RRdK1cVjZzk" {
                c.template = reference_config().template;
            }
            c
        })
}

fn save_config(path: &std::path::Path, config: &Config) -> Result<(), String> {
    if let Some(parent) = path.parent() { std::fs::create_dir_all(parent).map_err(|e|e.to_string())?; }
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, serde_json::to_vec(config).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    std::fs::rename(temp, path).map_err(|e|e.to_string())
}

fn reference_config() -> Config {
    let samples: Vec<f32> = include_bytes!("../assets/spike-planted-reference-48k.pcm16").chunks_exact(2)
        .map(|v|i16::from_le_bytes([v[0],v[1]]) as f32 / 32768.0).collect();
    Config { icon_enabled:true, template:Template::from_audio(&samples,0.0).expect("bundled spike reference must be valid"),
        correlation_threshold:default_correlation_threshold(), threshold:0.82, source:"youtube:RRdK1cVjZzk".into(), enabled:false }
}

#[derive(Clone, Serialize)]
pub struct Status {
    icon_enabled:bool, icon_status:String, icon_similarity:f32,
    enabled: bool, calibrated: bool, recording: bool, recorded_seconds: f64,
    recording_ready: bool, remaining: Option<f64>, full_defuse: bool, half_defuse: bool,
    full_sequence: bool, reference_seconds: f64, correlation_ready: bool, correlation_threshold: f32, correlation_similarity: f32,
    demo: bool, similarity: f32, peak_similarity: f32, listening: bool, level: f32, threshold: f32, reference_name: String, error: Option<String>,
}

struct Inner {
    icon_status:String, icon_similarity:f32,
    config: Option<Config>, enabled: bool, record_start: Option<Instant>,
    audio: Vec<f32>, preview: Recording, recording_ready: bool, timer: Option<Instant>, demo: bool,
    similarity: f32, peak_similarity: f32, listening: bool, level: f32, error: Option<String>, generation: u64,
    block_audio_until: Option<Instant>,
    correlation_similarity: f32,
}

impl Inner {
    fn status(&self) -> Status {
        let remaining = self.timer.map(|t| signal::remaining(t.elapsed().as_secs_f64()));
        Status { icon_enabled:self.config.as_ref().is_none_or(|c|c.icon_enabled), icon_status:self.icon_status.clone(), icon_similarity:self.icon_similarity, enabled:self.enabled, calibrated:self.config.is_some(), recording:self.record_start.is_some(),
            recorded_seconds:self.audio.len() as f64 / RATE as f64, recording_ready:self.recording_ready,
            remaining, full_defuse:remaining.is_some_and(|r| r > signal::FULL_DEFUSE_SECONDS + MARGIN),
            half_defuse:remaining.is_some_and(|r| r > signal::HALF_DEFUSE_SECONDS + MARGIN),
            correlation_ready:self.config.as_ref().is_some_and(|c|c.template.waveform.is_some()),
            correlation_threshold:self.config.as_ref().map_or(default_correlation_threshold(),|c|c.correlation_threshold),
            correlation_similarity:self.correlation_similarity,
            full_sequence:self.config.as_ref().is_some_and(|c|c.template.sequence.is_some()),
            reference_seconds:self.config.as_ref().map_or(0.0, |c|c.template.sequence.as_ref().map_or(signal::TEMPLATE_SECONDS, |seq|seq.completion_seconds+signal::TEMPLATE_SECONDS)),
            demo:self.demo, similarity:self.similarity, peak_similarity:self.peak_similarity, listening:self.listening, level:self.level,
            threshold:self.config.as_ref().map_or(0.90, |c| c.threshold),
            reference_name:self.config.as_ref().map_or("Yok", |c| if c.source.starts_with("youtube:") { "Hazır video sesi" } else { "Kendi kaydın" }).into(),
            error:self.error.clone() }
    }
    fn reset(&mut self) { self.timer = None; self.demo = false; self.similarity = 0.0; self.correlation_similarity = 0.0; self.generation += 1; }
    fn round_ended(&mut self) {
        if self.demo { return; }
        self.reset();
        // Do not let the victory/defuse sound immediately reopen the overlay.
        self.block_audio_until = Some(Instant::now() + Duration::from_secs(5));
    }
    fn can_detect(&self) -> bool {
        self.enabled && self.timer.is_none()
            && self.block_audio_until.is_none_or(|until| Instant::now() >= until)
    }
    fn fail(&mut self, error: String) {
        self.enabled = self.enabled && self.config.as_ref().is_some_and(|c|c.icon_enabled); self.record_start = None; self.audio.clear(); self.preview.clear(); self.recording_ready = false;
        self.reset(); self.error = Some(format!("Ses yakalama durdu: {error}. Windows varsayılan çıkışını kontrol edip yeniden aç."));
    }
}

#[derive(Clone)]
pub struct Spike { inner: Arc<Mutex<Inner>>, stop: Arc<AtomicBool>, game_visible: Arc<AtomicBool>, path: PathBuf }

fn main_only(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == "main" { Ok(()) } else { Err("Bu işlem ana pencereye ait.".into()) }
}

#[tauri::command]
pub fn spike_get_status(state: State<'_, Spike>) -> Status { state.inner.lock().status() }

#[tauri::command]
pub fn spike_set_enabled(window: WebviewWindow, state: State<'_, Spike>, enabled: bool) -> Result<(), String> {
    main_only(&window)?;
    let mut s = state.inner.lock();
    if enabled && s.config.is_none() {s.config=Some(reference_config());}
    if let Some(mut config) = s.config.clone() {
        config.enabled = enabled;
        config.icon_enabled = true;
        save_config(&state.path, &config)?;
        s.config = Some(config);
    }
    s.enabled = enabled; s.peak_similarity = 0.0; s.error = None; s.record_start = None; s.audio.clear(); s.preview.clear(); s.recording_ready = false; s.reset();
    Ok(())
}

#[tauri::command]
pub fn spike_set_icon_enabled(window:WebviewWindow,state:State<'_,Spike>,enabled:bool)->Result<(),String> {
    main_only(&window)?;
    let mut s=state.inner.lock();let mut config=s.config.clone().unwrap_or_else(reference_config);
    config.icon_enabled=enabled;save_config(&state.path,&config)?;
    s.config=Some(config);s.generation+=1;Ok(())
}

#[tauri::command]
pub fn spike_use_reference(window: WebviewWindow, state: State<'_, Spike>) -> Result<(), String> {
    main_only(&window)?;
    let mut config = reference_config();
    let mut s = state.inner.lock();
    config.enabled = s.config.as_ref().is_some_and(|c| c.enabled);
    config.icon_enabled = s.config.as_ref().is_none_or(|c| c.icon_enabled);
    save_config(&state.path, &config)?;
    s.enabled = config.enabled; s.config = Some(config); s.record_start = None; s.audio.clear(); s.preview.clear();
    s.recording_ready = false; s.error = None; s.peak_similarity = 0.0; s.reset();
    Ok(())
}

#[tauri::command]
pub fn spike_set_threshold(window: WebviewWindow, state: State<'_, Spike>, threshold: f32, correlation_threshold: Option<f32>) -> Result<(), String> {
    main_only(&window)?;
    if !threshold.is_finite() || !(0.7..=0.98).contains(&threshold) { return Err("Eşik 0,70–0,98 arasında olmalı.".into()); }
    let mut s = state.inner.lock();
    let mut config = s.config.clone().ok_or("Önce kurulma sesini tanıt.")?;
    if let Some(value)=correlation_threshold {
        if !value.is_finite() || !(0.10..=0.95).contains(&value) {return Err("Korelasyon eşiği 0,10–0,95 arasında olmalı.".into());}
        config.correlation_threshold=value;
    }
    config.threshold = threshold;
    save_config(&state.path, &config)?;
    s.config = Some(config); s.peak_similarity = 0.0; s.generation += 1;
    Ok(())
}

#[tauri::command]
pub fn spike_record(window: WebviewWindow, state: State<'_, Spike>, start: bool) -> Result<(), String> {
    main_only(&window)?;
    let mut s = state.inner.lock();
    s.enabled = !start && s.config.as_ref().is_some_and(|c| c.enabled); s.audio.clear(); s.preview.clear(); s.recording_ready = false; s.error = None; s.reset();
    s.record_start = if start { Some(Instant::now()) } else { None };
    Ok(())
}

#[tauri::command]
pub fn spike_get_recording(window: WebviewWindow, state: State<'_, Spike>) -> Result<tauri::ipc::Response, String> {
    main_only(&window)?;
    let s = state.inner.lock();
    if !s.recording_ready { return Err("Kayıt henüz hazır değil.".into()); }
    Ok(tauri::ipc::Response::new(s.preview.wav()?))
}

#[tauri::command]
pub fn spike_save_template(window: WebviewWindow, state: State<'_, Spike>, marker: f64, threshold: f32, start_marker: Option<f64>) -> Result<(), String> {
    main_only(&window)?;
    if !threshold.is_finite() || !(0.7..=0.98).contains(&threshold) { return Err("Eşik 0,70–0,98 arasında olmalı.".into()); }
    let mut s = state.inner.lock();
    if !s.recording_ready { return Err("Önce bir ses kaydı al.".into()); }
    let template = match start_marker {
        Some(start) => Template::from_full_audio(&s.audio,start,marker)?,
        None => Template::from_audio(&s.audio,marker)?,
    };
    let config = Config { icon_enabled:s.config.as_ref().is_none_or(|c|c.icon_enabled), template, threshold, correlation_threshold:s.config.as_ref().map_or(default_correlation_threshold(),|c|c.correlation_threshold), source:"custom".into(), enabled:s.config.as_ref().is_some_and(|c| c.enabled) };
    if let Some(parent) = state.path.parent() { std::fs::create_dir_all(parent).map_err(|e|e.to_string())?; }
    save_config(&state.path, &config)?;
    s.enabled = config.enabled; s.config = Some(config); s.audio.clear(); s.preview.clear(); s.recording_ready = false; s.error = None; s.reset();
    Ok(())
}

#[tauri::command]
pub fn spike_reset(window: WebviewWindow, state: State<'_, Spike>) -> Result<(), String> {
    main_only(&window)?; let mut s = state.inner.lock(); s.reset(); s.peak_similarity = 0.0; Ok(())
}

#[tauri::command]
pub fn spike_demo(window: WebviewWindow, state: State<'_, Spike>) -> Result<(), String> {
    main_only(&window)?;
    let mut s = state.inner.lock();
    s.record_start = None; s.audio.clear(); s.preview.clear(); s.recording_ready = false; s.reset();
    s.timer = Some(Instant::now()); s.demo = true; Ok(())
}

#[tauri::command]
pub fn spike_clear_template(window: WebviewWindow, state: State<'_, Spike>) -> Result<(), String> {
    main_only(&window)?;
    match std::fs::remove_file(&state.path) { Ok(()) => (), Err(e) if e.kind()==std::io::ErrorKind::NotFound => (), Err(e) => return Err(e.to_string()) }
    let mut s = state.inner.lock();
    s.config = None; s.enabled = false; s.record_start = None; s.audio.clear(); s.preview.clear(); s.recording_ready = false; s.reset(); Ok(())
}

pub fn setup(app: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let path = app.path().app_data_dir()?.join("spike-timer.json");
    let config = load_config(&path);
    let enabled = config.as_ref().is_some_and(|c| c.enabled);
    let state = Spike { inner:Arc::new(Mutex::new(Inner { config, enabled, record_start:None,
        audio:Vec::new(), preview:Recording::default(), recording_ready:false, timer:None, demo:false, similarity:0.0, peak_similarity:0.0, listening:false, level:0.0, error:None, generation:0, correlation_similarity:0.0, icon_status:"Bekleniyor".into(), icon_similarity:0.0, block_audio_until:None })),
        stop:Arc::new(AtomicBool::new(false)), game_visible:Arc::new(AtomicBool::new(false)), path };
    app.manage(state.clone());
    let overlay = tauri::WebviewWindowBuilder::new(app, "spike", tauri::WebviewUrl::App("index.html#spike".into()))
        .title("katsu's Tracker · Spike Timer").inner_size(200.0, 46.0).decorations(false).shadow(false).transparent(true)
        .always_on_top(true).skip_taskbar(true).focused(false).focusable(false).visible(false).resizable(false).build()?;
    overlay.set_ignore_cursor_events(true)?;
    let display_app = app.clone();
    let display_state = state.clone();
    std::thread::Builder::new().name("spike-display".into()).spawn(move || display_worker(display_app, display_state))?;
    let icon_state=state.clone();
    std::thread::Builder::new().name("spike-icon".into()).spawn(move || icon_worker(icon_state))?;
    let handle = app.clone();
    std::thread::Builder::new().name("spike-state".into()).spawn(move || worker(handle, state))?;
    Ok(())
}

pub fn shutdown(app: &tauri::AppHandle) { if let Some(s) = app.try_state::<Spike>() { s.stop.store(true, Ordering::Relaxed); } }

fn monitor_bounds(m: &tauri::Monitor) -> crate::spike_display::MonitorBounds {
    crate::spike_display::MonitorBounds { x:m.position().x, y:m.position().y,
        width:m.size().width, height:m.size().height }
}

fn overlay_placement(app: &tauri::AppHandle, overlay: &WebviewWindow, demo: bool, game: Option<crate::spike_display::MonitorBounds>) -> Option<crate::spike_display::Placement> {
    use crate::spike_display::{placement, select_target};
    let monitors = overlay.available_monitors().ok()?;
    let fallback = if demo {
        app.get_webview_window("main").and_then(|w|w.current_monitor().ok().flatten())
            .or_else(||overlay.primary_monitor().ok().flatten()).map(|m|monitor_bounds(&m))
    } else { None };
    let target = select_target(game, demo, fallback)?;
    let scale = crate::spike_display::target_scale(target, &monitors);
    Some(placement(target, scale, &crate::spike_appearance::current(app)))
}

// Window/process discovery runs separately so it cannot interrupt audio capture.
fn display_worker(app: tauri::AppHandle, state: Spike) {
    let Some(w) = app.get_webview_window("spike") else { return; };
    let mut shown = false;
    let mut last_placement = None;
    let mut last_request = (false, false);
    let mut game = None;
    let mut last_game_check = Instant::now()-Duration::from_secs(1);
    let mut last_check = Instant::now()-Duration::from_secs(1);
    while !state.stop.load(Ordering::Relaxed) {
        // Keep the fast native signal warm even before the first countdown.
        // The full Riot snapshot can be delayed by roster/rank/skin requests.
        if last_game_check.elapsed() >= Duration::from_millis(500) {
            game = crate::spike_display::game_monitor();
            state.game_visible.store(game.is_some(), Ordering::Relaxed);
            last_game_check = Instant::now();
        }
        let request = {
            let s = state.inner.lock();
            (s.timer.is_some_and(|t|t.elapsed().as_secs_f64()<signal::FUSE_SECONDS), s.demo)
        };
        if !request.0 {
            if shown {
                crate::set_webview_visible(&w, false);
                let _ = w.hide();
                shown = false;
            }
            last_placement = None;
        } else if request != last_request || last_check.elapsed()>=Duration::from_millis(500) {
            last_check = Instant::now();
            if let Some(p) = overlay_placement(&app, &w, request.1, game) {
                let positioned = if last_placement != Some(p) || !shown {
                    // Move onto the target DPI domain, resize in physical pixels, then
                    // re-center after Windows has applied its DPI-change suggestion.
                    let pos = tauri::PhysicalPosition::new(p.x,p.y);
                    w.set_position(pos).and_then(|_|w.set_size(tauri::PhysicalSize::new(p.width,p.height)))
                        .and_then(|_| {
                            // Windows can clamp a small borderless window to its minimum width.
                            // Center the actual window, not the requested width.
                            let actual=w.outer_size()?;
                            let x=i64::from(p.x)+(i64::from(p.width)-i64::from(actual.width))/2;
                            let y=i64::from(p.y)+(i64::from(p.height)-i64::from(actual.height))/2;
                            w.set_position(tauri::PhysicalPosition::new(x as i32,y as i32))
                        }).is_ok()
                } else { true };
                if positioned {
                    crate::spike_display::keep_above_game(&w);
                    last_placement = Some(p);
                    if !shown && w.show().is_ok() {
                        crate::set_webview_visible(&w, true);
                        shown = true;
                    }
                }
            } else {
                // A minimized/closed game has no visible target. Restore on its monitor
                // when it comes back; elapsed time continues instead of restarting.
                if shown {
                    crate::set_webview_visible(&w, false);
                    let _ = w.hide();
                    shown = false;
                }
                last_placement = None;
            }
        }
        last_request = request;
        std::thread::sleep(Duration::from_millis(80));
    }
}

fn icon_worker(state:Spike) {
    tracing::info!("[Spike] Detection mode: ICON ONLY; audio capture excluded from this build");
    let mut capture=crate::spike_icon::Capture::default();
    let mut gate=crate::spike_icon::Gate::default();
    let mut generation=u64::MAX;
    let mut target=None;
    let mut diagnostic=Instant::now()-Duration::from_secs(10);
    while !state.stop.load(Ordering::Relaxed) {
        let start=Instant::now();
        let (active,current)={let s=state.inner.lock();
            (s.enabled && s.record_start.is_none(),s.generation)};
        if generation!=current {gate.reset();generation=current;}
        if !active {gate.reset();let mut s=state.inner.lock();s.icon_status="Kapalı".into();s.icon_similarity=0.0;}
        else {match capture.read() {
            Ok(Some((pixels,w,h,scale,id)))=> {
                if target!=Some((id,w,h)) {gate.reset();target=Some((id,w,h));}
                let score=crate::spike_icon::similarity(&pixels,w,h,scale);
                let detected=gate.update(score>=crate::spike_icon::THRESHOLD,start);
                let mut s=state.inner.lock();s.icon_similarity=score;
                s.icon_status=if score>=crate::spike_icon::THRESHOLD {"Simge görünüyor"} else {"Simge izleniyor"}.into();
                // Recheck the current toggle and round state after processing this image.
                if s.generation==current && s.can_detect() && s.record_start.is_none() {
                    if let Some(at)=detected {s.timer=Some(at);s.demo=false;tracing::info!("[Spike] HUD icon edge confirmed: {:.3}",score);}
                }
            },
            Ok(None)=>{gate.reset();target=None;let mut s=state.inner.lock();s.icon_status="Oyun ön planda bekleniyor".into();s.icon_similarity=0.0;},
            Err(error)=>{gate.reset();let mut s=state.inner.lock();s.icon_status=error;s.icon_similarity=0.0;},
        }}
        if diagnostic.elapsed()>=Duration::from_secs(5) {
            let s=state.inner.lock();
            tracing::info!("[Spike] HUD diagnostic: enabled={}, status={}, score={:.3}, timer={}",s.enabled,s.icon_status,s.icon_similarity,s.timer.is_some());
            diagnostic=Instant::now();
        }
        std::thread::sleep(Duration::from_millis(100).saturating_sub(start.elapsed()));
    }
}

// Only icon recognition can start a real timer. No audio device is opened.
fn worker(app: tauri::AppHandle, state: Spike) {
    let mut session=crate::spike_session::SessionTracker::default();
    let mut round_log=std::env::var_os("LOCALAPPDATA").map(|base|
        crate::spike_round::RoundLog::new(PathBuf::from(base).join("VALORANT/Saved/Logs/ShooterGame.log")));
    let mut last_log_poll=Instant::now();
    while !state.stop.load(Ordering::Relaxed) {
        let round=app.state::<crate::state::AppState>().last_full_game_state.read().as_ref()
            .map(|g|(g.state.clone(),g.match_id.clone(),g.ally_score,g.enemy_score));
        let phase=app.state::<crate::state::AppState>().last_known_state.read().clone();
        let changed=session.changed(round.as_ref(),&phase);
        let ended=if last_log_poll.elapsed()>=Duration::from_millis(250) {
            last_log_poll=Instant::now();round_log.as_mut().is_some_and(|log|log.poll())
        } else {false};
        let mut s=state.inner.lock();
        if ended {s.round_ended();}
        if changed && !s.demo {s.reset();}
        if s.timer.is_some_and(|t|t.elapsed().as_secs_f64()>=signal::FUSE_SECONDS) {s.reset();}
        let status=s.status();drop(s);
        let _=app.emit_to("main","spike-status",&status);
        let _=app.emit_to("spike","spike-status",&status);
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_recording_persists_and_old_custom_fingerprint_is_not_replaced() {
        let mut config=reference_config();
        let completion_audio:Vec<f32>=include_bytes!("../assets/spike-planted-reference-48k.pcm16")
            .chunks_exact(2).map(|b|i16::from_le_bytes([b[0],b[1]]) as f32/32768.0).collect();
        let mut full:Vec<f32>=include_bytes!("../tests/fixtures/spike-activation-negative-48k.pcm16")
            .chunks_exact(2).map(|b|i16::from_le_bytes([b[0],b[1]]) as f32/32768.0).collect();
        let completion=full.len() as f64/RATE as f64;full.extend(completion_audio);
        let path=std::env::temp_dir().join(format!("spike-full-{}.json",uuid::Uuid::new_v4()));
        config.source="custom".into();config.enabled=true;config.threshold=0.76;
        let old=serde_json::to_vec(&config.template).unwrap();
        save_config(&path,&config).unwrap();
        assert_eq!(serde_json::to_vec(&load_config(&path).unwrap().template).unwrap(),old);
        config.template=Template::from_full_audio(&full,0.0,completion).unwrap();
        save_config(&path,&config).unwrap();
        assert!(std::fs::metadata(&path).unwrap().len()>100_000,"exercise increased config limit");
        let restored=load_config(&path).unwrap();
        assert!(restored.enabled);assert_eq!(restored.threshold,0.76);
        assert_eq!(restored.template.sequence.unwrap().completion_seconds,completion);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn upgrades_bundled_legacy_template_preserving_preferences() {
        let audio: Vec<_> = include_bytes!("../assets/spike-planted-reference.pcm16")
            .chunks_exact(2).map(|b|i16::from_le_bytes([b[0],b[1]]) as f32/32768.0).collect();
        let old = crate::spike_signal_legacy::Template::from_audio(&audio,0.0).unwrap();
        let mut json = serde_json::to_value(reference_config()).unwrap();
        json["template"] = serde_json::to_value(old).unwrap();
        json["threshold"] = serde_json::json!(0.75);
        json["enabled"] = serde_json::json!(true);
        let path = std::env::temp_dir().join(format!("spike-upgrade-{}.json",std::process::id()));
        std::fs::write(&path,serde_json::to_vec(&json).unwrap()).unwrap();
        let config = load_config(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(config.template.sample_rate,48000);
        assert_eq!(config.threshold,0.75); assert!(config.enabled);
    }
    #[test]
    fn first_plant_matches_while_initial_match_and_score_arrive() {
        use crate::spike_session::{SessionTracker, should_capture};
        let config = reference_config();
        let mut session = SessionTracker::default();
        let mut detector = Detector::default();
        assert!(should_capture(true,false,true,false));
        let mut audio: Vec<f32> = include_bytes!("../assets/spike-planted-reference-48k.pcm16").chunks_exact(2)
            .map(|v|i16::from_le_bytes([v[0],v[1]]) as f32/32768.0).collect();
        audio.extend(vec![0.0;320]);
        let mut matched = false;
        for (i, packet) in audio.chunks(137).enumerate() {
            // Full game snapshot arrives halfway through the FIRST reference.
            let snapshot = ("ingame".into(),Some("first-match".into()),None,None);
            if i == 30 && session.changed(Some(&snapshot),"ingame") { detector = Detector::default(); }
            matched |= detector.push(packet,&config.template).is_some_and(|(score,_)|score>=config.threshold);
        }
        assert!(matched, "initial snapshot must not discard the first plant's partial fingerprint");
        let mut s = Inner { config:Some(config), enabled:true, record_start:None, audio:vec![], preview:Recording::default(), recording_ready:false,
            timer:Some(Instant::now()), demo:false, similarity:1.0, peak_similarity:1.0, listening:true, level:0.0, error:None, generation:0, correlation_similarity:0.0, icon_status:"Bekleniyor".into(), icon_similarity:0.0, block_audio_until:None };
        let score = ("ingame".into(),Some("first-match".into()),Some(2),Some(1));
        if session.changed(Some(&score),"ingame") { s.reset(); }
        assert!(s.status().remaining.is_some(), "first score arrival must not cancel the detected countdown");
        s.round_ended();
        assert!(s.status().remaining.is_none(), "real round-end event must still close it");
    }
    #[test]
    fn saved_preferences_survive_reload_and_can_be_overwritten() {
        let path = std::env::temp_dir().join(format!("spike-settings-{}.json", uuid::Uuid::new_v4()));
        let mut config = reference_config();
        config.enabled = true; config.icon_enabled=false; config.threshold = 0.76;
        save_config(&path, &config).unwrap();
        let restored = load_config(&path).unwrap();
        assert!(!restored.icon_enabled); assert!(restored.enabled); assert_eq!(restored.threshold, 0.76);
        assert_eq!(restored.source, config.source); assert!(restored.template.valid());
        config.enabled = false;
        save_config(&path, &config).unwrap();
        assert!(!load_config(&path).unwrap().enabled);
        assert!(!path.with_extension("json.tmp").exists());
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn legacy_settings_keep_reference_and_threshold_without_enabling() {
        let mut legacy = serde_json::to_value(reference_config()).unwrap();
        legacy.as_object_mut().unwrap().remove("enabled");
        legacy.as_object_mut().unwrap().remove("icon_enabled");
        legacy["threshold"] = serde_json::json!(0.76);
        let restored: Config = serde_json::from_value(legacy).unwrap();
        assert!(restored.icon_enabled); assert!(!restored.enabled); assert_eq!(restored.threshold, 0.76);
        assert!(restored.template.valid()); assert_eq!(restored.source, "youtube:RRdK1cVjZzk");
    }
    #[test]
    fn bundled_reference_matches_quieter_noisy_playback_but_not_plant_activation() {
        let config = reference_config();
        let samples: Vec<f32> = include_bytes!("../assets/spike-planted-reference-48k.pcm16").chunks_exact(2)
            .map(|v|i16::from_le_bytes([v[0],v[1]]) as f32 / 32768.0).collect();
        let rms = (samples.iter().map(|v|v*v).sum::<f32>() / samples.len() as f32).sqrt();
        let mut random = 42u32;
        let mut playback = vec![0.0; 1973];
        playback.extend(samples.iter().map(|v| {
            random = random.wrapping_mul(1664525).wrapping_add(1013904223);
            let noise = (random as f64/u32::MAX as f64*2.0-1.0) as f32;
            v * 0.3 + noise * rms * 0.03
        }));
        playback.extend(vec![0.0; 320]);
        let mut detector = Detector::default();
        let peak = playback.chunks(137).filter_map(|p|detector.push(p,&config.template).map(|v|v.0)).fold(0.0_f32,f32::max);
        assert!(peak>=config.threshold, "reference score {peak}");
        let activation: Vec<f32> = include_bytes!("../tests/fixtures/spike-activation-negative-48k.pcm16").chunks_exact(2)
            .map(|v|i16::from_le_bytes([v[0],v[1]]) as f32/32768.0).collect();
        let mut detector = Detector::default();
        let negative = activation.chunks(137).filter_map(|p|detector.push(p,&config.template).map(|v|v.0)).fold(0.0_f32,f32::max);
        assert!(negative<config.threshold, "activation false match {negative}");
    }
    #[test]
    fn defuse_boundaries_include_margin_and_reset_clears_timer() {
        let mut s = Inner { config:None, enabled:false, record_start:None, audio:vec![], preview:Recording::default(), recording_ready:false,
            timer:Some(Instant::now()-Duration::from_secs_f64(37.6)), demo:true, similarity:1.0, peak_similarity:1.0, listening:false, level:0.0, error:None, generation:0, correlation_similarity:0.0, icon_status:"Bekleniyor".into(), icon_similarity:0.0, block_audio_until:None };
        assert!(!s.status().full_defuse); assert!(s.status().half_defuse);
        s.timer = Some(Instant::now()-Duration::from_secs_f64(41.1));
        assert!(!s.status().half_defuse);
        s.reset(); assert!(s.status().remaining.is_none()); assert!(!s.demo);
    }
    #[test]
    fn early_round_end_hides_countdown_and_prevents_immediate_retrigger() {
        let mut s = Inner { config:None, enabled:true, record_start:None, audio:vec![], preview:Recording::default(), recording_ready:false,
            timer:Some(Instant::now()-Duration::from_secs(10)), demo:false, similarity:1.0, peak_similarity:1.0, listening:true, level:0.0, error:None, generation:0, correlation_similarity:0.0, icon_status:"Bekleniyor".into(), icon_similarity:0.0, block_audio_until:None };
        assert!(s.status().remaining.unwrap() > 30.0);
        s.round_ended();
        assert!(s.status().remaining.is_none()); assert!(s.enabled); assert!(!s.can_detect());
        assert_eq!(s.generation, 1);
        s.block_audio_until = Some(Instant::now()-Duration::from_secs(1));
        assert!(s.can_detect()); // next plant can start normally
        s.demo = true; s.timer = Some(Instant::now());
        s.round_ended(); assert!(s.status().remaining.is_some());
    }
}
