//! Editable session-only aspect ratios; geometry remains the saved source of truth.
use eframe::egui;

pub(super) fn label(ratio: f64) -> String {
    for (w, h) in [(16, 9), (4, 3), (16, 10), (1, 1), (9, 16), (3, 4), (21, 9)] {
        if (ratio - w as f64 / h as f64).abs() < 1e-8 {
            return format!("{w}:{h}");
        }
    }
    if !(0.0001..=1e6).contains(&ratio) {
        return format!("{ratio:.6e}:1");
    }
    let value = format!("{ratio:.6}");
    format!("{}:1", value.trim_end_matches('0').trim_end_matches('.'))
}

pub(super) fn parse(text: &str) -> Result<f64, String> {
    let positive = |s: &str| {
        s.trim()
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite() && *v > 0.0)
    };
    let value = if let Some((w, h)) = text.split_once([':', '/']) {
        positive(w).zip(positive(h)).map(|(w, h)| w / h)
    } else {
        positive(text)
    };
    value
        .filter(|v| v.is_finite() && *v > 0.0)
        .ok_or_else(|| "Enter a positive ratio, e.g. 16:9, 4/3 or 1.85.".into())
}

#[derive(Default)]
pub(super) struct RatioEdit {
    pub text: String,
    pub dirty: bool,
    pub target: Option<f64>,
    pub error: String,
}
impl RatioEdit {
    pub fn commit(&mut self) -> Result<Option<f64>, String> {
        if !self.dirty {
            return Ok(None);
        }
        match parse(&self.text) {
            Ok(ratio) => {
                self.target = Some(ratio);
                self.dirty = false;
                self.error.clear();
                Ok(Some(ratio))
            }
            Err(error) => {
                self.error = error.clone();
                Err(error)
            }
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui, current: f64) -> bool {
        if !self.dirty && self.target.is_none() {
            self.text = label(current);
        }
        let mut committed = false;
        ui.horizontal(|ui| {
            ui.label(format!("Ratio {}", label(current)));
            let response = ui.add(egui::TextEdit::singleline(&mut self.text).desired_width(90.0))
                .on_hover_text("Enter width:height, width/height or a decimal. With aspect lock on, keeps width and adjusts height.");
            self.dirty |= response.changed();
            if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                *self = Self::default();
            } else if response.lost_focus() && self.dirty {
                committed = self.commit().is_ok();
            }
        });
        if !self.error.is_empty() {
            ui.colored_label(ui.visuals().error_fg_color, &self.error);
        }
        committed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ratio_notation_and_labels() {
        for (text, expected) in [
            ("16:9", 16.0 / 9.0),
            ("4 / 3", 4.0 / 3.0),
            ("1.85", 1.85),
            (" 9 : 16 ", 9.0 / 16.0),
        ] {
            assert_eq!(parse(text).unwrap(), expected);
        }
        for text in [
            "0",
            "-2:3",
            "3:0",
            "nan",
            "inf:2",
            "2/3/4",
            "",
            "1e308:1e-308",
        ] {
            assert!(parse(text).is_err(), "{text}");
        }
        assert_eq!(label(16.0 / 9.0), "16:9");
        assert_eq!(label(1.85), "1.85:1");
        assert!(parse(&label(1e-9)).unwrap() > 0.0);
    }
}
