use crate::events::AppEvent;
use crate::hooks::HookManager;
use crate::win_util::{close_hwnd, open_folder_in_explorer, show_and_focus_hwnd};
use crossbeam_channel::Sender;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

pub struct TrayHandle {
    tray: TrayIcon,
    item_open: MenuItem,
    item_hud: MenuItem,
    item_folder: MenuItem,
    item_quit: MenuItem,
}

impl TrayHandle {
    pub fn create(
        tx: Sender<AppEvent>,
        egui_ctx: egui::Context,
        root_hwnd: isize,
        quitting: Arc<AtomicBool>,
        shared_folder: Arc<Mutex<PathBuf>>,
        hooks: HookManager,
        hidden_mode: bool,
    ) -> Result<Self, String> {
        let (lbl_open, lbl_hud, lbl_folder, lbl_quit, tooltip) = tray_labels(hidden_mode);

        let menu = Menu::new();
        let item_open = MenuItem::new(lbl_open, true, None);
        let item_hud = MenuItem::new(lbl_hud, true, None);
        let item_folder = MenuItem::new(lbl_folder, true, None);
        let item_quit = MenuItem::new(lbl_quit, true, None);

        let id_open = item_open.id().clone();
        let id_hud = item_hud.id().clone();
        let id_folder = item_folder.id().clone();
        let id_quit = item_quit.id().clone();

        menu.append(&item_open)
            .and_then(|_| menu.append(&item_hud))
            .and_then(|_| menu.append(&item_folder))
            .and_then(|_| menu.append(&item_quit))
            .map_err(|e| format!("Failed to build tray menu: {e}"))?;

        let icon = build_tray_icon(hidden_mode)?;
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_tooltip(tooltip)
            .with_icon(icon)
            .build()
            .map_err(|e| format!("Failed to create tray icon: {e}"))?;

        let tx_menu = tx.clone();
        let ctx_menu = egui_ctx.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if event.id == id_open {
                let _ = tx_menu.send(AppEvent::OpenSettings);
                show_and_focus_hwnd(root_hwnd);
                ctx_menu.request_repaint();
            } else if event.id == id_hud {
                let _ = tx_menu.send(AppEvent::OpenHudEditor);
                show_and_focus_hwnd(root_hwnd);
                ctx_menu.request_repaint();
            } else if event.id == id_folder {
                if let Ok(folder) = shared_folder.lock() {
                    open_folder_in_explorer(&folder);
                }
            } else if event.id == id_quit {
                quitting.store(true, Ordering::SeqCst);
                hooks.shutdown();
                let _ = tx_menu.send(AppEvent::Quit);
                close_hwnd(root_hwnd);
                ctx_menu.request_repaint();
            }
        }));

        TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = tx.send(AppEvent::OpenSettings);
                show_and_focus_hwnd(root_hwnd);
                egui_ctx.request_repaint();
            }
        }));

        Ok(Self {
            tray,
            item_open,
            item_hud,
            item_folder,
            item_quit,
        })
    }

    pub fn set_hidden_mode(&self, hidden_mode: bool) {
        let (lbl_open, lbl_hud, lbl_folder, lbl_quit, tooltip) = tray_labels(hidden_mode);
        self.item_open.set_text(lbl_open);
        self.item_hud.set_text(lbl_hud);
        self.item_folder.set_text(lbl_folder);
        self.item_quit.set_text(lbl_quit);
        let _ = self.tray.set_tooltip(Some(tooltip));
        if let Ok(icon) = build_tray_icon(hidden_mode) {
            let _ = self.tray.set_icon(Some(icon));
        }
    }
}

fn tray_labels(
    hidden_mode: bool,
) -> (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
) {
    if hidden_mode {
        (
            "Display Settings",
            "Calibration Layout",
            "Open Profiles Folder",
            "Exit",
            "Display Color Service",
        )
    } else {
        (
            "Open ScreenAsk",
            "Edit HUD Layout",
            "Open screenshot folder",
            "Quit",
            "ScreenAsk",
        )
    }
}

pub fn build_app_icon_rgba(size: u32) -> Vec<u8> {
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let s = size as f32;
    let radius = (s * 0.16).max(3.0);

    // Inner white rectangle outline bounds (matching the tray icon proportions)
    let rx0 = (s * 0.18).round();
    let rx1 = (s * 0.82).round();
    let ry0 = (s * 0.24).round();
    let ry1 = (s * 0.76).round();
    let stroke = (s * 0.08).max(2.0);

    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;

            let dx = (fx - s * 0.5).abs() - (s * 0.5 - radius);
            let dy = (fy - s * 0.5).abs() - (s * 0.5 - radius);
            let dist = dx.max(0.0).hypot(dy.max(0.0)) + dx.max(dy).min(0.0) - radius;

            let alpha_bg = (0.5 - dist).clamp(0.0, 1.0);
            if alpha_bg <= 0.0 {
                continue;
            }

            let in_outer = fx >= rx0 && fx <= rx1 && fy >= ry0 && fy <= ry1;
            let in_inner = fx >= (rx0 + stroke)
                && fx <= (rx1 - stroke)
                && fy >= (ry0 + stroke)
                && fy <= (ry1 - stroke);

            let idx = ((y * size + x) * 4) as usize;
            if in_outer && !in_inner {
                rgba[idx] = 245;
                rgba[idx + 1] = 245;
                rgba[idx + 2] = 245;
                rgba[idx + 3] = (alpha_bg * 255.0).round() as u8;
            } else {
                rgba[idx] = 24;
                rgba[idx + 1] = 24;
                rgba[idx + 2] = 26;
                rgba[idx + 3] = (alpha_bg * 255.0).round() as u8;
            }
        }
    }

    rgba
}

/// Neutral generic hardware/display-driver style icon for discreet tray mode.
fn build_discreet_icon_rgba(size: u32) -> Vec<u8> {
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let s = size as f32;

    // Minimal monochrome Windows-tray-style monitor + stand silhouette
    let mx0 = (s * 0.12).round();
    let mx1 = (s * 0.88).round();
    let my0 = (s * 0.16).round();
    let my1 = (s * 0.68).round();
    let border = (s * 0.08).max(2.0);

    let neck_x0 = (s * 0.44).round();
    let neck_x1 = (s * 0.56).round();
    let neck_y0 = my1;
    let neck_y1 = (s * 0.80).round();

    let base_x0 = (s * 0.28).round();
    let base_x1 = (s * 0.72).round();
    let base_y0 = (s * 0.78).round();
    let base_y1 = (s * 0.86).round();

    // Small diagonal calibration bar inside the monitor
    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;

            let in_mon_outer = fx >= mx0 && fx <= mx1 && fy >= my0 && fy <= my1;
            let in_mon_inner = fx >= (mx0 + border)
                && fx <= (mx1 - border)
                && fy >= (my0 + border)
                && fy <= (my1 - border);
            let on_mon_frame = in_mon_outer && !in_mon_inner;

            let on_neck = fx >= neck_x0 && fx <= neck_x1 && fy >= neck_y0 && fy <= neck_y1;
            let on_base = fx >= base_x0 && fx <= base_x1 && fy >= base_y0 && fy <= base_y1;

            // Subtle inner horizontal slider/calibration bar
            let on_bar = in_mon_inner
                && fx >= (s * 0.28)
                && fx <= (s * 0.72)
                && (fy - s * 0.42).abs() <= (s * 0.035);
            let on_knob = in_mon_inner
                && (fx - s * 0.58).hypot(fy - s * 0.42) <= (s * 0.075);

            let idx = ((y * size + x) * 4) as usize;
            if on_mon_frame || on_neck || on_base || on_knob {
                rgba[idx] = 220;
                rgba[idx + 1] = 224;
                rgba[idx + 2] = 230;
                rgba[idx + 3] = 255;
            } else if on_bar {
                rgba[idx] = 155;
                rgba[idx + 1] = 162;
                rgba[idx + 2] = 172;
                rgba[idx + 3] = 220;
            }
        }
    }

    rgba
}

fn build_tray_icon(hidden_mode: bool) -> Result<Icon, String> {
    const SIZE: u32 = 64;
    let rgba = if hidden_mode {
        build_discreet_icon_rgba(SIZE)
    } else {
        build_app_icon_rgba(SIZE)
    };
    Icon::from_rgba(rgba, SIZE, SIZE).map_err(|e| format!("Failed to create tray icon image: {e}"))
}
