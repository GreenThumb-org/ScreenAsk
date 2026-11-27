use crate::config::DEFAULT_ACCENT_COLOR;
use crate::ui::theme::popover_frame;
use egui::{
    epaint::{Mesh, Vertex},
    Area, Color32, CornerRadius, Id, Order, Pos2, Rect, RichText, Sense, Stroke, TextEdit, Ui,
    Vec2,
};

pub struct AccentColorPicker {
    pub open: bool,
    hue: f32,
    sat: f32,
    val: f32,
    hex_buf: String,
    trigger_rect: Rect,
}

impl AccentColorPicker {
    pub fn new(initial: [u8; 3]) -> Self {
        let (h, s, v) = rgb_to_hsv(initial);
        Self {
            open: false,
            hue: h,
            sat: s,
            val: v,
            hex_buf: format_hex(initial),
            trigger_rect: Rect::NOTHING,
        }
    }

    pub fn sync_from_rgb(&mut self, rgb: [u8; 3]) {
        let (h, s, v) = rgb_to_hsv(rgb);
        self.hue = h;
        self.sat = s;
        self.val = v;
        self.hex_buf = format_hex(rgb);
    }

    /// Renders the Blur-style Accent Color row (swatch + hex + Reset) and returns `true` if changed.
    pub fn row_ui(&mut self, ui: &mut Ui, rgb: &mut [u8; 3], light_mode: bool) -> bool {
        let mut changed = false;
        let current = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("Accent Color").strong());
                ui.label(
                    RichText::new("Color of toggles, sliders and active tabs.")
                        .size(11.0)
                        .weak(),
                );
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Reset").clicked() {
                    *rgb = DEFAULT_ACCENT_COLOR;
                    self.sync_from_rgb(*rgb);
                    changed = true;
                }

                ui.add_space(4.0);
                ui.label(
                    RichText::new(format_hex(*rgb))
                        .monospace()
                        .size(12.0),
                );
                ui.add_space(4.0);

                // Rounded square swatch trigger button
                let swatch_outer = Vec2::splat(30.0);
                let (rect, resp) = ui.allocate_exact_size(swatch_outer, Sense::click());
                self.trigger_rect = rect;

                let frame_fill = if light_mode {
                    Color32::from_rgb(242, 242, 242)
                } else {
                    Color32::from_rgb(38, 38, 38)
                };
                let stroke_col = if self.open || resp.hovered() {
                    current
                } else if light_mode {
                    Color32::from_rgb(166, 166, 166)
                } else {
                    Color32::from_rgba_unmultiplied(251, 254, 254, 52)
                };
                ui.painter().rect(
                    rect,
                    CornerRadius::same(7),
                    frame_fill,
                    Stroke::new(1.2, stroke_col),
                    egui::StrokeKind::Inside,
                );
                ui.painter()
                    .rect_filled(rect.shrink(4.0), CornerRadius::same(4), current);

                if resp.clicked() {
                    self.open = !self.open;
                    if self.open {
                        self.sync_from_rgb(*rgb);
                    }
                }
            });
        });

        changed
    }

    /// Renders the floating HSV popover when open. Returns `true` if the color changed.
    pub fn popover_ui(&mut self, ctx: &egui::Context, rgb: &mut [u8; 3], light_mode: bool) -> bool {
        if !self.open {
            return false;
        }

        let mut changed = false;
        let pop_w = 212.0;
        let pop_h = 218.0;
        let screen = ctx.screen_rect();
        let desired_x = (self.trigger_rect.right() - pop_w)
            .clamp(screen.left() + 8.0, (screen.right() - pop_w - 8.0).max(8.0));
        let desired_y = if self.trigger_rect.bottom() + pop_h + 8.0 <= screen.bottom() {
            self.trigger_rect.bottom() + 6.0
        } else {
            (self.trigger_rect.top() - pop_h - 6.0).max(screen.top() + 8.0)
        };

        let area_resp = Area::new(Id::new("blur_accent_color_popover"))
            .order(Order::Foreground)
            .fixed_pos(Pos2::new(desired_x, desired_y))
            .show(ctx, |ui| {
                popover_frame(light_mode).show(ui, |ui| {
                    ui.set_width(pop_w - 20.0);

                    ui.horizontal(|ui| {
                        // 2D Saturation-Value square
                        let sv_size = Vec2::splat(158.0);
                        let (sv_rect, sv_resp) = ui.allocate_exact_size(sv_size, Sense::click_and_drag());
                        paint_sv_square(ui, sv_rect, self.hue);

                        if sv_resp.clicked() || sv_resp.dragged() {
                            if let Some(pos) = sv_resp.interact_pointer_pos() {
                                self.sat = ((pos.x - sv_rect.left()) / sv_rect.width()).clamp(0.0, 1.0);
                                self.val = (1.0 - (pos.y - sv_rect.top()) / sv_rect.height()).clamp(0.0, 1.0);
                                *rgb = hsv_to_rgb(self.hue, self.sat, self.val);
                                self.hex_buf = format_hex(*rgb);
                                changed = true;
                            }
                        }

                        // Circular thumb on SV square
                        let thumb_pos = Pos2::new(
                            sv_rect.left() + self.sat * sv_rect.width(),
                            sv_rect.top() + (1.0 - self.val) * sv_rect.height(),
                        );
                        let cur_col = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                        ui.painter().circle(
                            thumb_pos,
                            6.0,
                            cur_col,
                            Stroke::new(2.0, Color32::WHITE),
                        );
                        ui.painter().circle_stroke(
                            thumb_pos,
                            7.0,
                            Stroke::new(1.0, Color32::from_black_alpha(150)),
                        );

                        ui.add_space(6.0);

                        // Vertical Rainbow Hue bar
                        let hue_size = Vec2::new(18.0, 158.0);
                        let (hue_rect, hue_resp) =
                            ui.allocate_exact_size(hue_size, Sense::click_and_drag());
                        paint_hue_bar(ui, hue_rect);

                        if hue_resp.clicked() || hue_resp.dragged() {
                            if let Some(pos) = hue_resp.interact_pointer_pos() {
                                self.hue = ((pos.y - hue_rect.top()) / hue_rect.height()).clamp(0.0, 1.0);
                                *rgb = hsv_to_rgb(self.hue, self.sat, self.val);
                                self.hex_buf = format_hex(*rgb);
                                changed = true;
                            }
                        }

                        // Horizontal white pill thumb on Hue bar
                        let hue_y = hue_rect.top() + self.hue * hue_rect.height();
                        let pill = Rect::from_center_size(
                            Pos2::new(hue_rect.center().x, hue_y),
                            Vec2::new(hue_rect.width() + 4.0, 5.0),
                        );
                        ui.painter().rect(
                            pill,
                            CornerRadius::same(3),
                            Color32::WHITE,
                            Stroke::new(1.0, Color32::from_black_alpha(130)),
                            egui::StrokeKind::Outside,
                        );
                    });

                    ui.add_space(8.0);

                    // Bottom preview swatch + editable hex input
                    ui.horizontal(|ui| {
                        let cur_col = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                        let (prev_rect, _) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::hover());
                        ui.painter()
                            .rect_filled(prev_rect, CornerRadius::same(5), cur_col);

                        let hex_resp = ui.add(
                            TextEdit::singleline(&mut self.hex_buf)
                                .desired_width(152.0)
                                .font(egui::TextStyle::Monospace),
                        );
                        if hex_resp.changed() {
                            if let Some(parsed) = parse_hex(&self.hex_buf) {
                                *rgb = parsed;
                                let (h, s, v) = rgb_to_hsv(parsed);
                                self.hue = h;
                                self.sat = s;
                                self.val = v;
                                changed = true;
                            }
                        }
                    });
                });
            });

        // Close popover on Escape or clicking outside both popover and trigger swatch
        let pop_rect = area_resp.response.rect;
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.open = false;
        } else if ctx.input(|i| i.pointer.any_pressed()) {
            if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                if !pop_rect.contains(pos) && !self.trigger_rect.contains(pos) {
                    self.open = false;
                }
            }
        }

        changed
    }
}

fn paint_sv_square(ui: &Ui, rect: Rect, hue: f32) {
    const N: usize = 16;
    let mut mesh = Mesh::default();

    for iy in 0..=N {
        let v = 1.0 - (iy as f32 / N as f32);
        let y = egui::lerp(rect.top()..=rect.bottom(), iy as f32 / N as f32);
        for ix in 0..=N {
            let s = ix as f32 / N as f32;
            let x = egui::lerp(rect.left()..=rect.right(), s);
            let [r, g, b] = hsv_to_rgb(hue, s, v);
            mesh.vertices.push(Vertex {
                pos: Pos2::new(x, y),
                uv: egui::epaint::WHITE_UV,
                color: Color32::from_rgb(r, g, b),
            });
        }
    }

    let stride = (N + 1) as u32;
    for iy in 0..(N as u32) {
        for ix in 0..(N as u32) {
            let i0 = iy * stride + ix;
            let i1 = i0 + 1;
            let i2 = i0 + stride;
            let i3 = i2 + 1;
            mesh.indices.extend_from_slice(&[i0, i1, i2, i1, i3, i2]);
        }
    }

    ui.painter().add(egui::Shape::mesh(mesh));
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(4),
        Stroke::new(1.0, Color32::from_black_alpha(90)),
        egui::StrokeKind::Inside,
    );
}

fn paint_hue_bar(ui: &Ui, rect: Rect) {
    let stops: [Color32; 7] = [
        Color32::from_rgb(255, 0, 0),
        Color32::from_rgb(255, 255, 0),
        Color32::from_rgb(0, 255, 0),
        Color32::from_rgb(0, 255, 255),
        Color32::from_rgb(0, 0, 255),
        Color32::from_rgb(255, 0, 255),
        Color32::from_rgb(255, 0, 0),
    ];

    let mut mesh = Mesh::default();
    for (i, &col) in stops.iter().enumerate() {
        let t = i as f32 / (stops.len() - 1) as f32;
        let y = egui::lerp(rect.top()..=rect.bottom(), t);
        mesh.vertices.push(Vertex {
            pos: Pos2::new(rect.left(), y),
            uv: egui::epaint::WHITE_UV,
            color: col,
        });
        mesh.vertices.push(Vertex {
            pos: Pos2::new(rect.right(), y),
            uv: egui::epaint::WHITE_UV,
            color: col,
        });
    }

    for i in 0..((stops.len() - 1) as u32) {
        let base = i * 2;
        mesh.indices
            .extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
    }

    ui.painter().add(egui::Shape::mesh(mesh));
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(4),
        Stroke::new(1.0, Color32::from_black_alpha(90)),
        egui::StrokeKind::Inside,
    );
}

pub fn format_hex(rgb: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}

fn parse_hex(s: &str) -> Option<[u8; 3]> {
    let hex = s.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([r, g, b])
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [u8; 3] {
    let h6 = (h.fract() + 1.0).fract() * 6.0;
    let c = v * s;
    let x = c * (1.0 - (h6 % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h6 as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [
        ((r + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

fn rgb_to_hsv(rgb: [u8; 3]) -> (f32, f32, f32) {
    let r = rgb[0] as f32 / 255.0;
    let g = rgb[1] as f32 / 255.0;
    let b = rgb[2] as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let h = if delta <= f32::EPSILON {
        0.0
    } else if (max - r).abs() <= f32::EPSILON {
        ((g - b) / delta).rem_euclid(6.0) / 6.0
    } else if (max - g).abs() <= f32::EPSILON {
        (((b - r) / delta) + 2.0) / 6.0
    } else {
        (((r - g) / delta) + 4.0) / 6.0
    };

    let s = if max <= f32::EPSILON { 0.0 } else { delta / max };
    (h, s, max)
}
