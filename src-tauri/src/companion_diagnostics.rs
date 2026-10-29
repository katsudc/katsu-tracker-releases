//! Allowlisted diagnostics. Never reads raw log files, cookies or account/session contents.
use serde_json::{json, Value};
use tauri::Manager;
fn preview(app: &tauri::AppHandle) -> Value {
    let runtime = crate::companion_runtime::status(app);
    let output = crate::companion_runtime::guarded_get_spike_output_status(app.clone(), None);
    let spike = crate::companion_runtime::guarded_spike_get_status(app.clone());
    let game = crate::spike_display::game_monitor();
    let monitors = app.get_webview_window("main").and_then(|w| w.available_monitors().ok()).unwrap_or_default();
    let displays: Vec<_> = monitors.iter().enumerate().map(|(i,m)| json!({
        "display": i+1, "width":m.size().width,"height":m.size().height,"scaling":m.scale_factor(),
        "selected_for_game":game.is_some_and(|g| g.x >= m.position().x && g.x < m.position().x + m.size().width as i32 && g.y >= m.position().y && g.y < m.position().y + m.size().height as i32)
    })).collect();
    let icon = spike["icon_status"].as_str().unwrap_or("");
    let capture = match icon {
        "Simge izleniyor" | "Simge görünüyor" => "reading",
        "Oyun ön planda bekleniyor" | "Bekleniyor" => "waiting_foreground",
        "Kapalı" => "disabled", _ => "capture_unavailable",
    };
    let windows = windows_version();
    let commit = option_env!("KATSU_BUILD_COMMIT").filter(|v| v.len()==40 && v.chars().all(|c|c.is_ascii_hexdigit()));
    // Copy individual primitive fields; do not serialize whole API/connection objects.
    let widget = json!({
        "mode":output["mode"],"output":output["output"],
        "game_bar_installed":output["game_bar_installed"].as_bool(),
        "widget_installed":output["widget_installed"].as_bool(),
        "pinned":output["pinned"].as_bool(),"click_through":output["click_through"].as_bool(),
        "phase":output["phase"].as_str().filter(|s|s.len()<50 && s.chars().all(|c|c.is_ascii_lowercase()||c=='_')),
    });
    json!({
        "schema":1,"application":"katsu's Tracker","version":app.package_info().version.to_string(),"build_commit":commit,
        "windows":windows,"runtime_mode":runtime.mode,"disabled_modules":runtime.disabled_modules,"failed_modules":runtime.failed_modules,
        "displays":displays,"game_detected":game.is_some(),"timer_enabled":spike["enabled"].as_bool(),
        "countdown_active":spike["remaining"].is_number(),"hud_capture":capture,"widget":widget,
        "widget_setup":crate::gamebar_install_report::load(app),
        "events":[{"category":"runtime","state":crate::companion_runtime::status(app).mode},{"category":"hud","state":capture}],
        "exclusions":["account_names","player_ids","tokens","cookies","raw_logs","personal_files","file_paths","notes"]
    })
}
fn windows_version() -> Value {
    #[cfg(windows)]
    {
        use winreg::{RegKey,enums::{HKEY_LOCAL_MACHINE,KEY_READ}};
        if let Ok(key)=RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion",KEY_READ) {
            let text=|name:&str| key.get_value::<String,_>(name).ok().filter(|s|s.len()<100 && !s.chars().any(char::is_control));
            return json!({"product":text("ProductName"),"build":text("CurrentBuildNumber"),"release":text("DisplayVersion")});
        }
    }
    json!({"product":null,"build":null,"release":null})
}
fn main_only(window: &tauri::WebviewWindow) -> Result<(),String> { if window.label()=="main"{Ok(())}else{Err("window".into())} }
#[tauri::command]
pub async fn diagnostic_preview(window: tauri::WebviewWindow, app: tauri::AppHandle) -> Result<Value,String> {
    main_only(&window)?;
    tauri::async_runtime::spawn_blocking(move || preview(&app)).await.map_err(|_|"diagnostics_unavailable".into())
}
fn crc32(bytes:&[u8])->u32 {
    let mut crc=!0u32;
    for b in bytes {crc^=u32::from(*b);for _ in 0..8 {crc=(crc>>1)^if crc&1==1{0xedb88320}else{0};}}
    !crc
}
fn u16le(out:&mut Vec<u8>,n:u16){out.extend_from_slice(&n.to_le_bytes());}
fn u32le(out:&mut Vec<u8>,n:u32){out.extend_from_slice(&n.to_le_bytes());}
// ZIP "store" format: reports are a few KB; no compression worker/dependency is needed.
pub fn zip(files:&[(&str,Vec<u8>)])->Vec<u8> {
    let mut out=Vec::new();let mut central=Vec::new();
    for (name,data) in files {
        let offset=out.len() as u32;let crc=crc32(data);let n=name.as_bytes();
        u32le(&mut out,0x04034b50);u16le(&mut out,20);u16le(&mut out,0);u16le(&mut out,0);u16le(&mut out,0);u16le(&mut out,33);
        u32le(&mut out,crc);u32le(&mut out,data.len() as u32);u32le(&mut out,data.len() as u32);u16le(&mut out,n.len() as u16);u16le(&mut out,0);out.extend_from_slice(n);out.extend_from_slice(data);
        u32le(&mut central,0x02014b50);u16le(&mut central,20);u16le(&mut central,20);u16le(&mut central,0);u16le(&mut central,0);u16le(&mut central,0);u16le(&mut central,33);
        u32le(&mut central,crc);u32le(&mut central,data.len() as u32);u32le(&mut central,data.len() as u32);u16le(&mut central,n.len() as u16);
        for _ in 0..4 {u16le(&mut central,0);}u32le(&mut central,0);u32le(&mut central,offset);central.extend_from_slice(n);
    }
    let start=out.len() as u32;let len=central.len() as u32;out.extend_from_slice(&central);
    u32le(&mut out,0x06054b50);u16le(&mut out,0);u16le(&mut out,0);u16le(&mut out,files.len() as u16);u16le(&mut out,files.len() as u16);
    u32le(&mut out,len);u32le(&mut out,start);u16le(&mut out,0);out
}
#[tauri::command]
pub async fn diagnostic_zip(window:tauri::WebviewWindow,app:tauri::AppHandle)->Result<tauri::ipc::Response,String>{
    main_only(&window)?;
    let bytes=tauri::async_runtime::spawn_blocking(move ||{
        let report=preview(&app);
        let events=serde_json::to_vec_pretty(&report["events"]).unwrap_or_default();
        let report=serde_json::to_vec_pretty(&report).unwrap_or_default();
        zip(&[("report.json",report),("events.json",events),("README.txt",b"Local diagnostic snapshot. Only allowlisted system/integration states are included. Raw logs, account names, player IDs, file paths, notes, tokens and cookies are excluded. Events are sanitized state summaries, not original logs. Share this file only if you choose.\r\n".to_vec())])
    }).await.map_err(|_|"diagnostics_unavailable")?;
    Ok(tauri::ipc::Response::new(bytes))
}

#[cfg(test)]
mod tests {
    #[test]
    fn companion_smoke_zip_has_standard_headers() {
        let bytes=super::zip(&[("report.json",b"{\"schema\":1}".to_vec()),("events.json",b"[]".to_vec())]);
        assert_eq!(&bytes[..4],b"PK\x03\x04");
        assert_eq!(&bytes[bytes.len()-22..bytes.len()-18],b"PK\x05\x06");
        if let Some(path)=std::env::var_os("KATSU_QA_ZIP") { std::fs::write(path,bytes).unwrap(); }
    }
}
