use crate::config::{Config, HudTextAlign};
use crate::ui::overlay::{pt_to_px, resolve_overlay_style, SCREEN_MARGIN};
use crate::ui::theme::{accent_from_rgb, blur_toggle, contrast_text_on, popover_frame};
use crate::win_util::FocusedMonitorInfo;
use egui::{
    Area, Button, Color32, ColorImage, CornerRadius, CursorIcon, FontId, Frame, Grid, Id, Order,
    Pos2, Rect, RichText, Sense, Slider, Stroke, TextureHandle, TextureOptions, Vec2,
    ViewportBuilder, ViewportId,
};
use image::RgbImage;

const SAMPLE_TEXT: &str = "ScreenAsk HUD Preview\nDrag anywhere inside this box to move it. Drag any of the 4 sides or 4 corners to resize freely to your cursor.";
const MIN_BOX_W: f32 = 160.0;
const MIN_BOX_H: f32 = 48.0;

pub enum HudEditorOutcome {
    Ongoing,
    Save(Config),
    Cancel,
}

pub struct HudEditorState {
    pub monitor: FocusedMonitorInfo,
    pub draft: Config,
    pub box_rect: Rect,
    pub customize_open: bool,
    pub show_real_screen: bool,
    pending_backdrop: Option<RgbImage>,
    backdrop_tex: Option<TextureHandle>,
}

impl HudEditorState {
    pub fn new(
        monitor: FocusedMonitorInfo,
        cfg: &Config,
        backdrop_img: Option<RgbImage>,
    ) -> Self {
        let dpi = monitor.dpi_scale.max(0.5);
        let work_w = (monitor.work.width() as f32 / dpi).max(320.0);
        let work_h = (monitor.work.height() as f32 / dpi).max(240.0);

        let w = cfg.width.clamp(MIN_BOX_W, (work_w - SCREEN_MARGIN * 2.0).max(MIN_BOX_W));
        let h = cfg
            .hud_height
            .unwrap_or(115.0)
            .clamp(MIN_BOX_H, (work_h - SCREEN_MARGIN * 2.0).max(MIN_BOX_H));

        let min_x = SCREEN_MARGIN;
        let max_x = (work_w - w - SCREEN_MARGIN).max(min_x);
        let min_y = SCREEN_MARGIN;
        let max_y = (work_h - h - SCREEN_MARGIN).max(min_y);

        let nx = cfg.hud_x.unwrap_or(0.0).clamp(0.0, 1.0);
        let ny = cfg.hud_y.unwrap_or(1.0).clamp(0.0, 1.0);
        let x = min_x + (max_x - min_x) * nx;
        let y = min_y + (max_y - min_y) * ny;

        Self {
            monitor,
            draft: cfg.clone(),
            box_rect: Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h)),
            customize_open: false,
            show_real_screen: true,
            pending_backdrop: backdrop_img,
            backdrop_tex: None,
        }
    }

    pub fn reset(&mut self, work_w: f32, work_h: f32) {
        self.draft.width = 480.0;
        self.draft.hud_height = None;
        self.draft.hud_x = None;
        self.draft.hud_y = None;
        self.draft.hud_scale = 1.0;
        self.draft.font_size = 10.0;
        self.draft.opacity = 0.88;
        self.draft.hud_box_color = None;
        self.draft.hud_text_color = None;
        self.draft.hud_rounding = 10.0;
        self.draft.hud_padding = 12.0;
        self.draft.hud_show_box = true;
        self.draft.hud_border = true;
        self.draft.hud_text_align = HudTextAlign::Left;

        let w = 480.0_f32.min((work_w - SCREEN_MARGIN * 2.0).max(MIN_BOX_W));
        let h = 115.0;
        let x = SCREEN_MARGIN;
        let y = (work_h - h - SCREEN_MARGIN).max(SCREEN_MARGIN);
        self.box_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h));
    }

    fn build_saved_config(&self, work_w: f32, work_h: f32) -> Config {
        let mut out = self.draft.clone();
        let w = self.box_rect.width().clamp(MIN_BOX_W, 1800.0);
        let h = self.box_rect.height().clamp(MIN_BOX_H, 1200.0);
        let min_x = SCREEN_MARGIN;
        let max_x = (work_w - w - SCREEN_MARGIN).max(min_x);
        let min_y = SCREEN_MARGIN;
        let max_y = (work_h - h - SCREEN_MARGIN).max(min_y);

        let nx = if (max_x - min_x).abs() > 1.0 {
            ((self.box_rect.left() - min_x) / (max_x - min_x)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let ny = if (max_y - min_y).abs() > 1.0 {
            ((self.box_rect.top() - min_y) / (max_y - min_y)).clamp(0.0, 1.0)
        } else {
            1.0
        };

        out.width = w;
        out.hud_height = Some(h);
        out.hud_x = Some(nx);
        out.hud_y = Some(ny);
        out.hud_scale = 1.0;
        out.sanitize();
        out
    }
}

pub fn render_hud_editor_viewport(
    ctx: &egui::Context,
    state: &mut HudEditorState,
) -> HudEditorOutcome {
    let dpi = state.monitor.dpi_scale.max(0.5);
    let work_left = state.monitor.work.left as f32 / dpi;
    let work_top = state.monitor.work.top as f32 / dpi;
    let work_w = (state.monitor.work.width() as f32 / dpi).max(320.0);
    let work_h = (state.monitor.work.height() as f32 / dpi).max(240.0);

    let accent = accent_from_rgb(state.draft.accent_color);
    let on_accent = contrast_text_on(accent);
    let mut outcome = HudEditorOutcome::Ongoing;

    let viewport_id = ViewportId::from_hash_of("screenask_hud_editor_viewport");
    let builder = ViewportBuilder::default()
        .with_title("ScreenAsk HUD Layout Editor")
        .with_decorations(false)
        .with_transparent(false)
        .with_always_on_top()
        .with_taskbar(false)
        .with_resizable(false)
        .with_inner_size([work_w, work_h])
        .with_position([work_left, work_top]);

    ctx.show_viewport_immediate(viewport_id, builder, |ed_ctx, _class| {
        // Upload the real-screen capture texture once on first frame
        if let Some(rgb_img) = state.pending_backdrop.take() {
            let size = [rgb_img.width() as usize, rgb_img.height() as usize];
            let color_img = ColorImage::from_rgb(size, rgb_img.as_raw());
            state.backdrop_tex = Some(ed_ctx.load_texture(
                "hud_real_screen_backdrop",
                color_img,
                TextureOptions::LINEAR,
            ));
        }

        if ed_ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            outcome = HudEditorOutcome::Cancel;
        } else if ed_ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
            outcome = HudEditorOutcome::Save(state.build_saved_config(work_w, work_h));
        }

        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(Color32::from_rgb(14, 14, 16)))
            .show(ed_ctx, |ui| {
                let full_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(work_w, work_h));

                // 1. Real Screen Backdrop (or clean dark background if toggled off)
                if state.show_real_screen {
                    if let Some(tex) = &state.backdrop_tex {
                        ui.painter().image(
                            tex.id(),
                            full_rect,
                            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                }

                let (bg_color, fg_color, border_stroke) = resolve_overlay_style(&state.draft);
                let pad = state.draft.hud_padding.clamp(4.0, 32.0);
                let corner_r = state.draft.hud_rounding.clamp(0.0, 28.0).round() as u8;
                let font_px = pt_to_px(state.draft.font_size);

                // Keep box_rect inside work area bounds
                let max_right = work_w - SCREEN_MARGIN;
                let max_bottom = work_h - SCREEN_MARGIN;
                state.box_rect.min.x = state.box_rect.min.x.clamp(SCREEN_MARGIN, max_right - MIN_BOX_W);
                state.box_rect.min.y = state.box_rect.min.y.clamp(SCREEN_MARGIN, max_bottom - MIN_BOX_H);
                state.box_rect.max.x = state.box_rect.max.x.clamp(state.box_rect.min.x + MIN_BOX_W, max_right);
                state.box_rect.max.y = state.box_rect.max.y.clamp(state.box_rect.min.y + MIN_BOX_H, max_bottom);
                let box_rect = state.box_rect;

                // 2. Paint preview overlay card + aligned text + accent selection outline
                ui.painter().rect(
                    box_rect,
                    CornerRadius::same(corner_r),
                    bg_color,
                    border_stroke,
                    egui::StrokeKind::Inside,
                );

                let wrap_w = (box_rect.width() - pad * 2.0).max(40.0);
                let galley = ed_ctx.fonts(|f| {
                    f.layout(
                        SAMPLE_TEXT.to_string(),
                        FontId::proportional(font_px),
                        fg_color,
                        wrap_w,
                    )
                });
                let text_x = match state.draft.hud_text_align {
                    HudTextAlign::Left => box_rect.left() + pad,
                    HudTextAlign::Center => {
                        box_rect.left() + pad + ((wrap_w - galley.size().x).max(0.0) * 0.5)
                    }
                    HudTextAlign::Right => {
                        box_rect.left() + pad + (wrap_w - galley.size().x).max(0.0)
                    }
                };
                // Clip text cleanly inside the box
                let clip_painter = ui.painter().with_clip_rect(box_rect.shrink(pad * 0.5));
                clip_painter.galley(Pos2::new(text_x, box_rect.top() + pad), galley, fg_color);

                // Active bounding outline
                ui.painter().rect_stroke(
                    box_rect.expand(1.5),
                    CornerRadius::same(corner_r.saturating_add(1)),
                    Stroke::new(1.6, accent),
                    egui::StrokeKind::Outside,
                );

                // 4. Interactive 4 CORNERS (follow cursor freely in X & Y, opposite corner stays fixed!)
                let mut resized = false;
                let corner_defs = [
                    ("c_tl", box_rect.left_top(), CursorIcon::ResizeNwSe, true, true),
                    ("c_tr", box_rect.right_top(), CursorIcon::ResizeNeSw, false, true),
                    ("c_bl", box_rect.left_bottom(), CursorIcon::ResizeNeSw, true, false),
                    ("c_br", box_rect.right_bottom(), CursorIcon::ResizeNwSe, false, false),
                ];

                for (id_str, pos, cursor, is_left, is_top) in corner_defs {
                    let hit = Rect::from_center_size(pos, Vec2::splat(20.0));
                    let vis = Rect::from_center_size(pos, Vec2::splat(10.0));
                    let resp = ui.interact(hit, ui.id().with(id_str), Sense::drag());

                    if resp.hovered() || resp.dragged() {
                        ed_ctx.set_cursor_icon(cursor);
                    }
                    if resp.dragged() {
                        resized = true;
                        if let Some(p) = ed_ctx.input(|i| i.pointer.interact_pos()) {
                            if is_left {
                                state.box_rect.min.x =
                                    p.x.clamp(SCREEN_MARGIN, state.box_rect.max.x - MIN_BOX_W);
                            } else {
                                state.box_rect.max.x =
                                    p.x.clamp(state.box_rect.min.x + MIN_BOX_W, max_right);
                            }
                            if is_top {
                                state.box_rect.min.y =
                                    p.y.clamp(SCREEN_MARGIN, state.box_rect.max.y - MIN_BOX_H);
                            } else {
                                state.box_rect.max.y =
                                    p.y.clamp(state.box_rect.min.y + MIN_BOX_H, max_bottom);
                            }
                        }
                        ed_ctx.request_repaint();
                    }

                    ui.painter().rect(
                        vis,
                        CornerRadius::same(3),
                        accent,
                        Stroke::new(1.5, Color32::WHITE),
                        egui::StrokeKind::Outside,
                    );
                }

                // 5. Interactive 4 EDGES (Top, Bottom, Left, Right — follow cursor directly!)
                let edge_thickness = 14.0;
                let h_pill_len = (box_rect.width() * 0.25).clamp(24.0, 54.0);
                let v_pill_len = (box_rect.height() * 0.35).clamp(20.0, 48.0);

                // Left edge
                let left_hit = Rect::from_center_size(
                    Pos2::new(box_rect.left(), box_rect.center().y),
                    Vec2::new(edge_thickness, (box_rect.height() - 24.0).max(20.0)),
                );
                let left_resp = ui.interact(left_hit, ui.id().with("e_left"), Sense::drag());
                if left_resp.hovered() || left_resp.dragged() {
                    ed_ctx.set_cursor_icon(CursorIcon::ResizeHorizontal);
                }
                if left_resp.dragged() {
                    resized = true;
                    if let Some(p) = ed_ctx.input(|i| i.pointer.interact_pos()) {
                        state.box_rect.min.x =
                            p.x.clamp(SCREEN_MARGIN, state.box_rect.max.x - MIN_BOX_W);
                    }
                    ed_ctx.request_repaint();
                }
                ui.painter().rect(
                    Rect::from_center_size(
                        Pos2::new(box_rect.left(), box_rect.center().y),
                        Vec2::new(5.0, v_pill_len),
                    ),
                    CornerRadius::same(3),
                    accent,
                    Stroke::new(1.0, Color32::WHITE),
                    egui::StrokeKind::Outside,
                );

                // Right edge
                let right_hit = Rect::from_center_size(
                    Pos2::new(box_rect.right(), box_rect.center().y),
                    Vec2::new(edge_thickness, (box_rect.height() - 24.0).max(20.0)),
                );
                let right_resp = ui.interact(right_hit, ui.id().with("e_right"), Sense::drag());
                if right_resp.hovered() || right_resp.dragged() {
                    ed_ctx.set_cursor_icon(CursorIcon::ResizeHorizontal);
                }
                if right_resp.dragged() {
                    resized = true;
                    if let Some(p) = ed_ctx.input(|i| i.pointer.interact_pos()) {
                        state.box_rect.max.x =
                            p.x.clamp(state.box_rect.min.x + MIN_BOX_W, max_right);
                    }
                    ed_ctx.request_repaint();
                }
                ui.painter().rect(
                    Rect::from_center_size(
                        Pos2::new(box_rect.right(), box_rect.center().y),
                        Vec2::new(5.0, v_pill_len),
                    ),
                    CornerRadius::same(3),
                    accent,
                    Stroke::new(1.0, Color32::WHITE),
                    egui::StrokeKind::Outside,
                );

                // Top edge
                let top_hit = Rect::from_center_size(
                    Pos2::new(box_rect.center().x, box_rect.top()),
                    Vec2::new((box_rect.width() - 24.0).max(20.0), edge_thickness),
                );
                let top_resp = ui.interact(top_hit, ui.id().with("e_top"), Sense::drag());
                if top_resp.hovered() || top_resp.dragged() {
                    ed_ctx.set_cursor_icon(CursorIcon::ResizeVertical);
                }
                if top_resp.dragged() {
                    resized = true;
                    if let Some(p) = ed_ctx.input(|i| i.pointer.interact_pos()) {
                        state.box_rect.min.y =
                            p.y.clamp(SCREEN_MARGIN, state.box_rect.max.y - MIN_BOX_H);
                    }
                    ed_ctx.request_repaint();
                }
                ui.painter().rect(
                    Rect::from_center_size(
                        Pos2::new(box_rect.center().x, box_rect.top()),
                        Vec2::new(h_pill_len, 5.0),
                    ),
                    CornerRadius::same(3),
                    accent,
                    Stroke::new(1.0, Color32::WHITE),
                    egui::StrokeKind::Outside,
                );

                // Bottom edge
                let bot_hit = Rect::from_center_size(
                    Pos2::new(box_rect.center().x, box_rect.bottom()),
                    Vec2::new((box_rect.width() - 24.0).max(20.0), edge_thickness),
                );
                let bot_resp = ui.interact(bot_hit, ui.id().with("e_bottom"), Sense::drag());
                if bot_resp.hovered() || bot_resp.dragged() {
                    ed_ctx.set_cursor_icon(CursorIcon::ResizeVertical);
                }
                if bot_resp.dragged() {
                    resized = true;
                    if let Some(p) = ed_ctx.input(|i| i.pointer.interact_pos()) {
                        state.box_rect.max.y =
                            p.y.clamp(state.box_rect.min.y + MIN_BOX_H, max_bottom);
                    }
                    ed_ctx.request_repaint();
                }
                ui.painter().rect(
                    Rect::from_center_size(
                        Pos2::new(box_rect.center().x, box_rect.bottom()),
                        Vec2::new(h_pill_len, 5.0),
                    ),
                    CornerRadius::same(3),
                    accent,
                    Stroke::new(1.0, Color32::WHITE),
                    egui::StrokeKind::Outside,
                );

                // 6. Drag inside box body to move freely
                let body_resp = ui.interact(box_rect, ui.id().with("hud_box_move"), Sense::drag());
                if !resized {
                    if body_resp.dragged() {
                        ed_ctx.set_cursor_icon(CursorIcon::Grabbing);
                        let delta = body_resp.drag_delta();
                        let bw = state.box_rect.width();
                        let bh = state.box_rect.height();
                        let nx = (state.box_rect.left() + delta.x)
                            .clamp(SCREEN_MARGIN, (max_right - bw).max(SCREEN_MARGIN));
                        let ny = (state.box_rect.top() + delta.y)
                            .clamp(SCREEN_MARGIN, (max_bottom - bh).max(SCREEN_MARGIN));

                        state.box_rect = Rect::from_min_size(Pos2::new(nx, ny), Vec2::new(bw, bh));
                        ed_ctx.request_repaint();
                    } else if body_resp.hovered() {
                        ed_ctx.set_cursor_icon(CursorIcon::Grab);
                    }
                }

                // 7. Top-center floating toolbar + expandable Customize drawer
                let toolbar_w = 540.0_f32.min(work_w - 32.0);
                Area::new(Id::new("hud_editor_toolbar"))
                    .order(Order::Foreground)
                    .fixed_pos(Pos2::new((work_w - toolbar_w) * 0.5, 16.0))
                    .show(ed_ctx, |ui| {
                        popover_frame(false).show(ui, |ui| {
                            ui.set_width(toolbar_w - 20.0);

                            // Top Row: Title + Action buttons
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("HUD Layout Editor")
                                        .size(13.5)
                                        .strong()
                                        .color(Color32::WHITE),
                                );

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let save_btn = Button::new(
                                            RichText::new("Save & Done")
                                                .color(on_accent)
                                                .strong(),
                                        )
                                        .fill(accent)
                                        .stroke(Stroke::NONE);
                                        if ui.add(save_btn).clicked() {
                                            outcome = HudEditorOutcome::Save(
                                                state.build_saved_config(work_w, work_h),
                                            );
                                        }
                                        if ui.button("Cancel").clicked() {
                                            outcome = HudEditorOutcome::Cancel;
                                        }
                                        if ui.button("Reset").clicked() {
                                            state.reset(work_w, work_h);
                                        }
                                    },
                                );
                            });

                            ui.add_space(6.0);

                            // Second Row: Customize toggle button + hint
                            ui.horizontal(|ui| {
                                let label = if state.customize_open {
                                    "▴ Hide Customize"
                                } else {
                                    "▾ Customize"
                                };
                                let cust_btn = Button::new(RichText::new(label).size(11.5))
                                    .stroke(Stroke::new(
                                        1.0,
                                        if state.customize_open {
                                            accent
                                        } else {
                                            Color32::from_rgba_unmultiplied(251, 254, 254, 45)
                                        },
                                    ));
                                if ui.add(cust_btn).clicked() {
                                    state.customize_open = !state.customize_open;
                                }

                                ui.add_space(6.0);
                                ui.label(
                                    RichText::new("Drag box to move · All 4 sides & corners resize freely")
                                        .size(10.5)
                                        .color(Color32::from_rgba_unmultiplied(251, 254, 254, 155)),
                                );
                            });

                            // Expandable Customization Drawer
                            if state.customize_open {
                                ui.add_space(8.0);
                                ui.separator();
                                ui.add_space(8.0);

                                Grid::new("hud_customize_grid")
                                    .num_columns(2)
                                    .spacing([14.0, 8.0])
                                    .min_col_width(135.0)
                                    .show(ui, |ui| {
                                        ui.label("Font size (pt)");
                                        ui.add(
                                            Slider::new(&mut state.draft.font_size, 2.0..=36.0)
                                                .fixed_decimals(0),
                                        );
                                        ui.end_row();

                                        ui.label("Visibility (Opacity)");
                                        ui.add(
                                            Slider::new(&mut state.draft.opacity, 0.0..=1.0)
                                                .fixed_decimals(2),
                                        );
                                        ui.end_row();

                                        ui.label("Corner roundness");
                                        ui.add(
                                            Slider::new(&mut state.draft.hud_rounding, 0.0..=28.0)
                                                .fixed_decimals(0),
                                        );
                                        ui.end_row();

                                        ui.label("Inner padding");
                                        ui.add(
                                            Slider::new(&mut state.draft.hud_padding, 4.0..=32.0)
                                                .fixed_decimals(0),
                                        );
                                        ui.end_row();

                                        ui.label("Box color");
                                        ui.horizontal(|ui| {
                                            let mut rgb = state.draft.hud_box_color.unwrap_or(
                                                if state.draft.light_mode {
                                                    [255, 255, 255]
                                                } else {
                                                    [18, 18, 20]
                                                },
                                            );
                                            if ui.color_edit_button_srgb(&mut rgb).changed() {
                                                state.draft.hud_box_color = Some(rgb);
                                            }
                                            ui.label(
                                                RichText::new(format!(
                                                    "#{:02X}{:02X}{:02X}",
                                                    rgb[0], rgb[1], rgb[2]
                                                ))
                                                .monospace()
                                                .size(11.5),
                                            );
                                            if state.draft.hud_box_color.is_some()
                                                && ui.small_button("Reset").clicked()
                                            {
                                                state.draft.hud_box_color = None;
                                            }
                                        });
                                        ui.end_row();

                                        ui.label("Text color");
                                        ui.horizontal(|ui| {
                                            let mut rgb = state.draft.hud_text_color.unwrap_or(
                                                if state.draft.light_mode {
                                                    [18, 18, 20]
                                                } else {
                                                    [251, 254, 254]
                                                },
                                            );
                                            if ui.color_edit_button_srgb(&mut rgb).changed() {
                                                state.draft.hud_text_color = Some(rgb);
                                            }
                                            ui.label(
                                                RichText::new(format!(
                                                    "#{:02X}{:02X}{:02X}",
                                                    rgb[0], rgb[1], rgb[2]
                                                ))
                                                .monospace()
                                                .size(11.5),
                                            );
                                            if state.draft.hud_text_color.is_some()
                                                && ui.small_button("Reset").clicked()
                                            {
                                                state.draft.hud_text_color = None;
                                            }
                                        });
                                        ui.end_row();

                                        ui.label("Text alignment");
                                        ui.horizontal(|ui| {
                                            for (align, lbl) in [
                                                (HudTextAlign::Left, "Left"),
                                                (HudTextAlign::Center, "Center"),
                                                (HudTextAlign::Right, "Right"),
                                            ] {
                                                let sel = state.draft.hud_text_align == align;
                                                let btn = Button::new(RichText::new(lbl).color(
                                                    if sel { on_accent } else { Color32::WHITE },
                                                ))
                                                .fill(if sel {
                                                    accent
                                                } else {
                                                    Color32::from_rgb(48, 48, 48)
                                                })
                                                .stroke(Stroke::NONE);
                                                if ui.add(btn).clicked() {
                                                    state.draft.hud_text_align = align;
                                                }
                                            }
                                        });
                                        ui.end_row();

                                        ui.label("Options");
                                        ui.horizontal(|ui| {
                                            ui.label("Box");
                                            blur_toggle(
                                                ui,
                                                &mut state.draft.hud_show_box,
                                                accent,
                                                false,
                                            );
                                            ui.add_space(12.0);
                                            ui.label("Border");
                                            blur_toggle(
                                                ui,
                                                &mut state.draft.hud_border,
                                                accent,
                                                false,
                                            );
                                            ui.add_space(12.0);
                                            ui.label("Real screen");
                                            blur_toggle(
                                                ui,
                                                &mut state.show_real_screen,
                                                accent,
                                                false,
                                            );
                                        });
                                        ui.end_row();
                                    });
                            }
                        });
                    });
            });
    });

    outcome
}
