use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Manager as _, Runtime};

/**
 * Plugin in charge of minimizing main window to System Tray on close when tray is active
 */
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("app-close")
        .on_window_ready(|win| {
            if win.label() != "main" {
                return;
            }
            let window = win.clone();
            win.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event
                    && !crate::libs::tray::is_exiting_explicitly()
                    && window
                        .app_handle()
                        .tray_by_id(crate::libs::tray::HERA_TRAY_ID)
                        .is_some()
                {
                    let _ = window.hide();
                    api.prevent_close();
                }
            });
        })
        .build()
}
