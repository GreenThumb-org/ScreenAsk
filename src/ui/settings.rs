use crate::autostart::get_autostart;
use crate::config::{ApiProvider, Config, DismissMode};
use crate::events::RecordingTarget;
use crate::fonts::FontOption;
use crate::pdf::{copy_pdf_into_folder, list_pdf_names};
use crate::ui::color_picker::AccentColorPicker;
use crate::ui::icons::{paint_icon, TitleIcon};
use crate::ui::theme::{
    accent_from_rgb, autostart_button, blur_toggle, card_frame, contrast_text_on, moon_sun_toggle,
};
use egui::{
    Align2, Button, Color32, ComboBox, CornerRadius, FontId, Frame, Margin, Pos2, Rect, RichText,
    Sense, Stroke, TextEdit, TopBottomPanel, Ui, Vec2,
};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    Capture,
    Overlay,
    Style,
    Api,
}

impl SettingsTab {
    pub fn label(self) -> &'static str {
        match self {
            Self::Capture => "Capture & PDFs",
            Self::Overlay => "Overlay",
            Self::Style => "Style",
            Self::Api => "AI Provider & API",
        }
    }

    pub fn icon(self) -> TitleIcon {
        match self {
            Self::Capture => TitleIcon::Capture,
            Self::Overlay => TitleIcon::Overlay,
            Self::Style => TitleIcon::Style,
            Self::Api => TitleIcon::Api,
        }
    }

    pub const ALL: [Self; 4] = [Self::Capture, Self::Overlay, Self::Style, Self::Api];
}

pub enum SettingsAction {
    None,
    Save,
    SaveAndHide,
    TestCapture,
    StartRecording(RecordingTarget),
    CancelRecording,
    OpenFolder,
    OpenHudEditor,
    ThemeChanged,
    FontChanged,
    HiddenModeChanged,
    Minimize,
    CloseToTray,
}

pub struct SettingsForm {
    pub cfg: Config,
    pub active_tab: SettingsTab,
    pub autostart: bool,
    pub show_api_key: bool,
    pub recording: Option<RecordingTarget>,
    pub color_picker: AccentColorPicker,
    pub available_fonts: Vec<FontOption>,
    pub font_search: String,
    pub folder_pdfs: Vec<String>,
    pub pdf_picker_open: bool,
    pub measured_content_h: f32,
    pub status_error: Option<String>,
}

impl SettingsForm {
    pub fn new(cfg: Config, available_fonts: Vec<FontOption>) -> Self {
        let autostart = get_autostart();
        let active_tab = if cfg.effective_api_key().is_none() && !cfg.is_local_openai_endpoint() {
            SettingsTab::Api
        } else {
            SettingsTab::Capture
        };
        let color_picker = AccentColorPicker::new(cfg.accent_color);
        let folder_pdfs = list_pdf_names(&PathBuf::from(&cfg.folder));
        Self {
            cfg,
            active_tab,
            autostart,
            show_api_key: false,
            recording: None,
            color_picker,
            available_fonts,
            font_search: String::new(),
            folder_pdfs,
            pdf_picker_open: false,
            measured_content_h: 255.0,
            status_error: None,
        }
    }

    pub fn target_window_size(&self) -> (f32, f32) {
        let w = match self.active_tab {
            SettingsTab::Capture => 550.0,
            SettingsTab::Overlay => 565.0,
            SettingsTab::Style => 555.0,
            SettingsTab::Api => 515.0,
        };
        // Uniform 14px gap on top, bottom, left, and right around the card:
        // titlebar (42) + top gap (14) + exact card height + bottom gap (14) + footer (46)
        let h = (42.0 + 14.0 + self.measured_content_h + 14.0 + 46.0).clamp(220.0, 720.0);
        (w, h)
    }

    pub fn refresh_pdf_count(&mut self) {
        self.folder_pdfs = list_pdf_names(&PathBuf::from(&self.cfg.folder));
    }

    pub fn is_pdf_selected(&self, name: &str) -> bool {
        match &self.cfg.selected_pdfs {
            None => true,
            Some(list) => list.iter().any(|s| s.eq_ignore_ascii_case(name)),
        }
    }

    pub fn set_pdf_selected(&mut self, name: &str, selected: bool) {
        let mut list = match &self.cfg.selected_pdfs {
            Some(existing) => existing.clone(),
            None => self.folder_pdfs.clone(),
        };
        if selected {
            if !list.iter().any(|s| s.eq_ignore_ascii_case(name)) {
                list.push(name.to_string());
            }
        } else {
            list.retain(|s| !s.eq_ignore_ascii_case(name));
        }
        self.cfg.selected_pdfs = Some(list);
    }

    pub fn add_dropped_pdfs(&mut self, paths: &[PathBuf]) -> bool {
        let folder = PathBuf::from(&self.cfg.folder);
        let mut added_any = false;
        let mut last_err = None;

        for p in paths {
            match copy_pdf_into_folder(p, &folder) {
                Ok(filename) => {
                    added_any = true;
                    if let Some(list) = &mut self.cfg.selected_pdfs {
                        if !list.iter().any(|s| s.eq_ignore_ascii_case(&filename)) {
                            list.push(filename);
                        }
                    }
                }
                Err(e) => {
                    last_err = Some(e);
                }
            }
        }

        if added_any {
            self.refresh_pdf_count();
            self.status_error = None;
        } else if let Some(err) = last_err {
            self.status_error = Some(err);
        }
        added_any
    }

    pub fn set_status(&mut self, msg: impl Into<String>, is_error: bool) {
        if is_error {
            self.status_error = Some(msg.into());
        } else {
            self.status_error = None;
        }
    }

    /// Renders the custom Blur-style borderless window with single-line icon title bar,
    /// accent separator line, dedicated tab content, and pinned footer.
    pub fn show(&mut self, ctx: &egui::Context) -> SettingsAction {
        let mut action = SettingsAction::None;
        let light = self.cfg.light_mode;
        let accent = accent_from_rgb(self.cfg.accent_color);
        let on_accent = contrast_text_on(accent);
        let panel_bg = ctx.style().visuals.panel_fill;
        let title_bg = if light {
            Color32::from_rgb(240, 236, 242)
        } else {
            Color32::from_rgb(22, 22, 24)
        };

        // 1. Custom Blur-Style Title Bar (4 icon tabs | centered title | minimize & close | accent divider)
        TopBottomPanel::top("screenask_titlebar")
            .exact_height(42.0)
            .frame(Frame::NONE.fill(title_bg))
            .show_separator_line(false)
            .show(ctx, |ui| {
                let bar_rect = ui.max_rect();

                // Background drag area (allocated first so buttons sit on top)
                let drag_resp = ui.interact(
                    bar_rect,
                    ui.id().with("titlebar_drag"),
                    Sense::click_and_drag(),
                );
                if drag_resp.drag_started_by(egui::PointerButton::Primary) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }

                // Centered "ScreenAsk" title
                let title_col = if light {
                    Color32::from_rgb(20, 20, 24)
                } else {
                    Color32::from_rgba_unmultiplied(251, 254, 254, 235)
                };
                ui.painter().text(
                    bar_rect.center(),
                    Align2::CENTER_CENTER,
                    "ScreenAsk",
                    FontId::proportional(14.0),
                    title_col,
                );

                // Left icon tabs + Right window buttons (no maximize, no pin)
                let inner = bar_rect.shrink2(Vec2::new(8.0, 5.0));
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(inner), |ui| {
                    ui.horizontal_centered(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        for tab in SettingsTab::ALL {
                            let selected = self.active_tab == tab;
                            let btn_size = Vec2::splat(30.0);
                            let (rect, resp) = ui.allocate_exact_size(btn_size, Sense::click());

                            let bg = if selected {
                                Color32::from_rgba_unmultiplied(
                                    accent.r(),
                                    accent.g(),
                                    accent.b(),
                                    if light { 55 } else { 48 },
                                )
                            } else if resp.hovered() {
                                if light {
                                    Color32::from_black_alpha(18)
                                } else {
                                    Color32::from_white_alpha(16)
                                }
                            } else {
                                Color32::TRANSPARENT
                            };

                            if bg != Color32::TRANSPARENT {
                                ui.painter().rect_filled(rect, CornerRadius::same(7), bg);
                            }

                            let icon_col = if selected {
                                accent
                            } else if resp.hovered() {
                                title_col
                            } else if light {
                                Color32::from_black_alpha(150)
                            } else {
                                Color32::from_rgba_unmultiplied(251, 254, 254, 155)
                            };

                            paint_icon(ui, rect.shrink(6.5), tab.icon(), icon_col);

                            if resp.clicked() && self.active_tab != tab {
                                self.active_tab = tab;
                                self.color_picker.open = false;
                            }
                            resp.on_hover_text(tab.label());
                        }

                        // Right side: Minimize & Close only
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;

                            // Close (hide to tray)
                            let (c_rect, c_resp) =
                                ui.allocate_exact_size(Vec2::splat(30.0), Sense::click());
                            if c_resp.hovered() {
                                ui.painter().rect_filled(
                                    c_rect,
                                    CornerRadius::same(7),
                                    Color32::from_rgba_unmultiplied(235, 78, 78, 45),
                                );
                            }
                            let c_col = if c_resp.hovered() {
                                Color32::from_rgb(255, 117, 107)
                            } else if light {
                                Color32::from_black_alpha(180)
                            } else {
                                Color32::from_rgba_unmultiplied(251, 254, 254, 180)
                            };
                            paint_icon(ui, c_rect.shrink(8.0), TitleIcon::Close, c_col);
                            if c_resp.clicked() {
                                action = SettingsAction::CloseToTray;
                            }
                            c_resp.on_hover_text("Hide to tray");

                            // Minimize
                            let (m_rect, m_resp) =
                                ui.allocate_exact_size(Vec2::splat(30.0), Sense::click());
                            if m_resp.hovered() {
                                ui.painter().rect_filled(
                                    m_rect,
                                    CornerRadius::same(7),
                                    if light {
                                        Color32::from_black_alpha(18)
                                    } else {
                                        Color32::from_white_alpha(16)
                                    },
                                );
                            }
                            let m_col = if m_resp.hovered() {
                                title_col
                            } else if light {
                                Color32::from_black_alpha(180)
                            } else {
                                Color32::from_rgba_unmultiplied(251, 254, 254, 180)
                            };
                            paint_icon(ui, m_rect.shrink(8.0), TitleIcon::Minimize, m_col);
                            if m_resp.clicked() {
                                action = SettingsAction::Minimize;
                            }
                            m_resp.on_hover_text("Minimize");
                        });
                    });
                });

                // Full-width Blur-style accent separator line at bottom of title bar
                ui.painter().line_segment(
                    [
                        Pos2::new(bar_rect.left(), bar_rect.bottom() - 0.75),
                        Pos2::new(bar_rect.right(), bar_rect.bottom() - 0.75),
                    ],
                    Stroke::new(1.5, accent),
                );
            });

        // 2. Bottom Panel: Dark gray footer matching title bar with "v1.0 by BB" & action buttons
        TopBottomPanel::bottom("screenask_footer_panel")
            .exact_height(46.0)
            .frame(
                Frame::NONE
                    .fill(title_bg)
                    .inner_margin(Margin {
                        left: 14,
                        right: 14,
                        top: 8,
                        bottom: 8,
                    }),
            )
            .show_separator_line(true)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    if let Some(err) = &self.status_error {
                        ui.label(
                            RichText::new(err)
                                .size(11.5)
                                .color(Color32::from_rgb(255, 117, 107)),
                        );
                    } else {
                        ui.label(RichText::new("v1.1 by BB").size(11.5).weak());
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let primary_btn = Button::new(
                            RichText::new("Save & Hide").color(on_accent).strong(),
                        )
                        .fill(accent)
                        .stroke(Stroke::NONE);
                        if ui.add(primary_btn).clicked() {
                            action = SettingsAction::SaveAndHide;
                        }
                        if ui.button("Save").clicked() {
                            action = SettingsAction::Save;
                        }
                        if ui.button("Test Capture").clicked() {
                            action = SettingsAction::TestCapture;
                        }
                    });
                });
            });

        // 3. Central Panel: Active Tab Content (measures exact card height for uniform 14px margin on all 4 sides)
        egui::CentralPanel::default()
            .frame(
                Frame::NONE
                    .fill(panel_bg)
                    .inner_margin(Margin::same(14)),
            )
            .show(ctx, |ui| {
                ui.set_width(ui.available_width());
                let card_h = match self.active_tab {
                    SettingsTab::Capture => {
                        self.capture_tab_ui(ui, light, accent, &mut action)
                    }
                    SettingsTab::Overlay => {
                        self.overlay_tab_ui(ui, light, accent, &mut action)
                    }
                    SettingsTab::Style => {
                        self.style_tab_ui(ui, light, accent, &mut action)
                    }
                    SettingsTab::Api => self.api_tab_ui(ui, light),
                };
                if (card_h - self.measured_content_h).abs() > 0.5 {
                    self.measured_content_h = card_h;
                    ctx.request_repaint();
                }
            });

        // Subtle crisp outer border around the borderless window
        let win_rect = ctx.screen_rect();
        let outer_stroke = if light {
            Stroke::new(1.0, Color32::from_rgb(160, 160, 168))
        } else {
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(251, 254, 254, 34))
        };
        ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("window_outer_border"),
        ))
        .rect_stroke(
            Rect::from_min_max(win_rect.min, win_rect.max),
            CornerRadius::ZERO,
            outer_stroke,
            egui::StrokeKind::Inside,
        );

        // Handle OS drag-and-drop of PDF files anywhere onto the ScreenAsk window
        let dropped_paths: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        if !dropped_paths.is_empty() && self.add_dropped_pdfs(&dropped_paths) {
            action = SettingsAction::Save;
        }

        // Visual drag-over drop banner when dragging files over the window
        if ctx.input(|i| !i.raw.hovered_files.is_empty()) {
            let drop_painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("pdf_drop_overlay"),
            ));
            let r = ctx.screen_rect().shrink(8.0);
            drop_painter.rect(
                r,
                CornerRadius::same(12),
                Color32::from_rgba_unmultiplied(12, 12, 14, 215),
                Stroke::new(2.0, accent),
                egui::StrokeKind::Inside,
            );
            drop_painter.text(
                r.center(),
                Align2::CENTER_CENTER,
                "Drop PDF file(s) to copy into ScreenAsk folder",
                FontId::proportional(14.0),
                Color32::WHITE,
            );
        }

        // Floating Blur-style HSV accent color popover
        if self
            .color_picker
            .popover_ui(ctx, &mut self.cfg.accent_color, light)
        {
            action = SettingsAction::ThemeChanged;
        }

        action
    }

    fn capture_tab_ui(
        &mut self,
        ui: &mut Ui,
        light: bool,
        accent: Color32,
        action: &mut SettingsAction,
    ) -> f32 {
        card_frame(light)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                // Row 1: Capture Hotkey
                setting_row(
                    ui,
                    "Capture Hotkey",
                    "Key combo or mouse button to trigger capture.",
                    |ui| {
                        let is_rec = self.recording == Some(RecordingTarget::CaptureHotkey);
                        let btn_text = if is_rec { "Listening..." } else { "Record" };
                        if ui.button(btn_text).clicked() {
                            if is_rec {
                                *action = SettingsAction::CancelRecording;
                            } else {
                                *action =
                                    SettingsAction::StartRecording(RecordingTarget::CaptureHotkey);
                            }
                        }
                        let mut hk_display = self.cfg.hotkey.clone();
                        ui.add_enabled(
                            false,
                            TextEdit::singleline(&mut hk_display).desired_width(120.0),
                        );
                    },
                );

                row_divider(ui);

                // Row 2: Save Screenshots (explicitly shows Enabled / Disabled state)
                let save_subtitle = if self.cfg.save_screenshots {
                    "Enabled — PNGs are saved to your folder."
                } else {
                    "Disabled — sent to AI without saving PNGs."
                };
                setting_row(ui, "Save Screenshots", save_subtitle, |ui| {
                    if blur_toggle(ui, &mut self.cfg.save_screenshots, accent, light).changed() {
                        *action = SettingsAction::Save;
                    }
                    ui.add_space(4.0);
                    let state_text = if self.cfg.save_screenshots {
                        RichText::new("On").size(12.0).strong().color(accent)
                    } else {
                        RichText::new("Off").size(12.0).weak()
                    };
                    ui.label(state_text);
                });

                row_divider(ui);

                // Row 3: Screenshots & PDFs Folder (Browse + Open on top row, full-width path underneath)
                setting_row(
                    ui,
                    "Screenshots & PDFs Folder",
                    "Directory for saved PNGs and reference PDFs.",
                    |ui| {
                        if ui.button("Open").clicked() {
                            *action = SettingsAction::OpenFolder;
                        }
                        if ui.button("Browse").clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                self.cfg.folder = path.to_string_lossy().into_owned();
                                self.refresh_pdf_count();
                                *action = SettingsAction::Save;
                            }
                        }
                    },
                );
                ui.add_space(4.0);
                let path_resp = ui.add(
                    TextEdit::singleline(&mut self.cfg.folder)
                        .desired_width(ui.available_width()),
                );
                if path_resp.lost_focus() {
                    self.refresh_pdf_count();
                }

                row_divider(ui);

                // Row 4: Reference PDFs (button to select which PDFs are sent + Add PDF + drag-and-drop)
                let total_pdfs = self.folder_pdfs.len();
                let selected_count = self
                    .folder_pdfs
                    .iter()
                    .filter(|n| self.is_pdf_selected(n))
                    .count();
                let pdf_subtitle = if total_pdfs == 0 {
                    "No PDFs in folder · Drag & drop PDFs here.".to_string()
                } else {
                    format!(
                        "{selected_count} of {total_pdfs} selected · Drag & drop PDFs here."
                    )
                };

                setting_row(ui, "Reference PDFs", &pdf_subtitle, |ui| {
                    let sel_label = if self.pdf_picker_open {
                        "Hide PDFs"
                    } else {
                        "Select PDFs"
                    };
                    let sel_btn = Button::new(RichText::new(sel_label).size(12.0)).stroke(
                        if self.pdf_picker_open {
                            Stroke::new(1.0, accent)
                        } else {
                            Stroke::new(1.0, Color32::from_rgba_unmultiplied(251, 254, 254, 42))
                        },
                    );
                    if ui.add(sel_btn).clicked() {
                        self.pdf_picker_open = !self.pdf_picker_open;
                        if self.pdf_picker_open {
                            self.refresh_pdf_count();
                        }
                    }

                    if ui.button("Add PDF").clicked() {
                        if let Some(files) = rfd::FileDialog::new()
                            .add_filter("PDF Documents", &["pdf"])
                            .pick_files()
                        {
                            if self.add_dropped_pdfs(&files) {
                                *action = SettingsAction::Save;
                            }
                        }
                    }
                });

                if self.pdf_picker_open {
                    ui.add_space(6.0);
                    let box_fill = if light {
                        Color32::from_rgb(242, 242, 245)
                    } else {
                        Color32::from_rgb(18, 18, 20)
                    };
                    Frame::NONE
                        .fill(box_fill)
                        .stroke(Stroke::new(
                            1.0,
                            if light {
                                Color32::from_rgb(190, 190, 196)
                            } else {
                                Color32::from_rgba_unmultiplied(251, 254, 254, 28)
                            },
                        ))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::same(8))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                if ui.small_button("Select All").clicked() {
                                    self.cfg.selected_pdfs = Some(self.folder_pdfs.clone());
                                    *action = SettingsAction::Save;
                                }
                                if ui.small_button("Select None").clicked() {
                                    self.cfg.selected_pdfs = Some(Vec::new());
                                    *action = SettingsAction::Save;
                                }
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.small_button("Refresh").clicked() {
                                            self.refresh_pdf_count();
                                        }
                                    },
                                );
                            });
                            ui.add_space(4.0);
                            if self.folder_pdfs.is_empty() {
                                ui.label(
                                    RichText::new(
                                        "No .pdf files found. Drag & drop a PDF onto this window or click Add PDF.",
                                    )
                                    .size(11.0)
                                    .weak(),
                                );
                            } else {
                                let pdfs_snapshot = self.folder_pdfs.clone();
                                egui::ScrollArea::vertical()
                                    .max_height(110.0)
                                    .show(ui, |ui| {
                                        for name in &pdfs_snapshot {
                                            let mut checked = self.is_pdf_selected(name);
                                            if ui.checkbox(&mut checked, name).changed() {
                                                self.set_pdf_selected(name, checked);
                                                *action = SettingsAction::Save;
                                            }
                                        }
                                    });
                            }
                        });
                }

                row_divider(ui);

                // Row 5: Prompt (full-width multiline editor underneath title + description)
                ui.label(RichText::new("Prompt").strong());
                ui.label(
                    RichText::new("Instruction sent to the AI with every screenshot.")
                        .size(11.0)
                        .weak(),
                );
                ui.add_space(4.0);
                ui.add(
                    TextEdit::multiline(&mut self.cfg.prompt)
                        .desired_rows(4)
                        .desired_width(ui.available_width()),
                );
            })
            .response
            .rect
            .height()
    }

    fn overlay_tab_ui(
        &mut self,
        ui: &mut Ui,
        light: bool,
        accent: Color32,
        action: &mut SettingsAction,
    ) -> f32 {
        card_frame(light)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                // Row 1: HUD Layout Editor
                setting_row(
                    ui,
                    "HUD Layout",
                    "Position, resize, and customize box & text style.",
                    |ui| {
                        let hud_btn = Button::new(RichText::new("Edit HUD Layout").size(12.0))
                            .stroke(Stroke::new(1.0, accent));
                        if ui.add(hud_btn).clicked() {
                            *action = SettingsAction::OpenHudEditor;
                        }
                    },
                );

                row_divider(ui);

                // Row 2: Dismiss Behavior
                setting_row(
                    ui,
                    "Dismiss Behavior",
                    "Choose how the overlay closes after appearing.",
                    |ui| {
                        ComboBox::from_id_salt("dismiss_mode_combo")
                            .selected_text(self.cfg.dismiss_mode.label())
                            .truncate()
                            .width(220.0)
                            .show_ui(ui, |ui| {
                                for mode in DismissMode::ALL {
                                    ui.selectable_value(
                                        &mut self.cfg.dismiss_mode,
                                        mode,
                                        mode.label(),
                                    );
                                }
                            });
                    },
                );

                if self.cfg.dismiss_mode == DismissMode::CustomKey {
                    row_divider(ui);
                    setting_row(
                        ui,
                        "Dismiss Key / Mouse",
                        "Custom binding used to close the overlay.",
                        |ui| {
                            let is_rec = self.recording == Some(RecordingTarget::DismissHotkey);
                            let btn_text = if is_rec { "Listening..." } else { "Record" };
                            if ui.button(btn_text).clicked() {
                                if is_rec {
                                    *action = SettingsAction::CancelRecording;
                                } else {
                                    *action = SettingsAction::StartRecording(
                                        RecordingTarget::DismissHotkey,
                                    );
                                }
                            }
                            let mut d_display = self.cfg.dismiss_hotkey.clone();
                            ui.add_enabled(
                                false,
                                TextEdit::singleline(&mut d_display).desired_width(120.0),
                            );
                        },
                    );
                }

                if self.cfg.dismiss_mode != DismissMode::SameHotkey
                    && self.cfg.dismiss_mode != DismissMode::AnyInput
                {
                    row_divider(ui);
                    setting_row(
                        ui,
                        "Same Hotkey Hides",
                        "Press capture hotkey to close without re-capturing.",
                        |ui| {
                            blur_toggle(ui, &mut self.cfg.hotkey_dismisses_overlay, accent, light);
                        },
                    );
                }

                row_divider(ui);

                // Row 3: Auto-Hide Timeout
                setting_row(
                    ui,
                    "Auto-Hide Timeout",
                    "Seconds visible (0 = stay · right-click copies).",
                    |ui| {
                        ui.label(RichText::new("sec").size(11.5).weak());
                        ui.add(
                            egui::DragValue::new(&mut self.cfg.overlay_seconds)
                                .range(0..=300)
                                .speed(1),
                        );
                    },
                );
            })
            .response
            .rect
            .height()
    }

    fn style_tab_ui(
        &mut self,
        ui: &mut Ui,
        light: bool,
        accent: Color32,
        action: &mut SettingsAction,
    ) -> f32 {
        card_frame(light)
            .show(ui, |ui| {
            ui.set_width(ui.available_width());

            if self
                .color_picker
                .row_ui(ui, &mut self.cfg.accent_color, light)
            {
                *action = SettingsAction::ThemeChanged;
            }

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // Searchable App Font selector (truncated so long names never overlap)
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_max_width(240.0);
                    ui.label(RichText::new("App Font").strong());
                    ui.label(
                        RichText::new("Bundled or system-installed font.")
                            .size(11.0)
                            .weak(),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let prev_font = self.cfg.font_name.clone();
                    ComboBox::from_id_salt("app_font_combo")
                        .selected_text(&self.cfg.font_name)
                        .truncate()
                        .width(215.0)
                        .height(240.0)
                        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                        .show_ui(ui, |ui| {
                            ui.add(
                                TextEdit::singleline(&mut self.font_search)
                                    .hint_text("Type to filter fonts...")
                                    .desired_width(198.0),
                            );
                            ui.add_space(4.0);

                            let query = self.font_search.trim().to_lowercase();
                            let mut any_shown = false;
                            let mut separated = false;
                            let mut picked = false;

                            for opt in &self.available_fonts {
                                if !query.is_empty() && !opt.name.to_lowercase().contains(&query) {
                                    continue;
                                }
                                if !opt.is_bundled && !separated && query.is_empty() {
                                    ui.separator();
                                    separated = true;
                                }
                                any_shown = true;
                                if ui
                                    .selectable_value(
                                        &mut self.cfg.font_name,
                                        opt.name.clone(),
                                        &opt.name,
                                    )
                                    .clicked()
                                {
                                    picked = true;
                                }
                            }

                            if !any_shown {
                                ui.label(RichText::new("No matching fonts").weak());
                            }
                            if picked {
                                self.font_search.clear();
                                ui.close_menu();
                            }
                        });
                    if self.cfg.font_name != prev_font {
                        *action = SettingsAction::FontChanged;
                    }
                });
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("Theme").strong());
                    ui.label(
                        RichText::new(if self.cfg.light_mode {
                            "Light mode active"
                        } else {
                            "Dark mode active"
                        })
                        .size(11.0)
                        .weak(),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if moon_sun_toggle(ui, &mut self.cfg.light_mode, accent).changed() {
                        *action = SettingsAction::ThemeChanged;
                    }
                });
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("Start with Windows").strong());
                    ui.label(
                        RichText::new("Launch silently in the tray on login.")
                            .size(11.0)
                            .weak(),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    autostart_button(ui, &mut self.autostart, accent, light);
                });
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("Hidden Mode").strong());
                    ui.label(
                        RichText::new("Discreet display-service tray icon & menu.")
                            .size(11.0)
                            .weak(),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if blur_toggle(ui, &mut self.cfg.hidden_mode, accent, light).changed() {
                        *action = SettingsAction::HiddenModeChanged;
                    }
                });
            });
            })
            .response
            .rect
            .height()
    }

    fn api_tab_ui(&mut self, ui: &mut Ui, light: bool) -> f32 {
        card_frame(light)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                // Row 1: Provider Selector
                let prev_provider = self.cfg.provider;
                setting_row(
                    ui,
                    "AI Provider",
                    "Choose Gemini, Claude, or any OpenAI-compatible API.",
                    |ui| {
                        ComboBox::from_id_salt("api_provider_combo")
                            .selected_text(self.cfg.provider.label())
                            .width(220.0)
                            .show_ui(ui, |ui| {
                                for p in ApiProvider::ALL {
                                    ui.selectable_value(&mut self.cfg.provider, p, p.label());
                                }
                            });
                    },
                );
                if self.cfg.provider != prev_provider {
                    let (def_primary, def_fallback) = self.cfg.provider.default_models();
                    self.cfg.model = def_primary.to_string();
                    self.cfg.fallback_model = def_fallback.to_string();
                }

                // Optional Row: Base URL (for OpenAI / OpenRouter / Groq / Ollama / LM Studio)
                if self.cfg.provider == ApiProvider::OpenAi {
                    row_divider(ui);
                    setting_row(
                        ui,
                        "Base URL",
                        "OpenAI, OpenRouter, Groq, or local Ollama endpoint.",
                        |ui| {
                            ui.add(
                                TextEdit::singleline(&mut self.cfg.api_base_url)
                                    .hint_text("https://api.openai.com/v1")
                                    .desired_width(220.0),
                            );
                        },
                    );
                }

                row_divider(ui);

                // Row: API Key
                let key_desc = format!(
                    "API key for {} (or {}).",
                    self.cfg.provider.label(),
                    self.cfg.provider.env_var_name()
                );
                let key_hint = self.cfg.provider.key_hint();
                setting_row(ui, "API Key", &key_desc, |ui| {
                    let toggle_label = if self.show_api_key { "Hide" } else { "Show" };
                    if ui.button(toggle_label).clicked() {
                        self.show_api_key = !self.show_api_key;
                    }
                    ui.add(
                        TextEdit::singleline(&mut self.cfg.api_key)
                            .password(!self.show_api_key)
                            .hint_text(key_hint)
                            .desired_width(175.0),
                    );
                });

                row_divider(ui);

                // Row: Primary Model
                setting_row(
                    ui,
                    "Primary Model",
                    "Default vision model used for requests.",
                    |ui| {
                        ui.add(TextEdit::singleline(&mut self.cfg.model).desired_width(220.0));
                    },
                );

                row_divider(ui);

                // Row: Fallback Model
                setting_row(
                    ui,
                    "Fallback Model",
                    "Retried automatically on 429 / 503 / 529 / 500.",
                    |ui| {
                        ui.add(
                            TextEdit::singleline(&mut self.cfg.fallback_model)
                                .hint_text("Optional")
                                .desired_width(220.0),
                        );
                    },
                );
            })
            .response
            .rect
            .height()
    }
}

fn setting_row(ui: &mut Ui, title: &str, subtitle: &str, add_control: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_max_width(250.0);
            ui.label(RichText::new(title).strong());
            ui.label(RichText::new(subtitle).size(11.0).weak());
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), add_control);
    });
}

fn row_divider(ui: &mut Ui) {
    ui.add_space(6.0);
    ui.separator();
    ui.add_space(6.0);
}
