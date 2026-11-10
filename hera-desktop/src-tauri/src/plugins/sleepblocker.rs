use std::sync::Mutex;

use log::{info, warn};
use nosleep::{NoSleep, NoSleepType};
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Manager, Runtime, State};

use crate::libs::error::AnyResult;
use crate::plugins::config::ConfigManager;

pub struct NoSleepInstance(Mutex<Option<NoSleep>>);

#[tauri::command]
pub fn enable(
    config_manager: State<ConfigManager>,
    nosleep: State<NoSleepInstance>,
) -> AnyResult<()> {
    config_manager.set_sleepblocker(true)?;

    if let Ok(mut guard) = nosleep.0.lock() {
        if let Some(ref mut instance) = *guard {
            instance.start(NoSleepType::PreventUserIdleSystemSleep)?;
            info!("Sleepblocker enabled");
        } else {
            info!("Sleepblocker enabled (no-op: NoSleep unavailable on this platform)");
        }
    }

    Ok(())
}

#[tauri::command]
pub fn disable(
    config_manager: State<ConfigManager>,
    nosleep: State<NoSleepInstance>,
) -> AnyResult<()> {
    config_manager.set_sleepblocker(false)?;

    if let Ok(mut guard) = nosleep.0.lock() {
        if let Some(ref mut instance) = *guard {
            instance.stop()?;
            info!("Sleepblocker disabled");
        } else {
            info!("Sleepblocker disabled (no-op: NoSleep unavailable on this platform)");
        }
    }

    Ok(())
}

/**
 * Plugin in charge of preventing the app from going to sleep
 */
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new("sleepblocker")
        .invoke_handler(tauri::generate_handler![enable, disable])
        .setup(|app_handle, _api| {
            // Add NoSleep to the app state, handling headless platforms where NoSleep::new() fails
            let nosleep_instance = match NoSleep::new() {
                Ok(instance) => Some(instance),
                Err(err) => {
                    warn!("NoSleep initialization failed (running headless or without sleep inhibitor): {}", err);
                    None
                }
            };
            app_handle.manage(NoSleepInstance(Mutex::new(nosleep_instance)));

            // Enable it right away
            let config_manager = app_handle.state::<ConfigManager>();
            let nosleep = app_handle.state::<NoSleepInstance>();

            if config_manager.get()?.sleepblocker {
                enable(config_manager, nosleep)?;
            }

            Ok(())
        })
        .build()
}
