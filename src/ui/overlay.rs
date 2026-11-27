use crate::config::{Config, HudTextAlign};
use crate::ui::theme::overlay_colors;
use crate::win_util::FocusedMonitorInfo;
use egui::{
    Color32, CornerRadius, FontId, Frame, Layout, Margin, Stroke, ViewportBuilder, ViewportId,
};
use std::time::{Duration, Instant};

pub const SCREEN_MARGIN: f32 = 16.0;

pub struct OverlayState {
    pub text: String,
    pub monitor: FocusedMonitorInfo,
    pub hide_at: Option<Instant>,
    pub measured_height: f32,
}

pub fn resolve_overlay_style(cfg: &Config) -> (Color32, Color32, Stroke) {
    let (default_bg, default_fg, default_border) = overlay_colors(cfg.light_mode, cfg.opacity);
    let alpha_u8 = (cfg.opacity.clamp(0.0, 1.0) * 255.0).round() as u8;

    let fg = match cfg.hud_text_color {
        Some([r, g, b]) => Color32::from_rgb(r, g, b),
        None => default_fg,
    };

    if !cfg.hud_show_box {
        return (Color32::TRANSPARENT, fg, Stroke::NONE);
    }

    let bg = match cfg.hud_box_color {
        Some([r, g, b]) => Color32::from_rgba_unmultiplied(r, g, b, alpha_u8),
        None => Color32::from_rgba_unmultiplied(
            default_bg.r(),
            default_bg.g(),
            default_bg.b(),
            alpha_u8,
        ),
    };

    let stroke = if cfg.hud_border && alpha_u8 > 0 {
        Stroke::new(1.0, default_border)
    } else {
        Stroke::NONE
    };

    (bg, fg, stroke)
}

impl OverlayState {
    pub fn new(
        ctx: &egui::Context,
        text: String,
        monitor: FocusedMonitorInfo,
        cfg: &Config,
    ) -> Self {
        let hide_at = if cfg.overlay_seconds > 0 {
            Some(Instant::now() + Duration::from_secs(cfg.overlay_seconds as u64))
        } else {
            None
        };

        let pad = cfg.hud_padding.clamp(4.0, 32.0);
        let box_width = cfg.width.clamp(140.0, 1800.0);
        let font_px = pt_to_px(cfg.font_size);
        let wrap_width = (box_width - pad * 2.0).max(60.0);

        let estimated_text_h = ctx.fonts(|fonts| {
            fonts
                .layout(
                    text.clone(),
                    FontId::proportional(font_px),
                    Color32::WHITE,
                    wrap_width,
                )
                .size()
                .y
        });

        let auto_h = (estimated_text_h + pad * 2.0 + 6.0).clamp(40.0, 1000.0);
        let measured_height = match cfg.hud_height {
            Some(custom_h) => custom_h.max(auto_h),
            None => auto_h,
        };

        Self {
            text,
            monitor,
            hide_at,
            measured_height,
        }
    }
}

pub fn pt_to_px(pt: f32) -> f32 {
    (pt * (4.0 / 3.0)).clamp(2.5, 64.0)
}

/// Computes `(pos_x, pos_y)` in logical screen coordinates for an overlay of `(box_w, box_h)`
/// inside the given monitor's work area using normalized `(hud_x, hud_y)` in `0.0..=1.0`.
/// Defaults to bottom-left `(0.0, 1.0)` when unset.
pub fn compute_overlay_position(
    monitor: FocusedMonitorInfo,
    box_w: f32,
    box_h: f32,
    hud_x: Option<f32>,
    hud_y: Option<f32>,
) -> (f32, f32) {
    let scale = monitor.dpi_scale.max(0.5);
    let work_left = monitor.work.left as f32 / scale;
    let work_top = monitor.work.top as f32 / scale;
    let work_right = monitor.work.right as f32 / scale;
    let work_bottom = monitor.work.bottom as f32 / scale;

    let min_x = work_left + SCREEN_MARGIN;
    let max_x = (work_right - box_w - SCREEN_MARGIN).max(min_x);
    let min_y = work_top + SCREEN_MARGIN;
    let max_y = (work_bottom - box_h - SCREEN_MARGIN).max(min_y);

    let nx = hud_x.unwrap_or(0.0).clamp(0.0, 1.0);
    let ny = hud_y.unwrap_or(1.0).clamp(0.0, 1.0);

    let x = min_x + (max_x - min_x) * nx;
    let y = min_y + (max_y - min_y) * ny;
    (x, y)
}

pub fn render_overlay_viewport(ctx: &egui::Context, overlay: &mut OverlayState, cfg: &Config) {
    let scale = overlay.monitor.dpi_scale.max(0.5);
    let work_top = overlay.monitor.work.top as f32 / scale;
    let work_bottom = overlay.monitor.work.bottom as f32 / scale;
    let work_height = (work_bottom - work_top).max(120.0);

    let pad = cfg.hud_padding.clamp(4.0, 32.0).round();
    let corner_r = cfg.hud_rounding.clamp(0.0, 28.0).round() as u8;
    let box_width = cfg.width.clamp(140.0, 1800.0);
    let max_h = (work_height - SCREEN_MARGIN * 2.0).max(60.0);
    let box_height = overlay.measured_height.min(max_h);
    let (pos_x, pos_y) =
        compute_overlay_position(overlay.monitor, box_width, box_height, cfg.hud_x, cfg.hud_y);

    let viewport_id = ViewportId::from_hash_of("screenask_overlay_viewport");
    let builder = ViewportBuilder::default()
        .with_title("ScreenAsk Overlay")
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top()
        .with_taskbar(false)
        .with_active(false)
        .with_resizable(false)
        .with_mouse_passthrough(true)
        .with_inner_size([box_width, box_height])
        .with_position([pos_x, pos_y]);

    let (bg_color, fg_color, border_stroke) = resolve_overlay_style(cfg);
    let font_px = pt_to_px(cfg.font_size);
    let text = overlay.text.clone();
    let min_custom_h = cfg.hud_height.unwrap_or(40.0);
    let text_align = cfg.hud_text_align;
    let mut new_height: Option<f32> = None;

    ctx.show_viewport_immediate(viewport_id, builder, |ov_ctx, _class| {
        let frame = Frame::NONE
            .fill(bg_color)
            .corner_radius(CornerRadius::same(corner_r))
            .stroke(border_stroke)
            .inner_margin(Margin::same(pad as i8));

        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(Color32::TRANSPARENT))
            .show(ov_ctx, |ui| {
                let resp = frame.show(ui, |ui| {
                    ui.set_width((box_width - pad * 2.0 - 2.0).max(60.0));
                    ui.set_min_height((min_custom_h - pad * 2.0 - 2.0).max(16.0));
                    let layout = match text_align {
                        HudTextAlign::Left => Layout::top_down(egui::Align::LEFT),
                        HudTextAlign::Center => Layout::top_down(egui::Align::Center),
                        HudTextAlign::Right => Layout::top_down(egui::Align::RIGHT),
                    };
                    ui.with_layout(layout, |ui| {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&text)
                                    .font(FontId::proportional(font_px))
                                    .color(fg_color),
                            )
                            .wrap(),
                        );
                    });
                });

                let actual_h = (resp.response.rect.height() + 2.0)
                    .max(min_custom_h)
                    .clamp(40.0, max_h);
                if (actual_h - overlay.measured_height).abs() > 2.0 {
                    new_height = Some(actual_h);
                    let (ux, uy) = compute_overlay_position(
                        overlay.monitor,
                        box_width,
                        actual_h,
                        cfg.hud_x,
                        cfg.hud_y,
                    );
                    ov_ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                        box_width, actual_h,
                    )));
                    ov_ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                        ux, uy,
                    )));
                    ov_ctx.request_repaint();
                }
            });
    });

    if let Some(h) = new_height {
        overlay.measured_height = h;
    }
}
