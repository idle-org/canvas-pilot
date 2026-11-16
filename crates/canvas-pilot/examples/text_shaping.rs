//! Offline typography diagnostic; no device or user-settings access.
//! Run with `cargo run --example text_shaping` and compare with the same Noto fonts
//! in a browser or Pango. Editing the samples never modifies a project.
use eframe::egui;

#[path = "../src/language_packs.rs"]
#[allow(dead_code)]
mod language_packs;
#[path = "../src/localization.rs"]
mod localization;
rust_i18n::i18n!(
    "locales",
    fallback = "en",
    backend = language_packs::Backend
);

struct Probe {
    language: localization::Language,
    samples: Vec<(&'static str, String)>,
}
impl Default for Probe {
    fn default() -> Self {
        Self {
            language: localization::Language::En,
            samples: [
                (
                    "Nepali",
                    "नेपाली भाषा · कि की कु कृ · क्ष त्र ज्ञ श्र · कक्षा क्षितिज",
                ),
                ("Hindi", "हिन्दी भाषा · हिन्दी में लिखिए · क्ष त्र ज्ञ श्र · क्रिया"),
                ("Thai", "ภาษาไทย · กิ กี กุ กู้ กิ้ · น้ำ เก้า · การตั้งค่า"),
                (
                    "Mixed",
                    "Preview 1920×1080 · नेपाली हिन्दी ไทย · 日本語 简体中文",
                ),
                (
                    "Latin / Greek",
                    "Português · español · français · ĈĜĤĴŜŬ · Ἑλληνικὴ ἀρχαία",
                ),
                (
                    "Toki Pona",
                    "toki pona · o ante e suli sitelen · o awen e pali",
                ),
            ]
            .into_iter()
            .map(|(label, text)| (label, text.into()))
            .collect(),
        }
    }
}
impl eframe::App for Probe {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Canvas Pilot — text shaping");
            ui.label("Offline diagnostic: compare labels, wrapping and editable Unicode text. No device connection.");
            let before = self.language.clone();
            egui::ComboBox::from_id_salt("probe-language")
                .height(440.0)
                .selected_text(self.language.name(ui.ctx()))
                .show_ui(ui, |ui| {
                    for language in localization::Language::ALL {
                        let name = language.name(ui.ctx());
                        ui.selectable_value(&mut self.language, language, name);
                    }
                });
            if before != self.language { self.language.apply(ui.ctx()); }
            ui.label(localization::tr(ui.ctx(), "language.confirm"));
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (label, text) in &mut self.samples {
                    ui.separator();
                    ui.strong(*label);
                    ui.label(egui::RichText::new(text.as_str()).size(24.0));
                    ui.add(egui::TextEdit::singleline(text).font(egui::FontId::proportional(24.0)).desired_width(f32::INFINITY));
                    ui.horizontal(|ui| {
                        ui.set_width(300.0);
                        ui.add(egui::Label::new(text.as_str()).wrap());
                    });
                }
            });
        });
    }
}
fn main() -> eframe::Result {
    eframe::run_native(
        "Canvas Pilot — text shaping",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 900.0]),
            ..Default::default()
        },
        Box::new(|cc| {
            localization::Language::En.apply(&cc.egui_ctx);
            Ok(Box::<Probe>::default())
        }),
    )
}
