use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use tauri::image::Image;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};

use crate::libs::utils::get_theme_from_name;
use crate::plugins::config::ConfigManager;

pub const HERA_TRAY_ID: &str = "hera-tray";

static EXITING_EXPLICITLY: AtomicBool = AtomicBool::new(false);

fn last_tray_tooltip_slot() -> &'static Mutex<String> {
    static TOOLTIP: OnceLock<Mutex<String>> = OnceLock::new();
    TOOLTIP.get_or_init(|| Mutex::new(String::new()))
}

pub fn is_exiting_explicitly() -> bool {
    EXITING_EXPLICITLY.load(Ordering::SeqCst)
}

pub fn mark_exiting_explicitly() {
    EXITING_EXPLICITLY.store(true, Ordering::SeqCst);
}

pub fn is_window_visible_on_monitors(
    win_pos: (i32, i32),
    monitors: &[((i32, i32), (u32, u32))],
) -> bool {
    // Win32 minimized sentinel uses (-32000, -32000)
    if win_pos.0 <= -16_000 || win_pos.1 <= -16_000 {
        return false;
    }
    if monitors.is_empty() {
        return true;
    }
    let probe_x = win_pos.0 + 64;
    let probe_y = win_pos.1 + 32;
    monitors.iter().any(|&((mx, my), (mw, mh))| {
        let mr = mx.saturating_add(mw as i32);
        let mb = my.saturating_add(mh as i32);
        probe_x >= mx && probe_x < mr && probe_y >= my && probe_y < mb
    })
}

pub fn format_scanner_tray_tooltip(
    fast_done: usize,
    full_done: usize,
    total: usize,
    is_busy: bool,
) -> String {
    if is_busy && total > 0 {
        if fast_done < total {
            format!("HERA BETA · Escaneando biblioteca ({fast_done}/{total})")
        } else {
            format!("HERA BETA · Analizando Hot Cues ({full_done}/{total})")
        }
    } else if total > 0 {
        format!("HERA BETA · Biblioteca lista ({total} pistas)")
    } else {
        crate::plugins::hera_dj::format_now_playing_tray_tooltip(None)
    }
}

pub fn create_main_window<R: Runtime>(
    app: &AppHandle<R>,
) -> tauri::Result<tauri::WebviewWindow<R>> {
    let config_manager = app.state::<ConfigManager>();
    let theme_name = config_manager
        .get()
        .map(|c| c.theme)
        .unwrap_or_else(|_| "dark".to_string());

    let window_builder =
        WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
            .title("HERA Desktop")
            .visible(true)
            .focused(true)
            .theme(get_theme_from_name(&theme_name))
            .background_color(tauri::window::Color(12, 14, 18, 255))
            .inner_size(1180.0, 720.0)
            .min_inner_size(960.0, 580.0)
            .fullscreen(false)
            .resizable(true)
            .zoom_hotkeys_enabled(true);

    #[cfg(target_os = "windows")]
    let window = window_builder.disable_drag_drop_handler().build()?;

    #[cfg(target_os = "macos")]
    let window = window_builder
        .hidden_title(true)
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .traffic_light_position(tauri::Position::Logical(tauri::LogicalPosition {
            x: 10.0,
            y: 23.0,
        }))
        .build()?;

    #[cfg(target_os = "linux")]
    let window = window_builder.build()?;

    Ok(window)
}

pub fn restore_and_focus_main_window<R: Runtime>(app: &AppHandle<R>) {
    let window = match app.get_webview_window("main") {
        Some(win) => win,
        None => match create_main_window(app) {
            Ok(win) => win,
            Err(err) => {
                log::warn!("Failed to recreate main window: {err}");
                return;
            }
        },
    };

    let _ = window.set_skip_taskbar(false);
    if window.is_minimized().unwrap_or(false) {
        let _ = window.unminimize();
    }

    if let Ok(pos) = window.outer_position() {
        let monitors: Vec<((i32, i32), (u32, u32))> = window
            .available_monitors()
            .unwrap_or_default()
            .into_iter()
            .map(|m| {
                let p = m.position();
                let s = m.size();
                ((p.x, p.y), (s.width, s.height))
            })
            .collect();
        if !is_window_visible_on_monitors((pos.x, pos.y), &monitors) {
            let _ = window.center();
        }
    }

    let _ = window.show();

    let was_top = window.is_always_on_top().unwrap_or(false);
    if !was_top {
        let _ = window.set_always_on_top(true);
    }
    let _ = window.set_focus();
    if !was_top {
        let _ = window.set_always_on_top(false);
    }

    let _ = app.emit("hera:window-restored", ());
}

pub fn setup_system_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let open_item = MenuItemBuilder::with_id("tray_open", "Abrir HERA").build(app)?;
    let mini_item = MenuItemBuilder::with_id("tray_mini", "Modo Mini (Ctrl+Shift+M)").build(app)?;
    let cancel_item =
        MenuItemBuilder::with_id("tray_cancel_scan", "Cancelar escaneo").build(app)?;
    let quit_item = MenuItemBuilder::with_id("tray_quit", "Salir de HERA").build(app)?;

    let menu = MenuBuilder::new(app)
        .items(&[&open_item, &mini_item, &cancel_item])
        .separator()
        .item(&quit_item)
        .build()?;

    let initial_tooltip = crate::plugins::hera_dj::format_now_playing_tray_tooltip(None);
    let mut builder = TrayIconBuilder::with_id(HERA_TRAY_ID)
        .menu(&menu)
        .tooltip(&initial_tooltip)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| match event {
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            }
            | TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } => {
                restore_and_focus_main_window(tray.app_handle());
            }
            _ => {}
        })
        .on_menu_event(|app_handle, event| match event.id.as_ref() {
            "tray_open" => {
                restore_and_focus_main_window(app_handle);
            }
            "tray_mini" => {
                restore_and_focus_main_window(app_handle);
                let _ = app_handle.emit("hera:tray-toggle-mini", ());
            }
            "tray_cancel_scan" => {
                crate::plugins::hera_dj::workability::cancel_scan_flag();
                crate::plugins::hera_dj::scanner_v3::global_scheduler().clear_background_queues();
            }
            "tray_quit" => {
                mark_exiting_explicitly();
                app_handle.exit(0);
            }
            _ => {}
        });

    if let Some(icon) = app
        .default_window_icon()
        .cloned()
        .or_else(|| Image::from_bytes(include_bytes!("../../icons/32x32.png")).ok())
    {
        builder = builder.icon(icon);
    }

    let _tray = builder.build(app)?;
    if let Ok(mut guard) = last_tray_tooltip_slot().lock() {
        *guard = initial_tooltip;
    }
    Ok(())
}

pub fn update_tray_scanner_status<R: Runtime>(
    app: &AppHandle<R>,
    fast_done: usize,
    full_done: usize,
    total: usize,
    is_busy: bool,
) {
    let next = format_scanner_tray_tooltip(fast_done, full_done, total, is_busy);
    if let Ok(mut guard) = last_tray_tooltip_slot().lock() {
        if *guard == next {
            return;
        }
        *guard = next.clone();
    }
    if let Some(tray) = app.tray_by_id(HERA_TRAY_ID) {
        let _ = tray.set_tooltip(Some(next.as_str()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_monitor_visibility_rescues_minimized_and_offscreen_coords() {
        let monitors = [((0, 0), (1920, 1080))];
        assert!(!is_window_visible_on_monitors((-32000, -32000), &monitors));
        assert!(!is_window_visible_on_monitors((3000, 200), &monitors));
        assert!(is_window_visible_on_monitors((120, 80), &monitors));
    }

    #[test]
    fn test_format_scanner_tray_tooltip_states() {
        assert_eq!(
            format_scanner_tray_tooltip(40, 0, 200, true),
            "HERA BETA · Escaneando biblioteca (40/200)"
        );
        assert_eq!(
            format_scanner_tray_tooltip(200, 120, 200, true),
            "HERA BETA · Analizando Hot Cues (120/200)"
        );
        assert_eq!(
            format_scanner_tray_tooltip(200, 200, 200, false),
            "HERA BETA · Biblioteca lista (200 pistas)"
        );
        assert!(format_scanner_tray_tooltip(0, 0, 0, false).contains("HERA BETA"));
    }
}
