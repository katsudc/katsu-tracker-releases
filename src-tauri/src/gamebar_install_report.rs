//! Bounded, allowlisted installation diagnostics. Never stores helper output or user paths.
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};

pub const PREFIX: &str = "KATSU_WIDGET_SETUP:";
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct InstallReport {
    pub schema: u32,
    pub stage: String,
    pub code: String,
    pub hresult: Option<String>,
    pub activity_id: Option<String>,
    pub exit_code: Option<i32>,
    pub runtime_present: Option<bool>,
    pub publisher_trusted: Option<bool>,
}
fn safe_code(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64 && s.bytes().all(|c| c.is_ascii_lowercase() || c == b'_')
}
fn valid_hex(s: &str, count: usize) -> bool { s.len() == count && s.bytes().all(|c| c.is_ascii_hexdigit()) }
impl InstallReport {
    pub fn new(stage: &str, code: &str) -> Self { Self { schema: 1, stage: stage.into(), code: code.into(), ..Default::default() } }
    pub fn valid(&self) -> bool {
        self.schema == 1 && safe_code(&self.stage) && safe_code(&self.code)
            && self.hresult.as_ref().is_none_or(|s| s.starts_with("0x") && valid_hex(&s[2..], 8))
            && self.activity_id.as_ref().is_none_or(|s| uuid::Uuid::parse_str(s).is_ok() && s.len() == 36)
    }
}
pub fn parse(stdout: &[u8], stderr: &[u8], exit: Option<i32>) -> InstallReport {
    for line in String::from_utf8_lossy(stdout).lines().rev() {
        if let Some(json) = line.strip_prefix(PREFIX).filter(|s| s.len() < 4096) {
            if let Ok(mut value) = serde_json::from_str::<InstallReport>(json) {
                if value.valid() { value.exit_code = exit; return value; }
            }
        }
    }
    let code = match exit { Some(3) => "publisher_trust_required", Some(4) => "administrator_required", Some(5) => "setup_cancelled", Some(0) => "helper_completed", _ => "helper_failed" };
    let mut value = InstallReport::new("powershell_helper", code); value.exit_code = exit;
    // A script blocked before its catch handler may not emit JSON. Extract only a numeric HRESULT.
    let raw = String::from_utf8_lossy(stderr);
    value.hresult = raw.as_bytes().windows(10).find(|w| &w[..2] == b"0x" && w[2..].iter().all(u8::is_ascii_hexdigit)).map(|w| String::from_utf8_lossy(w).into_owned());
    value
}
pub fn load(app: &tauri::AppHandle) -> Option<InstallReport> {
    let path = app.path().app_data_dir().ok()?.join("gamebar-setup-report.json");
    if std::fs::metadata(&path).ok()?.len() > 4096 { return None; }
    let value: InstallReport = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    value.valid().then_some(value)
}
pub fn record(app: &tauri::AppHandle, report: InstallReport) {
    if !report.valid() { return; }
    tracing::info!("[Game Bar setup] stage={} code={} hresult={:?} exit={:?}", report.stage, report.code, report.hresult, report.exit_code);
    if let Ok(dir) = app.path().app_data_dir() {
        let temp = dir.join("gamebar-setup-report.json.tmp");
        if let Ok(bytes) = serde_json::to_vec_pretty(&report) {
            if std::fs::write(&temp, bytes).is_ok() { let _ = std::fs::rename(temp, dir.join("gamebar-setup-report.json")); }
        }
    }
    let _ = app.emit("game-bar-setup-report", &report);
}
#[tauri::command]
pub fn get_game_bar_setup_report(window: tauri::WebviewWindow, app: tauri::AppHandle) -> Result<Option<InstallReport>, String> {
    if window.label() != "main" { return Err("window".into()); }
    Ok(load(&app))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installer_report_keeps_hresult_but_excludes_raw_paths() {
        let output = b"other output\nKATSU_WIDGET_SETUP:{\"schema\":1,\"stage\":\"register_widget\",\"code\":\"dependency_missing\",\"hresult\":\"0x80073CF3\",\"activity_id\":\"12345678-1234-1234-1234-123456789abc\",\"runtime_present\":false}";
        let value = parse(output, b"C:\\Users\\private\\secret", Some(1));
        assert_eq!(value.code, "dependency_missing"); assert_eq!(value.hresult.as_deref(), Some("0x80073CF3"));
        assert!(!serde_json::to_string(&value).unwrap().contains("private"));
        let blocked = parse(b"", b"Blocked at private path with 0x80070005", Some(1));
        assert_eq!(blocked.stage, "powershell_helper"); assert_eq!(blocked.hresult.as_deref(), Some("0x80070005"));
        assert!(!InstallReport::new("C:\\private", "failure").valid());
    }
}
