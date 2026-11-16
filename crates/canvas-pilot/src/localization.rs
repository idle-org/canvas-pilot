//! Presentation-only localization. Catalog lookup and interpolation use rust-i18n.
use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    #[default]
    System,
    En,
    Fr,
}
impl Language {
    pub const ALL: [Self; 3] = [Self::System, Self::En, Self::Fr];

    pub fn apply(self, ctx: &egui::Context) {
        let locale = match self {
            Self::En => "en",
            Self::Fr => "fr",
            Self::System => system_language(sys_locale::get_locale().as_deref()),
        };
        ctx.data_mut(|data| data.insert_temp(egui::Id::new("application-locale"), locale));
        ctx.request_repaint();
    }
}

fn system_language(locale: Option<&str>) -> &'static str {
    match locale.and_then(|value| value.split(['-', '_', '.']).next()) {
        Some(value) if value.eq_ignore_ascii_case("fr") => "fr",
        _ => "en",
    }
}

pub fn locale(ctx: &egui::Context) -> &'static str {
    ctx.data(|data| data.get_temp::<&'static str>(egui::Id::new("application-locale")))
        .unwrap_or("en")
}

pub fn tr(ctx: &egui::Context, key: &str) -> String {
    rust_i18n::t!(key, locale = locale(ctx)).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn regional_system_locales_and_unsupported_languages() {
        for value in ["fr", "fr-FR", "fr_CA.UTF-8", "FR-be"] {
            assert_eq!(system_language(Some(value)), "fr");
        }
        for value in [None, Some(""), Some("C"), Some("de-DE"), Some("en-GB")] {
            assert_eq!(system_language(value), "en");
        }
    }

    #[test]
    fn contexts_are_independent_and_missing_locales_fall_back_to_english() {
        let french = egui::Context::default();
        let english = egui::Context::default();
        Language::Fr.apply(&french);
        Language::En.apply(&english);
        assert_eq!(tr(&french, "ui.close"), "Fermer");
        assert_eq!(tr(&english, "ui.close"), "Close");
        assert_eq!(rust_i18n::t!("ui.close", locale = "zz"), "Close");
        assert_eq!(
            rust_i18n::t!("theme.current", locale = "fr", name = "Midra"),
            "Thème actuel : Midra"
        );
        Language::En.apply(&french);
        assert_eq!(tr(&french, "ui.close"), "Close");
        assert_eq!(tr(&egui::Context::default(), "ui.close"), "Close");
    }

    #[test]
    fn catalogs_cover_each_other_and_ui_keys_with_matching_placeholders() {
        let english: BTreeMap<String, String> =
            serde_json::from_str(include_str!("../locales/en.json")).unwrap();
        let french: BTreeMap<String, String> =
            serde_json::from_str(include_str!("../locales/fr.json")).unwrap();
        assert_eq!(
            english.keys().collect::<Vec<_>>(),
            french.keys().collect::<Vec<_>>()
        );
        let placeholders = |text: &str| -> BTreeSet<String> {
            text.split("%{")
                .skip(1)
                .map(|part| part.split('}').next().unwrap().to_owned())
                .collect()
        };
        for (key, text) in &english {
            assert!(!text.trim().is_empty(), "empty English message: {key}");
            assert!(
                !french[key].trim().is_empty(),
                "empty French message: {key}"
            );
            assert_eq!(
                placeholders(text),
                placeholders(&french[key]),
                "placeholder mismatch: {key}"
            );
            assert_eq!(rust_i18n::t!(key.as_str(), locale = "en").as_ref(), text);
            assert_eq!(
                rust_i18n::t!(key.as_str(), locale = "fr").as_ref(),
                french[key]
            );
        }
        for source in [
            include_str!("gui.rs"),
            include_str!("menu_ui.rs"),
            include_str!("preferences_ui.rs"),
            include_str!("project_ui.rs"),
            include_str!("device_ui.rs"),
            include_str!("capture_ui.rs"),
            include_str!("layers_ui.rs"),
            include_str!("format_ui.rs"),
            include_str!("input_ui.rs"),
            include_str!("ratio_ui.rs"),
            include_str!("reference_ui.rs"),
            include_str!("recovery_ui.rs"),
        ] {
            for key in source.split('"').filter(|part| {
                part.starts_with("ui.")
                    || part.starts_with("language.")
                    || part.starts_with("theme.current")
            }) {
                assert!(english.contains_key(key), "missing catalog key: {key}");
            }
        }
    }
}
