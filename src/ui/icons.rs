use egui::{Color32, Pos2, Rect, Stroke, Ui, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleIcon {
    Capture,
    Overlay,
    Style,
    Api,
    Minimize,
    Close,
}

/// Paints crisp Blur/Lucide-style thin-stroke vector icons inside `rect`.
pub fn paint_icon(ui: &Ui, rect: Rect, icon: TitleIcon, color: Color32) {
    let p = ui.painter();
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.5;
    let stroke = Stroke::new(1.55, color);

    match icon {
        TitleIcon::Capture => {
            // Viewfinder / screen capture frame with center dot
            let r = s * 0.85;
            let arm = r * 0.52;
            // Top-left corner
            p.line_segment([Pos2::new(c.x - r, c.y - r + arm), Pos2::new(c.x - r, c.y - r)], stroke);
            p.line_segment([Pos2::new(c.x - r, c.y - r), Pos2::new(c.x - r + arm, c.y - r)], stroke);
            // Top-right corner
            p.line_segment([Pos2::new(c.x + r - arm, c.y - r), Pos2::new(c.x + r, c.y - r)], stroke);
            p.line_segment([Pos2::new(c.x + r, c.y - r), Pos2::new(c.x + r, c.y - r + arm)], stroke);
            // Bottom-left corner
            p.line_segment([Pos2::new(c.x - r, c.y + r - arm), Pos2::new(c.x - r, c.y + r)], stroke);
            p.line_segment([Pos2::new(c.x - r, c.y + r), Pos2::new(c.x - r + arm, c.y + r)], stroke);
            // Bottom-right corner
            p.line_segment([Pos2::new(c.x + r - arm, c.y + r), Pos2::new(c.x + r, c.y + r)], stroke);
            p.line_segment([Pos2::new(c.x + r, c.y + r), Pos2::new(c.x + r, c.y + r - arm)], stroke);
            // Center dot
            p.circle_filled(c, s * 0.22, color);
        }
        TitleIcon::Overlay => {
            // Stacked layers icon (matching Blur's 3rd icon)
            let w = s * 0.92;
            let h = s * 0.42;
            // Top diamond
            let top_y = c.y - s * 0.35;
            let pts = vec![
                Pos2::new(c.x, top_y - h),
                Pos2::new(c.x + w, top_y),
                Pos2::new(c.x, top_y + h),
                Pos2::new(c.x - w, top_y),
            ];
            p.add(egui::Shape::closed_line(pts, stroke));
            // Middle chevron
            let mid_y = c.y + s * 0.08;
            p.line_segment([Pos2::new(c.x - w, mid_y), Pos2::new(c.x, mid_y + h)], stroke);
            p.line_segment([Pos2::new(c.x, mid_y + h), Pos2::new(c.x + w, mid_y)], stroke);
            // Bottom chevron
            let bot_y = c.y + s * 0.44;
            p.line_segment([Pos2::new(c.x - w, bot_y), Pos2::new(c.x, bot_y + h)], stroke);
            p.line_segment([Pos2::new(c.x, bot_y + h), Pos2::new(c.x + w, bot_y)], stroke);
        }
        TitleIcon::Style => {
            // Gear / settings icon (matching Blur's 1st icon)
            let outer_r = s * 0.62;
            let inner_r = s * 0.28;
            p.circle_stroke(c, outer_r, stroke);
            p.circle_stroke(c, inner_r, stroke);
            let tooth_in = outer_r;
            let tooth_out = s * 0.92;
            for i in 0..8 {
                let ang = (i as f32) * std::f32::consts::FRAC_PI_4;
                let dir = Vec2::new(ang.cos(), ang.sin());
                p.line_segment([c + dir * tooth_in, c + dir * tooth_out], stroke);
            }
        }
        TitleIcon::Api => {
            // Key icon (for Gemini API)
            let head = Pos2::new(c.x - s * 0.36, c.y);
            let head_r = s * 0.38;
            p.circle_stroke(head, head_r, stroke);
            // Shaft
            let shaft_start = Pos2::new(head.x + head_r, c.y);
            let shaft_end = Pos2::new(c.x + s * 0.88, c.y);
            p.line_segment([shaft_start, shaft_end], stroke);
            // Two teeth
            p.line_segment(
                [
                    Pos2::new(c.x + s * 0.48, c.y),
                    Pos2::new(c.x + s * 0.48, c.y + s * 0.36),
                ],
                stroke,
            );
            p.line_segment(
                [
                    Pos2::new(c.x + s * 0.82, c.y),
                    Pos2::new(c.x + s * 0.82, c.y + s * 0.42),
                ],
                stroke,
            );
        }
        TitleIcon::Minimize => {
            let w = s * 0.72;
            p.line_segment([Pos2::new(c.x - w, c.y), Pos2::new(c.x + w, c.y)], stroke);
        }
        TitleIcon::Close => {
            let r = s * 0.56;
            p.line_segment(
                [Pos2::new(c.x - r, c.y - r), Pos2::new(c.x + r, c.y + r)],
                stroke,
            );
            p.line_segment(
                [Pos2::new(c.x + r, c.y - r), Pos2::new(c.x - r, c.y + r)],
                stroke,
            );
        }
    }
}
