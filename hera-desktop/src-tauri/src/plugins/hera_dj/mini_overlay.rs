use super::*;

#[derive(Debug, Clone)]
pub(super) struct NormalWindowSnapshot {
    pub(super) outer_pos: Option<tauri::PhysicalPosition<i32>>,
    pub(super) inner_size: tauri::PhysicalSize<u32>,
    pub(super) is_maximized: bool,
}

fn normal_window_snapshot_slot() -> &'static Mutex<Option<NormalWindowSnapshot>> {
    static SNAPSHOT: OnceLock<Mutex<Option<NormalWindowSnapshot>>> = OnceLock::new();
    SNAPSHOT.get_or_init(|| Mutex::new(None))
}

fn current_overlay_mode_slot() -> &'static Mutex<String> {
    static MODE: OnceLock<Mutex<String>> = OnceLock::new();
    MODE.get_or_init(|| Mutex::new("full".to_string()))
}

#[tauri::command]
pub async fn set_mini_overlay_mode<R: Runtime>(
    app: tauri::AppHandle<R>,
    request: SetMiniOverlayModeRequest,
) -> Result<SetMiniOverlayModeResponse, String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "No se encontró la ventana principal".to_string())?;

    let target_mode = match request.mode.trim().to_ascii_lowercase().as_str() {
        "pill" => "pill",
        "card" => "card",
        _ => "full",
    };

    let prev_mode = current_overlay_mode_slot()
        .lock()
        .map(|g| g.clone())
        .unwrap_or_else(|_| "full".to_string());

    if target_mode == "full" {
        let _ = window.set_always_on_top(false);
        let _ = window.set_decorations(true);
        let _ = window.set_min_size(Some(tauri::Size::Logical(tauri::LogicalSize::new(
            960.0, 580.0,
        ))));

        let saved = normal_window_snapshot_slot()
            .lock()
            .ok()
            .and_then(|mut g| g.take());

        let (restored_w, restored_h) = if let Some(snap) = saved {
            let scale = window.scale_factor().unwrap_or(1.0);
            let _ = window.set_size(tauri::Size::Physical(snap.inner_size));
            if let Some(pos) = snap.outer_pos {
                let _ = window.set_position(tauri::Position::Physical(pos));
            }
            if snap.is_maximized {
                let _ = window.maximize();
            }
            (
                (snap.inner_size.width as f64 / scale).round(),
                (snap.inner_size.height as f64 / scale).round(),
            )
        } else {
            let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize::new(1180.0, 720.0)));
            (1180.0, 720.0)
        };

        if let Ok(mut g) = current_overlay_mode_slot().lock() {
            *g = "full".to_string();
        }

        return Ok(SetMiniOverlayModeResponse {
            mode: "full".to_string(),
            pinned: false,
            width: restored_w,
            height: restored_h,
        });
    }

    if prev_mode == "full" {
        let snap = NormalWindowSnapshot {
            outer_pos: window.outer_position().ok(),
            inner_size: window
                .inner_size()
                .unwrap_or(tauri::PhysicalSize::new(1180, 720)),
            is_maximized: window.is_maximized().unwrap_or(false),
        };
        if snap.is_maximized {
            let _ = window.unmaximize();
        }
        if let Ok(mut g) = normal_window_snapshot_slot().lock() {
            *g = Some(snap);
        }
    }

    let (target_w, target_h) = if target_mode == "pill" {
        (560.0_f64, 52.0_f64)
    } else {
        (360.0_f64, 408.0_f64)
    };

    let scale = window.scale_factor().unwrap_or(1.0);
    let phys_w = (target_w * scale).round() as i32;
    let phys_h = (target_h * scale).round() as i32;

    let next_pos = if prev_mode != "full" && prev_mode != target_mode {
        if let (Ok(pos), Ok(sz)) = (window.outer_position(), window.inner_size()) {
            let right_edge = pos.x + sz.width as i32;
            let bottom_edge = pos.y + sz.height as i32;
            Some(tauri::PhysicalPosition::new(
                (right_edge - phys_w).max(12),
                (bottom_edge - phys_h).max(12),
            ))
        } else {
            None
        }
    } else if prev_mode == "full" {
        window.current_monitor().ok().flatten().map(|mon| {
            let m_pos = mon.position();
            let m_size = mon.size();
            let margin_x = (24.0 * scale).round() as i32;
            let margin_y = (56.0 * scale).round() as i32;
            tauri::PhysicalPosition::new(
                (m_pos.x + m_size.width as i32 - phys_w - margin_x).max(m_pos.x + 12),
                (m_pos.y + m_size.height as i32 - phys_h - margin_y).max(m_pos.y + 12),
            )
        })
    } else {
        None
    };

    let _ = window.set_decorations(false);
    let _ = window.set_always_on_top(request.pinned);
    let _ = window.set_min_size(Some(tauri::Size::Logical(tauri::LogicalSize::new(
        320.0, 48.0,
    ))));
    let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize::new(
        target_w, target_h,
    )));
    if let Some(pos) = next_pos {
        let _ = window.set_position(tauri::Position::Physical(pos));
    }

    if let Ok(mut g) = current_overlay_mode_slot().lock() {
        *g = target_mode.to_string();
    }

    Ok(SetMiniOverlayModeResponse {
        mode: target_mode.to_string(),
        pinned: request.pinned,
        width: target_w,
        height: target_h,
    })
}
