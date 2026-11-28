#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ai;
mod autostart;
mod capture;
mod config;
mod events;
mod fonts;
mod hooks;
mod pdf;
mod tray;
mod ui;
mod win_util;

use crate::config::load_config;
use crate::hooks::HookManager;
use crate::ui::app::ScreenAskApp;
use crossbeam_channel::unbounded;

fn main() -> eframe::Result<()> {
    #[cfg(windows)]
    let _instance_guard = match win_util::SingleInstanceGuard::acquire() {
        Some(guard) => guard,
        None => {
            // Another instance is already running.
            return Ok(());
        }
    };

    let cfg = load_config();
    let start_hidden =
        std::env::args().any(|a| a == "--background") && cfg.effective_api_key().is_some();

    let initial_size = if cfg.effective_api_key().is_none() {
        [515.0, 315.0]
    } else {
        [550.0, 395.0]
    };

    let (tx, rx) = unbounded();
    let hooks = HookManager::start(tx.clone(), &cfg);

    let app_icon = std::sync::Arc::new(egui::IconData {
        rgba: tray::build_app_icon_rgba(64),
        width: 64,
        height: 64,
    });

    let native_options = eframe::NativeOptions {
        centered: true,
        viewport: egui::ViewportBuilder::default()
            .with_title("ScreenAsk")
            .with_icon(app_icon)
            .with_decorations(false)
            .with_transparent(true)
            .with_inner_size(initial_size)
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "ScreenAsk",
        native_options,
        Box::new(move |cc| {
            Ok(Box::new(ScreenAskApp::new(
                cc,
                cfg,
                tx,
                rx,
                hooks,
                start_hidden,
            )))
        }),
    )
}
