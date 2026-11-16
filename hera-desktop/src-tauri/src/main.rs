// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod libs;
mod plugins;

use libs::init::init;
use log::{LevelFilter, info};
use plugins::config::get_storage_dir;
use std::env;
use tauri_plugin_log::fern::colors::ColoredLevelConfig;
use tauri_plugin_log::{Target, TargetKind};
use tauri_plugin_prevent_default::Flags;
use tauri_plugin_window_state::StateFlags;

/**
 * The beast
 */
fn main() {
    let config = match init() {
        Ok(config) => {
            println!("[init] Initialization successful");
            config
        }
        Err(e) => {
            println!("[init] Error during initialization: {}", e);
            std::process::exit(1);
        }
    };

    let enable_stream_server = match config.get() {
        Ok(conf) => conf.audio_stream_server,
        Err(e) => {
            println!("[init] Error while loading stream-server setting: {}", e);
            std::process::exit(1);
        }
    };

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        // Logging must be setup first, otherwise the logs won't be captured
        // while setting up the other plugins.
        .plugin(
            tauri_plugin_log::Builder::default()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::Webview),
                    Target::new(TargetKind::Folder {
                        path: get_storage_dir(),
                        file_name: Some("hera".into()),
                    }),
                ])
                .level(LevelFilter::Info)
                // Prevent noisy logs from logging
                .level_for("lofty", LevelFilter::Error)
                .level_for("symphonia_bundle_mp3", LevelFilter::Error)
                .level_for("symphonia_format_isomp4", LevelFilter::Error)
                .level_for("symphonia_core", LevelFilter::Error)
                .level_for("sqlx::query", LevelFilter::Error)
                .with_colors(ColoredLevelConfig::default())
                .max_file_size(50_000)
                .build(),
        )
        // Ensure a single instance of the app is running, if needed, play opened files
        .plugin(tauri_plugin_single_instance::init(|app_handle, args, _| {
            libs::tray::restore_and_focus_main_window(app_handle);
            plugins::file_associations::handle_opened_files(app_handle, args);
        }))
        // Custom integrations
        .plugin(plugins::app_close::init())
        .plugin(plugins::config::init(config))
        .plugin(plugins::app_menu::init())
        .plugin(plugins::cover::init())
        .plugin(plugins::db::init())
        .plugin(plugins::debug::init())
        .plugin(plugins::default_view::init())
        .plugin(plugins::hera_dj::init())
        .plugin(plugins::hera_agent::init())
        .plugin(plugins::file_associations::init())
        .plugin(plugins::sleepblocker::init())
        // Tauri integrations with the Operating System
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .plugin(
            tauri_plugin_prevent_default::Builder::new()
                .with_flags(Flags::all().difference(Flags::FOCUS_MOVE))
                .build(),
        )
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(
                    StateFlags::all() & !StateFlags::VISIBLE, // Museeks manages its visible state by itself
                )
                .build(),
        );

    // Experimental: local HTTP server for audio streaming
    // (WebKitGTK's asset protocol doesn't support media streaming)
    let builder = if enable_stream_server {
        builder.plugin(plugins::stream_server::init())
    } else {
        builder
    };

    #[cfg(any(debug_assertions, feature = "mcp-bridge"))]
    let builder = builder.plugin(
        tauri_plugin_mcp_bridge::Builder::new()
            .bind_address("127.0.0.1")
            .build(),
    );

    builder
        .setup(|app| {
            libs::tray::create_main_window(app.handle())?;
            if let Err(err) = libs::tray::setup_system_tray(app.handle()) {
                log::warn!("System tray setup warning: {err}");
            }

            info!("Main window built");

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        // Handle file associations on macOS, this will get triggered when the
        // app starts, AND when the app is already running and a file is opened.
        .run(
            #[allow(unused_variables)]
            |app_handle, event| {
                #[cfg(target_os = "macos")]
                plugins::file_associations::handle_run_event(app_handle, event);
            },
        );
}
