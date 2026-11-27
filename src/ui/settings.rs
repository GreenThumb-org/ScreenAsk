use crate::autostart::get_autostart;
use crate::config::{Config, DismissMode};
use crate::events::RecordingTarget;
use crate::fonts::FontOption;
use crate::ui::color_picker::AccentColorPicker;
use crate::ui::icons::{paint_icon, TitleIcon};
use crate::ui::theme::{
    accent_from_rgb, autostart_button, blur_toggle, card_frame, contrast_text_on, moon_sun_toggle,
};
use egui::{
    Align2, Button, Color32, ComboBox, CornerRadius, FontId, Frame, Margin, Pos2, Rect, RichText,
    Sense, Stroke, TextEdit, TopBottomPanel, Ui, Vec2,
};

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
            Self::Api => "Gemini API",
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
    pub measured_content_h: f32,
    pub status_error: Option<String>,
}

impl SettingsForm {
    pub fn new(cfg: Config, available_fonts: Vec<FontOption>) -> Self {
        let autostart = get_autostart();
        let active_tab = if cfg.effective_api_key().is_none() {
            SettingsTab::Api
        } else {
            SettingsTab::Capture
        };
        let color_picker = AccentColorPicker::new(cfg.accent_color);
        Self {
            cfg,
            active_tab,
            autostart,
            show_api_key: false,
            recording: None,
            color_picker,
            available_fonts,
            font_search: String::new(),
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

    pub fn refresh_pdf_count(&mut self) {}

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
                        ui.label(RichText::new("v1.0 by BB").size(11.5).weak());
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
                    "Disabled — sent to Gemini without saving PNGs."
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

                // Row 3: Screenshots & Reference PDFs Folder
                setting_row(
                    ui,
                    "Screenshots & PDFs Folder",
                    "Any .pdf files here are auto-loaded as context with each image.",
                    |ui| {
                        if ui.button("Open").clicked() {
                            *action = SettingsAction::OpenFolder;
                        }
                        if ui.button("Browse").clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                self.cfg.folder = path.to_string_lossy().into_owned();
                            }
                        }
                        ui.add(TextEdit::singleline(&mut self.cfg.folder).desired_width(155.0));
                    },
                );

                row_divider(ui);

                // Row 4: Prompt (full-width multiline editor underneath title + description)
                ui.label(RichText::new("Prompt").strong());
                ui.label(
                    RichText::new("Instruction sent to Gemini with every screenshot.")
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

                setting_row(
                    ui,
                    "API Key",
                    "Google AI Studio key (or GEMINI_API_KEY).",
                    |ui| {
                        let toggle_label = if self.show_api_key { "Hide" } else { "Show" };
                        if ui.button(toggle_label).clicked() {
                            self.show_api_key = !self.show_api_key;
                        }
                        ui.add(
                            TextEdit::singleline(&mut self.cfg.api_key)
                                .password(!self.show_api_key)
                                .hint_text("AIza...")
                                .desired_width(175.0),
                        );
                    },
                );

                row_divider(ui);

                setting_row(
                    ui,
                    "Primary Model",
                    "Default Gemini model used for vision requests.",
                    |ui| {
                        ui.add(TextEdit::singleline(&mut self.cfg.model).desired_width(220.0));
                    },
                );

                row_divider(ui);

                setting_row(
                    ui,
                    "Fallback Model",
                    "Retried automatically on 429 / 503 / 500.",
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
