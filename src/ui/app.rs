use crate::autostart::set_autostart;
use crate::capture::{capture_monitor, encode_for_gemini, save_screenshot_png};
use crate::config::{save_config, Config};
use crate::events::{AppEvent, RecordingTarget};
use crate::fonts::{apply_font, discover_available_fonts};
use crate::gemini::ask_gemini;
use crate::hooks::HookManager;
use crate::pdf::load_pdf_sources;
use crate::tray::TrayHandle;
use crate::ui::hud_editor::{render_hud_editor_viewport, HudEditorOutcome, HudEditorState};
use crate::ui::overlay::{render_overlay_viewport, OverlayState};
use crate::ui::settings::{SettingsAction, SettingsForm};
use crate::ui::theme::apply_blur_theme;
use crate::win_util::{
    focused_monitor_info, get_window_frame_overhead, get_window_outer_rect,
    open_folder_in_explorer, resize_hwnd_anchored_top_left, show_and_focus_hwnd,
};
use crossbeam_channel::{Receiver, Sender};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub struct ScreenAskApp {
    cfg: Config,
    form: SettingsForm,
    tx: Sender<AppEvent>,
    rx: Receiver<AppEvent>,
    hooks: HookManager,
    tray: Option<TrayHandle>,
    overlay: Option<OverlayState>,
    hud_editor: Option<HudEditorState>,
    saved_root_rect_before_hud: Option<(i32, i32, i32, i32)>,
    root_hwnd: isize,
    shared_folder: Arc<Mutex<PathBuf>>,
    pending_test_capture: Option<Instant>,
    capture_in_flight: Arc<AtomicBool>,
    quitting: Arc<AtomicBool>,
    hide_after_init: bool,
    init_frames: u8,
    cur_win_size: (f32, f32),
    resize_anchor_tl: Option<(i32, i32)>,
    last_frame_at: Instant,
}

impl ScreenAskApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        cfg: Config,
        tx: Sender<AppEvent>,
        rx: Receiver<AppEvent>,
        hooks: HookManager,
        start_hidden: bool,
    ) -> Self {
        hooks.set_egui_ctx(cc.egui_ctx.clone());

        let available_fonts = discover_available_fonts();
        apply_font(&cc.egui_ctx, &cfg.font_name, &available_fonts);
        apply_blur_theme(&cc.egui_ctx, cfg.light_mode, cfg.accent_color);

        let root_hwnd = extract_win32_hwnd(cc);
        let quitting = Arc::new(AtomicBool::new(false));
        let shared_folder = Arc::new(Mutex::new(PathBuf::from(&cfg.folder)));

        let tray = match TrayHandle::create(
            tx.clone(),
            cc.egui_ctx.clone(),
            root_hwnd,
            Arc::clone(&quitting),
            Arc::clone(&shared_folder),
            hooks.clone(),
            cfg.hidden_mode,
        ) {
            Ok(t) => Some(t),
            Err(e) => {
                eprintln!("Tray icon error: {e}");
                None
            }
        };

        let form = SettingsForm::new(cfg.clone(), available_fonts);
        let cur_win_size = form.target_window_size();

        Self {
            cfg,
            form,
            tx,
            rx,
            hooks,
            tray,
            overlay: None,
            hud_editor: None,
            saved_root_rect_before_hud: None,
            root_hwnd,
            shared_folder,
            pending_test_capture: None,
            capture_in_flight: Arc::new(AtomicBool::new(false)),
            quitting,
            hide_after_init: start_hidden,
            init_frames: 0,
            cur_win_size,
            resize_anchor_tl: None,
            last_frame_at: Instant::now(),
        }
    }

    fn save_form_config(&mut self, ctx: &egui::Context) -> bool {
        self.form.cfg.sanitize();
        self.form.refresh_pdf_count();
        match save_config(&self.form.cfg) {
            Ok(()) => {
                self.cfg = self.form.cfg.clone();
                if let Ok(mut f) = self.shared_folder.lock() {
                    *f = PathBuf::from(&self.cfg.folder);
                }
                self.hooks.update_config(&self.cfg);
                apply_font(ctx, &self.cfg.font_name, &self.form.available_fonts);
                apply_blur_theme(ctx, self.cfg.light_mode, self.cfg.accent_color);

                if let Err(err) = set_autostart(self.form.autostart) {
                    self.form
                        .set_status(format!("Saved, but autostart failed: {err}"), true);
                } else {
                    self.form
                        .set_status("Saved. Running in background.", false);
                }
                true
            }
            Err(e) => {
                self.form.set_status(e, true);
                false
            }
        }
    }

    fn auto_save_customization(&mut self) {
        self.form.cfg.sanitize();
        self.cfg = self.form.cfg.clone();
        if let Ok(mut f) = self.shared_folder.lock() {
            *f = PathBuf::from(&self.cfg.folder);
        }
        self.hooks.update_config(&self.cfg);
        let _ = save_config(&self.cfg);
    }

    fn open_hud_editor(&mut self, ctx: &egui::Context) {
        self.close_overlay();
        self.form.color_picker.open = false;
        self.resize_anchor_tl = None;

        // Capture the focused monitor BEFORE moving the Settings window off-screen.
        let monitor = focused_monitor_info();

        // Park the root window off-screen (instead of Visible(false)) so eframe keeps
        // pumping updates for the HUD Editor while the desktop stays completely unobstructed.
        if let Some((l, t, r, b)) = get_window_outer_rect(self.root_hwnd) {
            self.saved_root_rect_before_hud = Some((l, t, (r - l).max(100), (b - t).max(100)));
            resize_hwnd_anchored_top_left(
                self.root_hwnd,
                -32000,
                -32000,
                (r - l).max(100),
                (b - t).max(100),
            );
            thread::sleep(Duration::from_millis(45));
        }

        // Capture the live screen of the focused monitor's work area so the HUD Editor
        // displays the real desktop/apps behind the preview box instead of a black screen.
        let backdrop_img = capture_monitor(monitor.work).ok();

        self.hooks.set_hud_editing(true);
        self.hud_editor = Some(HudEditorState::new(monitor, &self.form.cfg, backdrop_img));
        ctx.request_repaint();
    }

    fn close_hud_editor(&mut self, ctx: &egui::Context) {
        self.hud_editor = None;
        self.hooks.set_hud_editing(false);
        if let Some((l, t, w, h)) = self.saved_root_rect_before_hud.take() {
            resize_hwnd_anchored_top_left(self.root_hwnd, l, t, w, h);
            self.show_settings(ctx);
        }
    }

    fn hide_settings(&mut self, ctx: &egui::Context) {
        self.form.recording = None;
        self.form.color_picker.open = false;
        self.hooks.cancel_recording();
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
    }

    fn show_settings(&mut self, ctx: &egui::Context) {
        self.hide_after_init = false;
        self.resize_anchor_tl = None;
        self.form.refresh_pdf_count();
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        show_and_focus_hwnd(self.root_hwnd);
    }

    fn close_overlay(&mut self) {
        self.overlay = None;
        self.hooks.set_overlay_visible(false);
    }

    fn copy_and_close_overlay(&mut self) {
        if let Some(ov) = &self.overlay {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(ov.text.clone());
            }
        }
        self.close_overlay();
    }

    fn update_smooth_window_resize(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        let dt = (now - self.last_frame_at).as_secs_f32().clamp(0.001, 0.05);
        self.last_frame_at = now;

        let target = self.form.target_window_size();
        let dw = target.0 - self.cur_win_size.0;
        let dh = target.1 - self.cur_win_size.1;

        if dw.abs() <= 0.5 && dh.abs() <= 0.5 {
            self.resize_anchor_tl = None;
            return;
        }

        // Lock the current physical top-left corner at the start of a resize transition.
        let anchor = match self.resize_anchor_tl {
            Some(a) => a,
            None => {
                let Some((left, top, _right, _bottom)) = get_window_outer_rect(self.root_hwnd)
                else {
                    return;
                };
                self.resize_anchor_tl = Some((left, top));
                (left, top)
            }
        };

        let t = 1.0 - (-18.0 * dt).exp();
        let next_w = if dw.abs() < 0.8 {
            target.0
        } else {
            self.cur_win_size.0 + dw * t
        };
        let next_h = if dh.abs() < 0.8 {
            target.1
        } else {
            self.cur_win_size.1 + dh * t
        };

        self.cur_win_size = (next_w, next_h);

        let ppp = ctx.pixels_per_point().max(0.5);
        let (extra_w, extra_h) = get_window_frame_overhead(self.root_hwnd);
        let outer_w = (next_w * ppp).round() as i32 + extra_w;
        let outer_h = (next_h * ppp).round() as i32 + extra_h;

        resize_hwnd_anchored_top_left(self.root_hwnd, anchor.0, anchor.1, outer_w, outer_h);

        if (target.0 - next_w).abs() > 0.5 || (target.1 - next_h).abs() > 0.5 {
            ctx.request_repaint();
        } else {
            self.resize_anchor_tl = None;
        }
    }

    fn trigger_capture(&mut self, ctx: &egui::Context) {
        if self.form.recording.is_some() {
            return;
        }
        if self
            .capture_in_flight
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return;
        }

        self.close_overlay();

        let cfg = self.cfg.clone();
        let tx = self.tx.clone();
        let ctx_clone = ctx.clone();
        let in_flight = Arc::clone(&self.capture_in_flight);

        thread::spawn(move || {
            thread::sleep(Duration::from_millis(60));

            let monitor = focused_monitor_info();
            let result = (|| -> Result<String, String> {
                let img = capture_monitor(monitor.monitor)?;
                let folder = PathBuf::from(&cfg.folder);
                if cfg.save_screenshots {
                    let _ = save_screenshot_png(&img, &folder);
                }
                let pdf_context = load_pdf_sources(&folder);
                let jpeg = encode_for_gemini(&img)?;
                ask_gemini(&cfg, &jpeg, &pdf_context)
            })();

            let text = match result {
                Ok(answer) => answer,
                Err(err) => format!("Error: {err}"),
            };

            in_flight.store(false, Ordering::SeqCst);
            let _ = tx.send(AppEvent::ShowOverlay {
                text,
                monitor: Some(monitor),
            });
            ctx_clone.request_repaint();
        });
    }

    fn drain_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                AppEvent::Capture => {
                    self.trigger_capture(ctx);
                }
                AppEvent::ShowOverlay { text, monitor } => {
                    let mon = monitor.unwrap_or_else(focused_monitor_info);
                    self.overlay = Some(OverlayState::new(ctx, text, mon, &self.cfg));
                    self.hooks.set_overlay_visible(true);
                }
                AppEvent::CloseOverlay => {
                    self.close_overlay();
                }
                AppEvent::CopyAndClose => {
                    self.copy_and_close_overlay();
                }
                AppEvent::OpenSettings => {
                    self.show_settings(ctx);
                }
                AppEvent::OpenHudEditor => {
                    self.open_hud_editor(ctx);
                }
                AppEvent::HotkeyRecorded { target, binding } => {
                    match target {
                        RecordingTarget::CaptureHotkey => {
                            self.form.cfg.hotkey = binding;
                        }
                        RecordingTarget::DismissHotkey => {
                            self.form.cfg.dismiss_hotkey = binding;
                        }
                    }
                    self.form.recording = None;
                    self.form
                        .set_status("Hotkey recorded. Click Save to apply.", false);
                }
                AppEvent::Quit => {
                    self.auto_save_customization();
                    self.quitting.store(true, Ordering::SeqCst);
                    self.close_overlay();
                    self.hud_editor = None;
                    self.hooks.shutdown();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
    }
}

impl eframe::App for ScreenAskApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_events(ctx);

        if self.hide_after_init {
            self.init_frames = self.init_frames.saturating_add(1);
            if self.init_frames >= 2 {
                self.hide_after_init = false;
                self.hide_settings(ctx);
            } else {
                ctx.request_repaint();
            }
        }

        if ctx.input(|i| i.viewport().close_requested()) && !self.quitting.load(Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.auto_save_customization();
            self.hide_settings(ctx);
        }

        if let Some(at) = self.pending_test_capture {
            let now = Instant::now();
            if now >= at {
                self.pending_test_capture = None;
                self.trigger_capture(ctx);
            } else {
                ctx.request_repaint_after(at - now);
            }
        }

        if let Some(ov) = &self.overlay {
            if let Some(hide_at) = ov.hide_at {
                let now = Instant::now();
                if now >= hide_at {
                    self.close_overlay();
                } else {
                    ctx.request_repaint_after(hide_at - now);
                }
            }
        }

        // Smooth top-left-anchored window resize (paused while HUD Editor parks root window off-screen).
        if self.hud_editor.is_none() {
            self.update_smooth_window_resize(ctx);
        }

        match self.form.show(ctx) {
            SettingsAction::None => {}
            SettingsAction::Save => {
                self.save_form_config(ctx);
            }
            SettingsAction::SaveAndHide => {
                if self.save_form_config(ctx) {
                    self.hide_settings(ctx);
                }
            }
            SettingsAction::TestCapture => {
                if self.save_form_config(ctx) {
                    self.hide_settings(ctx);
                    self.pending_test_capture = Some(Instant::now() + Duration::from_millis(300));
                    ctx.request_repaint_after(Duration::from_millis(300));
                }
            }
            SettingsAction::StartRecording(target) => {
                self.form.recording = Some(target);
                self.form
                    .set_status("Listening... press any key combo or mouse button.", false);
                self.hooks.start_recording(target);
            }
            SettingsAction::CancelRecording => {
                self.form.recording = None;
                self.hooks.cancel_recording();
                self.form.set_status("Recording cancelled.", false);
            }
            SettingsAction::OpenFolder => {
                open_folder_in_explorer(&PathBuf::from(&self.form.cfg.folder));
            }
            SettingsAction::OpenHudEditor => {
                self.auto_save_customization();
                self.open_hud_editor(ctx);
            }
            SettingsAction::ThemeChanged => {
                apply_blur_theme(ctx, self.form.cfg.light_mode, self.form.cfg.accent_color);
                self.auto_save_customization();
            }
            SettingsAction::FontChanged => {
                apply_font(ctx, &self.form.cfg.font_name, &self.form.available_fonts);
                self.auto_save_customization();
            }
            SettingsAction::HiddenModeChanged => {
                if let Some(tray) = &self.tray {
                    tray.set_hidden_mode(self.form.cfg.hidden_mode);
                }
                self.auto_save_customization();
            }
            SettingsAction::Minimize => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
            SettingsAction::CloseToTray => {
                self.auto_save_customization();
                self.hide_settings(ctx);
            }
        }

        if let Some(editor) = &mut self.hud_editor {
            match render_hud_editor_viewport(ctx, editor) {
                HudEditorOutcome::Ongoing => {}
                HudEditorOutcome::Save(updated_cfg) => {
                    self.form.cfg = updated_cfg;
                    self.auto_save_customization();
                    self.form.set_status("HUD layout saved.", false);
                    self.close_hud_editor(ctx);
                }
                HudEditorOutcome::Cancel => {
                    self.close_hud_editor(ctx);
                }
            }
        }

        if let Some(overlay) = &mut self.overlay {
            render_overlay_viewport(ctx, overlay, &self.cfg);
        }
    }
}

fn extract_win32_hwnd(cc: &eframe::CreationContext<'_>) -> isize {
    #[cfg(windows)]
    if let Ok(handle) = cc.window_handle() {
        if let RawWindowHandle::Win32(win32) = handle.as_raw() {
            return win32.hwnd.get();
        }
    }
    0
}
