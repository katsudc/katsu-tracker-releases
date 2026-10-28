//! Read the user's selected display mode without touching the game or its files.
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenMode {
    Fullscreen,
    Borderless,
    Windowed,
    #[default]
    Unknown,
}

pub fn parse(text: &str) -> ScreenMode {
    let mut section = false;
    let mut confirmed = ScreenMode::Unknown;
    for raw in text.trim_start_matches('\u{feff}').lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            section = line == "[/Script/ShooterGame.ShooterGameUserSettings]";
            continue;
        }
        if !section || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let mode = match value.trim() {
                    "0" => ScreenMode::Fullscreen,
                    "1" => ScreenMode::Borderless,
                    "2" => ScreenMode::Windowed,
                    _ => ScreenMode::Unknown,
                };
            match key.trim() {
                "FullscreenMode" => return mode,
                // The live game's shared config can omit FullscreenMode while
                // retaining the applied mode here. Never use PreferredFullscreenMode.
                "LastConfirmedFullscreenMode" => confirmed = mode,
                _ => {}
            }
        }
    }
    confirmed
}
fn read(path: &Path) -> Option<(SystemTime, ScreenMode)> {
    let meta = std::fs::metadata(path).ok()?;
    if meta.len() > 1024 * 1024 {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    let mode = parse(&text);
    (mode != ScreenMode::Unknown).then(|| (meta.modified().unwrap_or(SystemTime::UNIX_EPOCH), mode))
}
pub fn selected(root: &Path, puuid: &str) -> ScreenMode {
    let mut paths = vec![root.join("WindowsClient/GameUserSettings.ini")];
    if uuid::Uuid::parse_str(puuid).is_ok() {
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten().take(128) {
                let name = entry.file_name();
                let Some(name) = name.to_str() else {
                    continue;
                };
                if name.starts_with(&format!("{puuid}-"))
                    && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
                {
                    paths.push(entry.path().join("WindowsClient/GameUserSettings.ini"));
                }
            }
        }
    }
    paths
        .iter()
        .filter_map(|p| read(p))
        .max_by_key(|v| v.0)
        .map_or(ScreenMode::Unknown, |v| v.1)
}
pub fn config_root() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|p| PathBuf::from(p).join("VALORANT/Saved/Config"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_only_actual_setting_and_section() {
        for (value, mode) in [
            (0, ScreenMode::Fullscreen),
            (1, ScreenMode::Borderless),
            (2, ScreenMode::Windowed),
            (9, ScreenMode::Unknown),
        ] {
            let input=format!("[Other]\nFullscreenMode=0\n[/Script/ShooterGame.ShooterGameUserSettings]\nLastConfirmedFullscreenMode=2\nPreferredFullscreenMode=2\nFullscreenMode={value}\n");
            assert_eq!(parse(&input), mode);
        }
        assert_eq!(parse("[Other]\nFullscreenMode=0"), ScreenMode::Unknown);
        assert_eq!(parse("[/Script/ShooterGame.ShooterGameUserSettings]\n;FullscreenMode=0\nFullscreenMode=1"),ScreenMode::Borderless);
    }
    #[test]
    fn omitted_mode_uses_confirmed_setting_but_never_preference() {
        assert_eq!(parse("[/Script/ShooterGame.ShooterGameUserSettings]\nPreferredFullscreenMode=0\nLastConfirmedFullscreenMode=0"), ScreenMode::Fullscreen);
        assert_eq!(parse("[/Script/ShooterGame.ShooterGameUserSettings]\nLastConfirmedFullscreenMode=0\nFullscreenMode=1"), ScreenMode::Borderless);
        assert_eq!(parse("[/Script/ShooterGame.ShooterGameUserSettings]\nPreferredFullscreenMode=0"), ScreenMode::Unknown);
        assert_eq!(parse("[Other]\nLastConfirmedFullscreenMode=0"), ScreenMode::Unknown);
    }
    #[test]
    fn excludes_another_accounts_profile() {
        let root = std::env::temp_dir().join(format!("katsu-screen-mode-{}", uuid::Uuid::new_v4()));
        let own = "11111111-1111-4111-8111-111111111111";
        let other = "22222222-2222-4222-8222-222222222222";
        for (id, mode) in [(own, 1), (other, 0)] {
            let path = root.join(format!("{id}-eu/WindowsClient"));
            std::fs::create_dir_all(&path).unwrap();
            std::fs::write(
                path.join("GameUserSettings.ini"),
                format!("[/Script/ShooterGame.ShooterGameUserSettings]\nFullscreenMode={mode}"),
            )
            .unwrap();
        }
        assert_eq!(selected(&root, own), ScreenMode::Borderless);
        assert_eq!(selected(&root, "../../not-a-profile"), ScreenMode::Unknown);
        // Every temporary file is explicitly owned by this test.
        for id in [own, other] {
            let path = root.join(format!("{id}-eu/WindowsClient"));
            std::fs::remove_file(path.join("GameUserSettings.ini")).unwrap();
            std::fs::remove_dir(path).unwrap();
            std::fs::remove_dir(root.join(format!("{id}-eu"))).unwrap();
        }
        std::fs::remove_dir(root).unwrap();
    }
}
