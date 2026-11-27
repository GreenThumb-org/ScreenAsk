use egui::{Color32, CornerRadius, Frame, Margin, Sense, Shadow, Stroke, Ui, Visuals};

pub fn accent_from_rgb(rgb: [u8; 3]) -> Color32 {
    Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}

pub fn contrast_text_on(bg: Color32) -> Color32 {
    let lum = 0.299 * (bg.r() as f32) + 0.587 * (bg.g() as f32) + 0.114 * (bg.b() as f32);
    if lum > 155.0 {
        Color32::from_rgb(18, 18, 20)
    } else {
        Color32::from_rgb(250, 252, 254)
    }
}

pub fn apply_blur_theme(ctx: &egui::Context, light_mode: bool, accent_rgb: [u8; 3]) {
    let accent = accent_from_rgb(accent_rgb);
    let mut visuals = if light_mode {
        build_blur_light(accent)
    } else {
        build_blur_dark(accent)
    };

    visuals.slider_trailing_fill = true;
    visuals.window_corner_radius = CornerRadius::same(10);
    visuals.menu_corner_radius = CornerRadius::same(8);
    visuals.popup_shadow = Shadow {
        offset: [0, 8],
        blur: 20,
        spread: 0,
        color: Color32::from_black_alpha(if light_mode { 40 } else { 150 }),
    };

    ctx.set_visuals(visuals);

    ctx.style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(10.0, 5.0);
        style.spacing.interact_size.y = 26.0;
        style.spacing.slider_width = 210.0;
    });
}

fn build_blur_dark(accent: Color32) -> Visuals {
    // Exact Blur-AutoClicker dark palette (pure black / neutral dark gray)
    let mut v = Visuals::dark();
    let bg_base = Color32::from_rgb(12, 12, 14); // --bg-base
    let bg_elevated = Color32::from_rgb(38, 38, 38); // --bg-elevated
    let bg_input = Color32::from_rgb(48, 48, 48); // slightly darker than --bg-input for crisp contrast
    let bg_hover = Color32::from_rgb(59, 59, 59); // --bg-input
    let bg_active = Color32::from_rgb(68, 68, 68);
    let border_subtle = Color32::from_rgba_unmultiplied(251, 254, 254, 28); // --border-subtle
    let border_focus = Color32::from_rgba_unmultiplied(251, 254, 254, 76); // --border-focus
    let text_primary = Color32::from_rgba_unmultiplied(251, 254, 254, 232); // --text-primary
    let text_muted = Color32::from_rgba_unmultiplied(251, 254, 254, 175); // --text-muted

    v.panel_fill = bg_base;
    v.window_fill = Color32::from_rgb(26, 26, 26); // --bg-surface
    v.extreme_bg_color = bg_elevated;
    v.faint_bg_color = Color32::from_rgb(26, 26, 26);
    v.override_text_color = Some(text_primary);
    v.selection.bg_fill = accent;
    v.selection.stroke = Stroke::new(1.0, contrast_text_on(accent));

    v.widgets.noninteractive.bg_fill = bg_base;
    v.widgets.noninteractive.weak_bg_fill = bg_elevated;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, border_subtle);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, text_muted);
    v.widgets.noninteractive.corner_radius = CornerRadius::same(7);

    v.widgets.inactive.bg_fill = bg_elevated;
    v.widgets.inactive.weak_bg_fill = bg_elevated;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, border_subtle);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, text_primary);
    v.widgets.inactive.corner_radius = CornerRadius::same(7);

    v.widgets.hovered.bg_fill = bg_hover;
    v.widgets.hovered.weak_bg_fill = bg_hover;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, border_focus);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    v.widgets.hovered.corner_radius = CornerRadius::same(7);

    v.widgets.active.bg_fill = bg_active;
    v.widgets.active.weak_bg_fill = bg_input;
    v.widgets.active.bg_stroke = Stroke::new(1.2, accent);
    v.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    v.widgets.active.corner_radius = CornerRadius::same(7);

    v.widgets.open.bg_fill = bg_elevated;
    v.widgets.open.weak_bg_fill = bg_elevated;
    v.widgets.open.bg_stroke = Stroke::new(1.0, accent);
    v.widgets.open.fg_stroke = Stroke::new(1.0, text_primary);
    v.widgets.open.corner_radius = CornerRadius::same(7);

    v
}

fn build_blur_light(accent: Color32) -> Visuals {
    // Exact Blur-AutoClicker light palette
    let mut v = Visuals::light();
    let bg_base = Color32::from_rgb(229, 223, 231); // --bg-base
    let bg_surface = Color32::from_rgb(255, 255, 255); // --bg-surface
    let bg_elevated = Color32::from_rgb(242, 242, 242); // --bg-elevated
    let bg_input = Color32::from_rgb(217, 217, 217); // --bg-input
    let border_subtle = Color32::from_rgb(166, 166, 166); // --border-subtle
    let border_focus = Color32::from_rgb(89, 89, 89); // --border-focus
    let text_primary = Color32::from_rgb(0, 0, 0);
    let text_muted = Color32::from_rgba_unmultiplied(0, 0, 0, 185);

    v.panel_fill = bg_base;
    v.window_fill = bg_surface;
    v.extreme_bg_color = bg_elevated;
    v.faint_bg_color = bg_surface;
    v.override_text_color = Some(text_primary);
    v.selection.bg_fill = accent;
    v.selection.stroke = Stroke::new(1.0, contrast_text_on(accent));

    v.widgets.noninteractive.bg_fill = bg_base;
    v.widgets.noninteractive.weak_bg_fill = bg_elevated;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, border_subtle);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, text_muted);
    v.widgets.noninteractive.corner_radius = CornerRadius::same(7);

    v.widgets.inactive.bg_fill = bg_elevated;
    v.widgets.inactive.weak_bg_fill = bg_elevated;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, border_subtle);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, text_primary);
    v.widgets.inactive.corner_radius = CornerRadius::same(7);

    v.widgets.hovered.bg_fill = bg_input;
    v.widgets.hovered.weak_bg_fill = bg_input;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, border_focus);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, text_primary);
    v.widgets.hovered.corner_radius = CornerRadius::same(7);

    v.widgets.active.bg_fill = bg_input;
    v.widgets.active.weak_bg_fill = bg_input;
    v.widgets.active.bg_stroke = Stroke::new(1.2, accent);
    v.widgets.active.fg_stroke = Stroke::new(1.0, text_primary);
    v.widgets.active.corner_radius = CornerRadius::same(7);

    v
}

pub fn card_frame(light_mode: bool) -> Frame {
    let (fill, stroke) = if light_mode {
        (
            Color32::from_rgb(255, 255, 255),
            Stroke::new(1.0, Color32::from_rgb(195, 195, 200)),
        )
    } else {
        (
            Color32::from_rgb(26, 26, 26), // Blur --bg-surface
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(251, 254, 254, 28)),
        )
    };

    Frame::NONE
        .fill(fill)
        .stroke(stroke)
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::symmetric(14, 12))
}

pub fn popover_frame(light_mode: bool) -> Frame {
    let (fill, stroke) = if light_mode {
        (
            Color32::from_rgb(248, 248, 248),
            Stroke::new(1.0, Color32::from_rgb(140, 140, 140)),
        )
    } else {
        (
            Color32::from_rgb(38, 38, 38), // Blur --bg-elevated
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(251, 254, 254, 52)),
        )
    };

    Frame::NONE
        .fill(fill)
        .stroke(stroke)
        .corner_radius(CornerRadius::same(10))
        .shadow(Shadow {
            offset: [0, 8],
            blur: 20,
            spread: 0,
            color: Color32::from_black_alpha(150),
        })
        .inner_margin(Margin::same(10))
}

pub fn overlay_colors(light_mode: bool, opacity: f32) -> (Color32, Color32, Color32) {
    let alpha_u8 = (opacity.clamp(0.2, 1.0) * 255.0).round() as u8;
    if light_mode {
        (
            Color32::from_rgba_unmultiplied(255, 255, 255, alpha_u8),
            Color32::from_rgb(18, 18, 20),
            Color32::from_rgba_unmultiplied(0, 0, 0, (alpha_u8 / 6).max(24)),
        )
    } else {
        (
            Color32::from_rgba_unmultiplied(18, 18, 20, alpha_u8),
            Color32::from_rgba_unmultiplied(251, 254, 254, 235),
            Color32::from_rgba_unmultiplied(251, 254, 254, (alpha_u8 / 7).max(24)),
        )
    }
}

/// Sleek pill toggle switch colored with the user's accent color (Blur style).
pub fn blur_toggle(ui: &mut Ui, on: &mut bool, accent: Color32, light_mode: bool) -> egui::Response {
    let desired_size = egui::vec2(38.0, 20.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool_responsive(response.id, *on);
        let off_bg = if light_mode {
            Color32::from_rgb(195, 195, 200)
        } else {
            Color32::from_rgb(51, 51, 51) // Blur --bg-input-off
        };
        let bg = Color32::from_rgb(
            egui::lerp((off_bg.r() as f32)..=(accent.r() as f32), how_on) as u8,
            egui::lerp((off_bg.g() as f32)..=(accent.g() as f32), how_on) as u8,
            egui::lerp((off_bg.b() as f32)..=(accent.b() as f32), how_on) as u8,
        );

        let radius = rect.height() * 0.5;
        ui.painter().rect_filled(rect, radius, bg);

        let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let center = egui::pos2(circle_x, rect.center().y);
        ui.painter()
            .circle_filled(center, radius - 2.5, Color32::WHITE);
    }

    response
}

/// Animated Moon <-> Sun switch for Dark / Light mode.
pub fn moon_sun_toggle(
    ui: &mut Ui,
    light_mode: &mut bool,
    accent: Color32,
) -> egui::Response {
    let desired_size = egui::vec2(54.0, 26.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click());
    if response.clicked() {
        *light_mode = !*light_mode;
        response.mark_changed();
    }

    if ui.is_rect_visible(rect) {
        let t = ui.ctx().animate_bool_with_time(response.id, *light_mode, 0.24);
        let p = ui.painter();
        let radius = rect.height() * 0.5;

        // Track background smoothly transitions from night dark gray to accent tint
        let night_bg = Color32::from_rgb(38, 40, 48);
        let day_bg = accent;
        let track_bg = Color32::from_rgb(
            egui::lerp(night_bg.r() as f32..=day_bg.r() as f32, t) as u8,
            egui::lerp(night_bg.g() as f32..=day_bg.g() as f32, t) as u8,
            egui::lerp(night_bg.b() as f32..=day_bg.b() as f32, t) as u8,
        );
        let border_col = Color32::from_rgba_unmultiplied(251, 254, 254, 42);
        p.rect(
            rect,
            CornerRadius::same(radius as u8),
            track_bg,
            Stroke::new(1.0, border_col),
            egui::StrokeKind::Inside,
        );

        // Subtle stars on the right side in dark mode (fade out as sun slides in)
        let star_alpha = ((1.0 - t) * 190.0).round().clamp(0.0, 255.0) as u8;
        if star_alpha > 8 {
            let star_col = Color32::from_rgba_unmultiplied(240, 244, 255, star_alpha);
            p.circle_filled(egui::pos2(rect.right() - 15.0, rect.top() + 8.5), 1.3, star_col);
            p.circle_filled(egui::pos2(rect.right() - 9.5, rect.bottom() - 8.5), 1.0, star_col);
            p.circle_filled(egui::pos2(rect.right() - 18.5, rect.bottom() - 9.5), 0.9, star_col);
        }

        // Sliding thumb
        let cx = egui::lerp((rect.left() + radius)..=(rect.right() - radius), t);
        let cy = rect.center().y;
        let center = egui::pos2(cx, cy);

        // Sun rays grow & gently rotate as t -> 1
        if t > 0.15 {
            let ray_t = ((t - 0.15) / 0.85).clamp(0.0, 1.0);
            let ray_alpha = (ray_t * 245.0).round() as u8;
            let ray_col = Color32::from_rgba_unmultiplied(255, 245, 210, ray_alpha);
            let inner_r = 5.4;
            let outer_r = 5.4 + 3.2 * ray_t;
            let rot = (1.0 - ray_t) * 0.6;
            for i in 0..8 {
                let ang = rot + (i as f32) * std::f32::consts::FRAC_PI_4;
                let dir = egui::vec2(ang.cos(), ang.sin());
                p.line_segment(
                    [center + dir * inner_r, center + dir * outer_r],
                    Stroke::new(1.3, ray_col),
                );
            }
        }

        // Main celestial disc (moon silver-white -> warm sun gold-white)
        let disc_r = egui::lerp(7.5..=5.0, t);
        let disc_col = Color32::from_rgb(
            egui::lerp(236.0..=255.0, t) as u8,
            egui::lerp(240.0..=248.0, t) as u8,
            egui::lerp(250.0..=214.0, t) as u8,
        );
        p.circle_filled(center, disc_r, disc_col);

        // Crescent cutout slides away smoothly as moon morphs into sun
        if t < 0.85 {
            let cut_t = (1.0 - t / 0.85).clamp(0.0, 1.0);
            let cut_offset = egui::vec2(egui::lerp(11.0..=3.2, cut_t), egui::lerp(-8.0..=-2.2, cut_t));
            let cut_r = 6.2 * cut_t;
            p.circle_filled(center + cut_offset, cut_r, track_bg);
        }
    }

    response
}

/// Sleek Windows autostart button with 4-pane Windows icon and Enabled/Disabled state.
pub fn autostart_button(
    ui: &mut Ui,
    enabled: &mut bool,
    accent: Color32,
    light_mode: bool,
) -> egui::Response {
    let desired_size = egui::vec2(108.0, 26.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click());
    if response.clicked() {
        *enabled = !*enabled;
        response.mark_changed();
    }

    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        let (bg, border, fg) = if *enabled {
            (
                Color32::from_rgba_unmultiplied(
                    accent.r(),
                    accent.g(),
                    accent.b(),
                    if light_mode { 48 } else { 42 },
                ),
                Stroke::new(1.2, accent),
                if light_mode {
                    Color32::from_rgb(18, 18, 22)
                } else {
                    Color32::from_rgb(248, 250, 252)
                },
            )
        } else {
            (
                if light_mode {
                    Color32::from_rgb(236, 236, 240)
                } else {
                    Color32::from_rgb(38, 38, 38)
                },
                Stroke::new(
                    1.0,
                    if response.hovered() {
                        accent
                    } else if light_mode {
                        Color32::from_rgb(170, 170, 176)
                    } else {
                        Color32::from_rgba_unmultiplied(251, 254, 254, 42)
                    },
                ),
                if light_mode {
                    Color32::from_black_alpha(180)
                } else {
                    Color32::from_rgba_unmultiplied(251, 254, 254, 175)
                },
            )
        };

        p.rect(
            rect,
            CornerRadius::same(7),
            bg,
            border,
            egui::StrokeKind::Inside,
        );

        // 4-pane Windows logo icon on the left
        let icon_col = if *enabled { accent } else { fg };
        let ix = rect.left() + 10.0;
        let iy = rect.center().y - 5.0;
        let cell = 4.2;
        let gap = 1.6;
        for (dx, dy) in [(0.0, 0.0), (cell + gap, 0.0), (0.0, cell + gap), (cell + gap, cell + gap)] {
            p.rect_filled(
                egui::Rect::from_min_size(egui::pos2(ix + dx, iy + dy), egui::vec2(cell, cell)),
                CornerRadius::same(1),
                icon_col,
            );
        }

        let label = if *enabled { "Enabled" } else { "Disabled" };
        p.text(
            egui::pos2(rect.left() + 28.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(12.0),
            fg,
        );
    }

    response
}
