//! Native offline layer positioning workspace.
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2};

const BG: Color32 = Color32::from_rgb(17, 23, 32);
const PANEL: Color32 = Color32::from_rgb(27, 35, 48);
const FIELD: Color32 = Color32::from_rgb(18, 26, 37);
const LINE: Color32 = Color32::from_rgb(53, 68, 88);
const TEXT: Color32 = Color32::from_rgb(237, 243, 252);
const MUTED: Color32 = Color32::from_rgb(165, 181, 201);
const CYAN: Color32 = Color32::from_rgb(102, 219, 239);
const AMBER: Color32 = Color32::from_rgb(246, 197, 119);
const PURPLE: Color32 = Color32::from_rgb(102, 85, 217);

fn main() -> eframe::Result {
    eframe::run_native(
        "Canvas Pilot",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_app_id("canvas-pilot")
                .with_icon(
                    eframe::icon_data::from_png_bytes(include_bytes!("../assets/app-icon.png"))
                        .expect("embedded application icon must be a valid PNG"),
                )
                .with_inner_size([1440.0, 940.0])
                .with_min_inner_size([1000.0, 720.0]),
            ..Default::default()
        },
        Box::new(|cc| {
            let mut style = (*cc.egui_ctx.global_style()).clone();
            style.visuals = egui::Visuals::dark();
            style.visuals.extreme_bg_color = FIELD;
            style.visuals.selection.bg_fill = PURPLE;
            style.visuals.widgets.inactive.bg_fill = FIELD;
            style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, LINE);
            style.spacing.item_spacing = Vec2::new(12.0, 10.0);
            cc.egui_ctx.set_global_style(style);
            Ok(Box::new(gui::Editor::with_saved_settings(&cc.egui_ctx)))
        }),
    )
}

rust_i18n::i18n!("locales", fallback = "en");
mod localization;
use localization::tr;

mod gui;
mod reference;
mod settings;
mod theme;

/// Draw in local logical points; viewport artwork alone uses a fitted scale.
struct Ink {
    painter: egui::Painter,
    origin: Pos2,
    scale: f32,
}

impl Ink {
    fn new(ui: &egui::Ui, rect: Rect) -> Self {
        Self {
            painter: ui.painter_at(rect),
            origin: rect.min,
            scale: 1.0,
        }
    }

    fn point(&self, x: f32, y: f32) -> Pos2 {
        self.origin + Vec2::new(x, y) * self.scale
    }

    fn rect(&self, x: f32, y: f32, w: f32, h: f32, fill: Color32) {
        self.painter.rect_filled(
            Rect::from_min_size(self.point(x, y), Vec2::new(w, h) * self.scale),
            0.0,
            fill,
        );
    }

    fn card(&self, x: f32, y: f32, w: f32, h: f32, fill: Color32, border: Color32) {
        self.painter.rect(
            Rect::from_min_size(self.point(x, y), Vec2::new(w, h) * self.scale),
            5.0,
            fill,
            Stroke::new(1.0_f32, border),
            StrokeKind::Inside,
        );
    }

    fn text(&self, x: f32, y: f32, text: &str, size: f32, color: Color32) {
        self.painter.text(
            self.point(x, y),
            Align2::LEFT_TOP,
            text,
            FontId::proportional(size * self.scale),
            color,
        );
    }

    fn line(&self, from: [f32; 2], to: [f32; 2], color: Color32) {
        self.painter.line_segment(
            [self.point(from[0], from[1]), self.point(to[0], to[1])],
            Stroke::new(1.0_f32, color),
        );
    }

    fn button(&self, x: f32, y: f32, w: f32, label: &str, primary: bool) {
        let palette = theme::palette(self.painter.ctx());
        self.card(
            x,
            y,
            w,
            34.0,
            if primary {
                palette.purple
            } else {
                palette.field
            },
            palette.line,
        );
        self.text(x + 12.0, y + 9.0, label, 13.0, palette.text);
    }

    fn field(&self, x: f32, y: f32, w: f32, label: &str, value: &str) {
        let palette = theme::palette(self.painter.ctx());
        self.text(x, y, label, 12.0, palette.muted);
        self.card(x, y + 23.0, w, 42.0, palette.field, palette.line);
        self.text(x + 12.0, y + 34.0, value, 18.0, palette.text);
    }
}

const SCREEN_LABEL_TOP: f32 = 24.0;

fn artwork_scale(output: [u32; 2]) -> f32 {
    (output[0] as f32 / 1920.0).min(output[1] as f32 / 1080.0)
}

fn screen_labels(ink: &Ink, output: [u32; 2]) {
    let palette = theme::palette(ink.painter.ctx());
    let width = output[0] as f32 / 2.0;
    let k = artwork_scale(output);
    ink.text(
        0.0,
        -SCREEN_LABEL_TOP * k,
        &tr(ink.painter.ctx(), "viewport.screen"),
        12.0 * k,
        palette.muted,
    );
    ink.painter.text(
        ink.point(width, -SCREEN_LABEL_TOP * k),
        Align2::RIGHT_TOP,
        format!("{} × {} px", output[0], output[1]),
        FontId::proportional(13.0 * k * ink.scale),
        palette.muted,
    );
}

fn screen_artwork(ink: &Ink, title: &str, output: [u32; 2]) {
    let palette = theme::palette(ink.painter.ctx());
    let width = output[0] as f32 / 2.0;
    let height = output[1] as f32 / 2.0;
    let k = artwork_scale(output);
    ink.rect(0.0, 0.0, width, height, palette.canvas);
    screen_title(ink, title, output);
    ink.text(
        24.0 * k,
        height - 78.0 * k,
        "Ideas worth sharing.",
        27.0 * k,
        palette.muted,
    );
}

fn screen_title(ink: &Ink, title: &str, output: [u32; 2]) {
    let palette = theme::palette(ink.painter.ctx());
    let k = artwork_scale(output);
    let width = output[0] as f32 / 2.0;
    let height = 36.0 * k;
    let rect = Rect::from_min_max(ink.point(0.0, 0.0), ink.point(width, height));
    let banner = Ink {
        painter: ink
            .painter
            .with_clip_rect(ink.painter.clip_rect().intersect(rect)),
        origin: ink.origin,
        scale: ink.scale,
    };
    banner.rect(0.0, 0.0, width, height, palette.banner);
    banner.text(24.0 * k, 10.0 * k, title, 13.0 * k, palette.muted);
}

fn screen_reference_hint(ink: &Ink, output: [u32; 2]) {
    let size = Vec2::new(output[0] as f32, output[1] as f32) / 2.0;
    let rect = Rect::from_min_max(ink.origin, ink.point(size.x, size.y));
    let painter = ink
        .painter
        .with_clip_rect(ink.painter.clip_rect().intersect(rect));
    let color = theme::palette(ink.painter.ctx()).muted;
    let text = painter.layout(
        "Illustrative reference artwork · Drag & drop an image or use Import > Reference image…"
            .into(),
        FontId::proportional(12.0 * ink.scale),
        color,
        (size.x - 48.0).max(1.0) * ink.scale,
    );
    let pos = ink.point(24.0, size.y - 14.0) - Vec2::new(0.0, text.size().y);
    painter.galley(pos, text, color);
}
