//! Locate the actual game window by PID, independently of the tracker and focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonitorBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub fn placement(
    m: MonitorBounds,
    scale: f64,
    appearance: &crate::spike_appearance::Appearance,
) -> Placement {
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    let (logical_width, logical_height) = appearance.logical_size();
    let width = ((logical_width * scale).round() as u32).min(m.width);
    let height = ((logical_height * scale).round() as u32).min(m.height);
    let (cx, cy) = appearance
        .position
        .map(|p| (p.x * f64::from(m.width), p.y * f64::from(m.height)))
        .unwrap_or((
            f64::from(m.width) / 2.0,
            28.0 * scale + f64::from(height) / 2.0,
        ));
    let left = (cx - f64::from(width) / 2.0)
        .round()
        .clamp(0.0, f64::from(m.width - width));
    let top = (cy - f64::from(height) / 2.0)
        .round()
        .clamp(0.0, f64::from(m.height - height));
    Placement {
        x: m.x + left as i32,
        y: m.y + top as i32,
        width,
        height,
    }
}

pub fn target_scale(target: MonitorBounds, monitors: &[tauri::Monitor]) -> f64 {
    monitors
        .iter()
        .max_by_key(|m| {
            let left = target.x.max(m.position().x) as i64;
            let top = target.y.max(m.position().y) as i64;
            let right = (i64::from(target.x) + i64::from(target.width))
                .min(i64::from(m.position().x) + i64::from(m.size().width));
            let bottom = (i64::from(target.y) + i64::from(target.height))
                .min(i64::from(m.position().y) + i64::from(m.size().height));
            (right - left).max(0) * (bottom - top).max(0)
        })
        .map(|m| m.scale_factor())
        .unwrap_or(1.0)
}

/// Reassert only our own click-through window's z order; never alter the game.
pub fn keep_above_game(window: &tauri::WebviewWindow) {
    #[cfg(windows)]
    if let Ok(handle) = window.hwnd() {
        use windows::Win32::{
            Foundation::HWND,
            UI::WindowsAndMessaging::{
                SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER,
                SWP_NOSIZE,
            },
        };
        unsafe {
            let _ = SetWindowPos(
                HWND(handle.0),
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_NOOWNERZORDER,
            );
        }
    }
}

/// A missing/minimized game must not redirect a real countdown to the tracker.
pub fn select_target(
    game: Option<MonitorBounds>,
    demo: bool,
    fallback: Option<MonitorBounds>,
) -> Option<MonitorBounds> {
    game.or_else(|| if demo { fallback } else { None })
}

#[cfg(windows)]
pub fn game_monitor() -> Option<MonitorBounds> {
    use windows::Win32::{
        Foundation::{BOOL, HWND, LPARAM, POINT, RECT},
        Graphics::Gdi::ClientToScreen,
        UI::HiDpi::{SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2},
        UI::WindowsAndMessaging::{
            EnumWindows, GetClientRect, GetForegroundWindow, GetWindowRect,
            GetWindowThreadProcessId, IsIconic, IsWindowVisible,
        },
    };
    struct Search {
        pids: Vec<u32>,
        foreground: HWND,
        best: Option<(bool, i64, HWND)>,
    }
    unsafe extern "system" fn visit(window: HWND, param: LPARAM) -> BOOL {
        let search = &mut *(param.0 as *mut Search);
        let mut pid = 0;
        GetWindowThreadProcessId(window, Some(&mut pid));
        if !search.pids.contains(&pid)
            || !IsWindowVisible(window).as_bool()
            || IsIconic(window).as_bool()
        {
            return BOOL(1);
        }
        let mut rect = RECT::default();
        if GetWindowRect(window, &mut rect).is_err() {
            return BOOL(1);
        }
        let width = i64::from(rect.right) - i64::from(rect.left);
        let height = i64::from(rect.bottom) - i64::from(rect.top);
        if width <= 0 || height <= 0 {
            return BOOL(1);
        }
        let rank = (window == search.foreground, width * height);
        if search.best.is_none_or(|best| rank > (best.0, best.1)) {
            search.best = Some((rank.0, rank.1, window));
        }
        BOOL(1)
    }
    let pids = crate::process::game_process_ids();
    if pids.is_empty() {
        return None;
    }
    unsafe {
        let mut search = Search {
            pids,
            foreground: GetForegroundWindow(),
            best: None,
        };
        EnumWindows(Some(visit), LPARAM(&mut search as *mut Search as isize)).ok()?;
        let window = search.best?.2;
        // Capture physical client coordinates, including non-native fullscreen resolutions.
        let previous = SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let mut r = RECT::default();
        let mut origin = POINT::default();
        let ok =
            GetClientRect(window, &mut r).is_ok() && ClientToScreen(window, &mut origin).as_bool();
        if !previous.0.is_null() {
            SetThreadDpiAwarenessContext(previous);
        }
        if !ok || r.right <= r.left || r.bottom <= r.top {
            return None;
        }
        Some(MonitorBounds {
            x: origin.x,
            y: origin.y,
            width: (r.right - r.left) as u32,
            height: (r.bottom - r.top) as u32,
        })
    }
}

#[cfg(not(windows))]
pub fn game_monitor() -> Option<MonitorBounds> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    const PRIMARY: MonitorBounds = MonitorBounds {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };
    const LEFT: MonitorBounds = MonitorBounds {
        x: -2560,
        y: -200,
        width: 2560,
        height: 1440,
    };
    #[test]
    fn game_monitor_wins_over_tracker_and_follows_moves() {
        assert_eq!(select_target(Some(LEFT), false, Some(PRIMARY)), Some(LEFT));
        assert_eq!(
            select_target(Some(PRIMARY), false, Some(LEFT)),
            Some(PRIMARY)
        );
        assert_eq!(select_target(Some(LEFT), true, Some(PRIMARY)), Some(LEFT));
    }
    #[test]
    fn missing_or_minimized_game_only_allows_demo_fallback() {
        assert_eq!(select_target(None, false, Some(PRIMARY)), None);
        assert_eq!(select_target(None, true, Some(PRIMARY)), Some(PRIMARY));
        assert_eq!(select_target(None, true, None), None);
    }
    #[test]
    fn placement_handles_negative_coordinates_and_mixed_dpi() {
        assert_eq!(
            placement(PRIMARY, 1.0, &Default::default()),
            Placement {
                x: 860,
                y: 28,
                width: 200,
                height: 46
            }
        );
        assert_eq!(
            placement(LEFT, 1.5, &Default::default()),
            Placement {
                x: -1430,
                y: -158,
                width: 300,
                height: 69
            }
        );
        let above = MonitorBounds {
            x: 0,
            y: -2160,
            width: 3840,
            height: 2160,
        };
        assert_eq!(
            placement(above, 2.0, &Default::default()),
            Placement {
                x: 1720,
                y: -2104,
                width: 400,
                height: 92
            }
        );
    }
    #[test]
    fn placement_stays_inside_small_monitor_and_rejects_invalid_scale() {
        let small = MonitorBounds {
            x: 20,
            y: 30,
            width: 100,
            height: 60,
        };
        assert_eq!(
            placement(small, 2.0, &Default::default()),
            Placement {
                x: 20,
                y: 30,
                width: 100,
                height: 60
            }
        );
        assert_eq!(
            placement(PRIMARY, f64::NAN, &Default::default()),
            placement(PRIMARY, 1.0, &Default::default())
        );
    }
    #[test]
    fn custom_center_scales_and_clamps_across_resolutions() {
        use crate::spike_appearance::{Appearance, Point};
        let a = Appearance {
            position: Some(Point { x: 0.25, y: 0.7 }),
            font_size: 60.0,
            ..Default::default()
        };
        for (w, h, dpi) in [
            (1280, 720, 1.0),
            (1920, 1080, 1.25),
            (3840, 2160, 2.0),
            (1024, 768, 1.0),
        ] {
            let m = MonitorBounds {
                x: -4000,
                y: -500,
                width: w,
                height: h,
            };
            let p = placement(m, dpi, &a);
            assert!(((p.x - m.x) as f64 + p.width as f64 / 2.0 - w as f64 * 0.25).abs() <= 1.0);
            assert!(((p.y - m.y) as f64 + p.height as f64 / 2.0 - h as f64 * 0.7).abs() <= 1.0);
        }
        let p = placement(
            PRIMARY,
            1.0,
            &Appearance {
                position: Some(Point { x: 1.0, y: 0.0 }),
                ..a
            },
        );
        assert_eq!(p.x + p.width as i32, 1920);
        assert_eq!(p.y, 0);
    }
    #[test]
    #[ignore = "read-only check requiring a visible running VALORANT window"]
    fn live_game_monitor_probe() {
        let monitor = game_monitor().expect("visible VALORANT window required");
        assert!(monitor.width > 0 && monitor.height > 0);
        println!("Detected VALORANT monitor: {monitor:?}");
    }
}
