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
    Ja,
    #[serde(rename = "zh-CN")]
    ZhCn,
    Es,
    Grc,
    Eo,
}
impl Language {
    pub const ALL: [Self; 8] = [
        Self::System,
        Self::En,
        Self::Fr,
        Self::Ja,
        Self::ZhCn,
        Self::Es,
        Self::Grc,
        Self::Eo,
    ];

    pub fn name(self, ctx: &egui::Context) -> String {
        let (native, key) = match self {
            Self::System => return tr(ctx, "language.system"),
            Self::En => ("English", "language.name.en"),
            Self::Fr => ("Français", "language.name.fr"),
            Self::Ja => ("日本語", "language.name.ja"),
            Self::ZhCn => ("简体中文", "language.name.zh-CN"),
            Self::Es => ("Español", "language.name.es"),
            Self::Grc => ("Ἑλληνικὴ ἀρχαία", "language.name.grc"),
            Self::Eo => ("Esperanto", "language.name.eo"),
        };
        format!("{native} — {}", tr(ctx, key))
    }

    pub fn apply(self, ctx: &egui::Context) {
        let locale = match self {
            Self::En => "en",
            Self::Fr => "fr",
            Self::Ja => "ja",
            Self::ZhCn => "zh-CN",
            Self::Es => "es",
            Self::Grc => "grc",
            Self::Eo => "eo",
            Self::System => system_language(sys_locale::get_locale().as_deref()),
        };
        ctx.set_fonts(font_definitions(locale));
        ctx.data_mut(|data| data.insert_temp(egui::Id::new("application-locale"), locale));
        ctx.request_repaint();
    }
}

fn system_language(locale: Option<&str>) -> &'static str {
    let normalized = locale
        .unwrap_or("")
        .split('.')
        .next()
        .unwrap_or("")
        .replace('_', "-")
        .to_ascii_lowercase();
    let parts: Vec<_> = normalized.split('-').collect();
    match parts[0] {
        "fr" => "fr",
        "ja" => "ja",
        "es" => "es",
        "grc" => "grc",
        "eo" => "eo",
        // Do not silently substitute Simplified Chinese for a Traditional locale.
        "zh" if !parts
            .iter()
            .any(|part| matches!(*part, "hant" | "tw" | "hk" | "mo")) =>
        {
            "zh-CN"
        }
        _ => "en",
    }
}

fn font_definitions(locale: &str) -> egui::FontDefinitions {
    use egui::{FontData, FontFamily};
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "noto-sans".into(),
        FontData::from_static(include_bytes!("../assets/fonts/NotoSans-Regular.ttf")).into(),
    );
    // One collection, shared bytes: language-specific Han forms, full upstream repertoires.
    for (name, index) in [("noto-jp", 0), ("noto-sc", 2)] {
        let mut data =
            FontData::from_static(include_bytes!("../assets/fonts/NotoSansCJK-Regular.ttc"));
        data.index = index;
        fonts.font_data.insert(name.into(), data.into());
    }
    let cjk = if locale == "zh-CN" {
        ["noto-sc", "noto-jp"]
    } else {
        ["noto-jp", "noto-sc"]
    };
    // Consistent Latin and polytonic Greek; CJK follows the selected locale.
    let proportional = fonts.families.get_mut(&FontFamily::Proportional).unwrap();
    proportional.splice(0..0, ["noto-sans", cjk[0], cjk[1]].map(str::to_owned));
    let monospace = fonts.families.get_mut(&FontFamily::Monospace).unwrap();
    monospace.extend(["noto-sans", cjk[0], cjk[1]].map(str::to_owned));
    fonts
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
    fn regional_locales_match_only_supported_scripts() {
        for (input, expected) in [
            ("ja_JP.UTF-8", "ja"),
            ("es-MX", "es"),
            ("eo", "eo"),
            ("grc-GR", "grc"),
            ("zh_CN.UTF-8", "zh-CN"),
            ("ZH-hans-SG", "zh-CN"),
            ("zh", "zh-CN"),
            ("zh-Hant", "en"),
            ("zh-TW", "en"),
            ("zh-HK", "en"),
            ("zh-MO", "en"),
            ("el-GR", "en"),
            ("ne-NP", "en"),
            ("th-TH", "en"),
        ] {
            assert_eq!(system_language(Some(input)), expected, "{input}");
        }
    }

    #[test]
    fn language_names_follow_current_locale_and_keep_native_names() {
        let ctx = egui::Context::default();
        Language::Fr.apply(&ctx);
        assert_eq!(Language::En.name(&ctx), "English — Anglais");
        assert_eq!(Language::Fr.name(&ctx), "Français — Français");
        assert_eq!(Language::Es.name(&ctx), "Español — Espagnol");
        Language::En.apply(&ctx);
        assert_eq!(Language::Fr.name(&ctx), "Français — French");
        assert_eq!(Language::Ja.name(&ctx), "日本語 — Japanese");
        assert_eq!(Language::System.name(&ctx), tr(&ctx, "language.system"));
    }

    #[test]
    fn embedded_fonts_cover_catalogs_endonyms_and_mixed_text() {
        let catalogs = [
            (Language::En, include_str!("../locales/en.json")),
            (Language::Fr, include_str!("../locales/fr.json")),
            (Language::Ja, include_str!("../locales/ja.json")),
            (Language::ZhCn, include_str!("../locales/zh-CN.json")),
            (Language::Es, include_str!("../locales/es.json")),
            (Language::Grc, include_str!("../locales/grc.json")),
            (Language::Eo, include_str!("../locales/eo.json")),
        ];
        for (language, json) in catalogs {
            let ctx = egui::Context::default();
            language.apply(&ctx);
            let catalog: BTreeMap<String, String> = serde_json::from_str(json).unwrap();
            let mut text = catalog.values().cloned().collect::<Vec<_>>().join(" ");
            for choice in Language::ALL {
                text.push_str(&choice.name(&ctx));
            }
            text.push_str(" 日本語 中文 龍 繁體 Ἑλληνικὴ ἀρχαία ĈĜĤĴŜŬ ĉĝĥĵŝŭ Preview Content ratio 1920×1080");
            let characters: BTreeSet<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                ctx.fonts(|fonts| {
                    for font in [
                        egui::FontId::proportional(14.0),
                        egui::FontId::monospace(14.0),
                    ] {
                        for c in &characters {
                            assert!(
                                fonts.has_glyph(&font, *c),
                                "{language:?}: missing {c:?} in {font:?}"
                            );
                        }
                        let galley = fonts.layout_no_wrap(text.clone(), font, egui::Color32::WHITE);
                        assert!(galley.size().x.is_finite() && galley.size().x > 0.0);
                    }
                });
            });
        }
        let jp = font_definitions("ja");
        let cn = font_definitions("zh-CN");
        assert_eq!(jp.families[&egui::FontFamily::Proportional][1], "noto-jp");
        assert_eq!(cn.families[&egui::FontFamily::Proportional][1], "noto-sc");
        assert_eq!(jp.font_data["noto-jp"].index, 0);
        assert_eq!(cn.font_data["noto-sc"].index, 2);
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
        let catalogs = [
            ("en", include_str!("../locales/en.json")),
            ("fr", include_str!("../locales/fr.json")),
            ("ja", include_str!("../locales/ja.json")),
            ("zh-CN", include_str!("../locales/zh-CN.json")),
            ("es", include_str!("../locales/es.json")),
            ("grc", include_str!("../locales/grc.json")),
            ("eo", include_str!("../locales/eo.json")),
        ];
        let placeholders = |text: &str| -> BTreeSet<String> {
            text.split("%{")
                .skip(1)
                .map(|part| part.split('}').next().unwrap().to_owned())
                .collect()
        };
        for (locale, json) in catalogs {
            let translated: BTreeMap<String, String> = serde_json::from_str(json).unwrap();
            assert_eq!(
                english.keys().collect::<Vec<_>>(),
                translated.keys().collect::<Vec<_>>(),
                "{locale}"
            );
            for (key, text) in &english {
                assert!(!translated[key].trim().is_empty(), "empty {locale}: {key}");
                assert_eq!(
                    placeholders(text),
                    placeholders(&translated[key]),
                    "{locale}: {key}"
                );
                assert_eq!(
                    rust_i18n::t!(key.as_str(), locale = locale).as_ref(),
                    translated[key],
                    "{locale}: {key}"
                );
            }
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
